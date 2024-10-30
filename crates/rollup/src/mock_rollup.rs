#![deny(missing_docs)]
//! StarterRollup provides a minimal self-contained rollup implementation

use anyhow::Error;
use async_trait::async_trait;
use sov_attester_incentives::BondingProofServiceImpl;
use sov_db::ledger_db::LedgerDb;
use sov_db::storage_manager::NativeStorageManager;
use sov_mock_da::storable::service::StorableMockDaService;
use sov_mock_da::MockDaSpec;
use sov_mock_zkvm::{MockCodeCommitment, MockZkVerifier, MockZkvm};
use sov_modules_api::default_spec::DefaultSpec;
use sov_modules_api::higher_kinded_types::Generic;
use sov_modules_api::DaSyncState;
use sov_modules_api::SyncStatus;
use sov_modules_api::{CryptoSpec, OperatingMode, SovApiProofSerializer, Spec, Zkvm};
use sov_modules_rollup_blueprint::pluggable_traits::PluggableSpec;
use sov_modules_rollup_blueprint::{FullNodeBlueprint, RollupBlueprint};
use sov_modules_stf_blueprint::Runtime as RuntimeTrait;
use sov_modules_stf_blueprint::{RuntimeEndpoints, StfBlueprint};
use sov_risc0_adapter::host::Risc0Host;
use sov_risc0_adapter::Risc0Verifier;
use sov_rollup_interface::execution_mode::{ExecutionMode, Native, Zk};
use sov_rollup_interface::node::da::DaServiceWithRetries;
use sov_rollup_interface::zk::aggregated_proof::CodeCommitment;
use sov_sequencer::SequencerDb;
use sov_state::Storage;
use sov_state::{DefaultStorageSpec, ProverStorage, ZkStorage};
use sov_stf_runner::processes::{ParallelProverService, ProverService, RollupProverConfig};
use sov_stf_runner::RollupConfig;
use std::sync::Arc;
use stf_starter::Runtime;
use tokio::sync::watch::{self};

/// Rollup with [`MockDaService`].
#[derive(Default)]
pub struct MockRollup<M> {
    phantom: std::marker::PhantomData<M>,
}

/// This is the place, where all the rollup components come together, and
/// they can be easily swapped with alternative implementations as needed.

impl<M: ExecutionMode> RollupBlueprint<M> for MockRollup<M>
where
    DefaultSpec<MockDaSpec, Risc0Verifier, MockZkVerifier, M>: PluggableSpec,
{
    type Spec = DefaultSpec<MockDaSpec, Risc0Verifier, MockZkVerifier, M>;
    type Runtime = Runtime<Self::Spec>;
}

#[async_trait]
impl FullNodeBlueprint<Native> for MockRollup<Native> {
    type DaService = DaServiceWithRetries<StorableMockDaService>;
    /// Inner Zkvm representing the rollup circuit
    type InnerZkvmHost = Risc0Host<'static>;
    /// Outer Zkvm representing the circuit verifier for recursion
    type OuterZkvmHost = MockZkvm;
    /// Manager for the native storage lifecycle.
    type StorageManager = NativeStorageManager<
        MockDaSpec,
        ProverStorage<DefaultStorageSpec<<<Self::Spec as Spec>::CryptoSpec as CryptoSpec>::Hasher>>,
    >;
    /// Prover service.
    type ProverService = ParallelProverService<
        <Self::Spec as Spec>::Address,
        <<Self::Spec as Spec>::Storage as Storage>::Root,
        <<Self::Spec as Spec>::Storage as Storage>::Witness,
        Self::DaService,
        Self::InnerZkvmHost,
        Self::OuterZkvmHost,
        StfBlueprint<
            <Self::Spec as Generic>::With<Zk>,
            <MockRollup<Zk> as RollupBlueprint<Zk>>::Runtime,
        >,
    >;

    type ProofSerializer = SovApiProofSerializer<Self::Spec>;

    type BondingProofService = BondingProofServiceImpl<Self::Spec, Self::Runtime>;

    fn create_bonding_proof_service(
        &self,
        attester_address: <Self::Spec as Spec>::Address,
        storage: tokio::sync::watch::Receiver<<Self::Spec as Spec>::Storage>,
    ) -> Self::BondingProofService {
        let runtime = Runtime::<Self::Spec>::default();
        BondingProofServiceImpl::new(
            attester_address,
            runtime.attester_incentives.clone(),
            storage,
            runtime,
        )
    }

    fn get_operating_mode(
        genesis: &<Self::Runtime as RuntimeTrait<Self::Spec>>::GenesisConfig,
    ) -> OperatingMode {
        genesis.chain_state.operating_mode
    }

    fn create_outer_code_commitment(
        &self,
    ) -> <<Self::ProverService as ProverService>::Verifier as Zkvm>::CodeCommitment {
        MockCodeCommitment::default()
    }

    async fn create_endpoints(
        &self,
        storage: watch::Receiver<<Self::Spec as Spec>::Storage>,
        sync_status_receiver: watch::Receiver<SyncStatus>,
        ledger_db: &LedgerDb,
        sequencer_db: &SequencerDb,
        da_service: &Self::DaService,
        da_sync_state: Arc<DaSyncState>,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> Result<RuntimeEndpoints, Error> {
        sov_modules_rollup_blueprint::register_endpoints::<Self, Native>(
            storage.clone(),
            sync_status_receiver,
            ledger_db,
            sequencer_db,
            da_service,
            da_sync_state,
            &rollup_config.sequencer,
            &rollup_config.runner,
        )
        .await
    }

    async fn create_da_service(
        &self,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> Self::DaService {
        DaServiceWithRetries::new_fast(
            StorableMockDaService::from_config(rollup_config.da.clone()).await,
        )
    }

    async fn create_prover_service(
        &self,
        prover_config: RollupProverConfig,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
        _da_service: &Self::DaService,
    ) -> Self::ProverService {
        let inner_vm = if let RollupProverConfig::Skip = prover_config {
            Risc0Host::new(b"")
        } else {
            let elf = std::fs::read(risc0_starter::MOCK_DA_PATH)
                .unwrap_or_else(|e| {
                    panic!(
                        "Could not read guest elf file from `{}`. {}",
                        risc0_starter::MOCK_DA_PATH,
                        e
                    )
                })
                .leak();
            Risc0Host::new(elf)
        };
        let outer_vm = MockZkvm::new_non_blocking();
        let zk_stf = StfBlueprint::new();
        let zk_storage = ZkStorage::new();
        let da_verifier = Default::default();

        ParallelProverService::new_with_default_workers(
            inner_vm,
            outer_vm,
            zk_stf,
            da_verifier,
            prover_config,
            zk_storage,
            CodeCommitment::default(),
            rollup_config.proof_manager.prover_address,
        )
    }

    fn create_storage_manager(
        &self,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> Result<Self::StorageManager, Error> {
        NativeStorageManager::new(&rollup_config.storage.path)
    }
}

impl sov_modules_rollup_blueprint::WalletBlueprint<Native> for MockRollup<Native> {}
