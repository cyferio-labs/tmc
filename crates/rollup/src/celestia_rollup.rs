#![deny(missing_docs)]
//! StarterRollup provides a minimal self-contained rollup implementation

use async_trait::async_trait;
use backon::ExponentialBuilder;
use sov_attester_incentives::BondingProofServiceImpl;
use sov_celestia_adapter::types::Namespace;
use sov_celestia_adapter::verifier::{CelestiaSpec, CelestiaVerifier, RollupParams};
use sov_celestia_adapter::CelestiaService;
use sov_db::ledger_db::LedgerDb;
use sov_db::storage_manager::NativeStorageManager;
use sov_mock_zkvm::{MockCodeCommitment, MockZkVerifier, MockZkvm};
use sov_modules_api::default_spec::DefaultSpec;
use sov_modules_api::{CryptoSpec, OperatingMode, SovApiProofSerializer, Spec};
use sov_modules_rollup_blueprint::pluggable_traits::PluggableSpec;
use sov_modules_rollup_blueprint::{FullNodeBlueprint, RollupBlueprint};
use sov_modules_stf_blueprint::Runtime as RuntimeTrait;
use sov_modules_stf_blueprint::{RuntimeEndpoints, StfBlueprint};
use sov_risc0_adapter::host::Risc0Host;
use sov_risc0_adapter::Risc0Verifier;
use sov_rollup_interface::execution_mode::{ExecutionMode, Native, Zk};
use sov_rollup_interface::node::da::DaServiceWithRetries;
use sov_rollup_interface::node::DaSyncState;
use sov_rollup_interface::node::SyncStatus;
use sov_rollup_interface::zk::aggregated_proof::CodeCommitment;
use sov_rollup_interface::zk::Zkvm;
use sov_sequencer::SequencerDb;
use sov_state::Storage;
use sov_state::{DefaultStorageSpec, ProverStorage, ZkStorage};
use sov_stf_runner::processes::{ParallelProverService, ProverService, RollupProverConfig};
use sov_stf_runner::RollupConfig;
use std::sync::Arc;
use stf_starter::Runtime;
use tokio::sync::watch;

/// The rollup stores its data in the namespace "sov-test-b" on Celestia.
/// You can change this constant to point your rollup at a different namespace.
const ROLLUP_BATCH_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-b");

/// The rollup stores the zk proofs in the namespace "sov-test-p" on Celestia.
const ROLLUP_PROOF_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-p");

/// Rollup with [`CelestiaDaService`].
#[derive(Default)]
pub struct CelestiaRollup<M> {
    phantom: std::marker::PhantomData<M>,
}

/// This is the place, where all the rollup components come together, and
/// they can be easily swapped with alternative implementations as needed.
impl<M: ExecutionMode> RollupBlueprint<M> for CelestiaRollup<M>
where
    DefaultSpec<CelestiaSpec, Risc0Verifier, MockZkVerifier, M>: PluggableSpec,
{
    type Spec = DefaultSpec<CelestiaSpec, Risc0Verifier, MockZkVerifier, M>;
    type Runtime = Runtime<Self::Spec>;
}

#[async_trait]
impl FullNodeBlueprint<Native> for CelestiaRollup<Native> {
    type DaService = DaServiceWithRetries<CelestiaService>;
    /// Inner Zkvm representing the rollup circuit
    type InnerZkvmHost = Risc0Host<'static>;
    /// Outer Zkvm representing the circuit verifier for recursion
    type OuterZkvmHost = MockZkvm;

    type StorageManager = NativeStorageManager<
        CelestiaSpec,
        ProverStorage<DefaultStorageSpec<<<Self::Spec as Spec>::CryptoSpec as CryptoSpec>::Hasher>>,
    >;

    type ProverService = ParallelProverService<
        <Self::Spec as Spec>::Address,
        <<Self::Spec as Spec>::Storage as Storage>::Root,
        <<Self::Spec as Spec>::Storage as Storage>::Witness,
        Self::DaService,
        Self::InnerZkvmHost,
        Self::OuterZkvmHost,
        StfBlueprint<
            <CelestiaRollup<Zk> as RollupBlueprint<Zk>>::Spec,
            <CelestiaRollup<Zk> as RollupBlueprint<Zk>>::Runtime,
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
        sync_status_receiver: tokio::sync::watch::Receiver<SyncStatus>,
        ledger_db: &LedgerDb,
        sequencer_db: &SequencerDb,
        da_service: &Self::DaService,
        da_sync_state: Arc<DaSyncState>,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> anyhow::Result<RuntimeEndpoints> {
        sov_modules_rollup_blueprint::register_endpoints::<Self, _>(
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
        DaServiceWithRetries::with_exponential_backoff(
            CelestiaService::new(
                rollup_config.da.clone(),
                RollupParams {
                    rollup_batch_namespace: ROLLUP_BATCH_NAMESPACE,
                    rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
                },
            )
            .await,
            // NOTE: Current exponential backoff policy defaults:
            // jitter: false, factor: 2, min_delay: 1s, max_delay: 60s, max_times: 3,
            ExponentialBuilder::default(),
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
            let elf = std::fs::read(risc0_starter::ROLLUP_PATH)
                .unwrap_or_else(|e| {
                    panic!(
                        "Could not read guest elf file from `{}`. {}",
                        risc0_starter::ROLLUP_PATH,
                        e
                    )
                })
                .leak();
            Risc0Host::new(elf)
        };

        let outer_vm = MockZkvm::new_non_blocking();
        let zk_stf = StfBlueprint::new();
        let zk_storage = ZkStorage::new();

        let da_verifier = CelestiaVerifier {
            rollup_batch_namespace: ROLLUP_BATCH_NAMESPACE,
            rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
        };

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
    ) -> anyhow::Result<Self::StorageManager> {
        NativeStorageManager::new(&rollup_config.storage.path)
    }
}

impl sov_modules_rollup_blueprint::WalletBlueprint<Native> for CelestiaRollup<Native> {}
