// TODO: Rename this file to change the name of this method from METHOD_NAME

#![no_main]

use sov_celestia_adapter::types::Namespace;
use sov_celestia_adapter::verifier::CelestiaSpec;
use sov_celestia_adapter::verifier::CelestiaVerifier;
use sov_mock_zkvm::{MockZkGuest, MockZkVerifier};
use sov_modules_api::default_spec::DefaultSpec;
use sov_modules_api::execution_mode::Zk;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_risc0_adapter::guest::Risc0Guest;
use sov_risc0_adapter::Risc0Verifier;
use sov_state::ZkStorage;
use stf_starter::runtime::Runtime;
use stf_starter::StfVerifier;

/// The namespace for the rollup on Celestia. Must be kept in sync with the "rollup/src/lib.rs"
const ROLLUP_BATCH_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-b");
const ROLLUP_PROOF_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-p");

risc0_zkvm::guest::entry!(main);

pub fn main() {
    let guest = Risc0Guest::new();
    let storage = ZkStorage::new();
    let stf: StfBlueprint<
        DefaultSpec<CelestiaSpec, Risc0Verifier, MockZkVerifier, Zk>,
        Runtime<_>,
    > = StfBlueprint::new();

    let stf_verifier = StfVerifier::<_, _, _, Risc0Guest, MockZkGuest>::new(
        stf,
        CelestiaVerifier {
            rollup_batch_namespace: ROLLUP_BATCH_NAMESPACE,
            rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
        },
    );
    stf_verifier
        .run_block(guest, storage)
        .expect("Prover must be honest");
}
