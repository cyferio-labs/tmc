use crate::spec::SuiDaLayerSpec;
use sov_rollup_interface::da::{DaSpec, DaVerifier, RelevantBlobs, RelevantProofs};

#[derive(Clone, Default)]
/// DaVerifier used in tests.
pub struct SuiDaVerifier {}

impl DaVerifier for SuiDaVerifier {
    type Spec = SuiDaLayerSpec;

    type Error = anyhow::Error;

    fn new(_params: <Self::Spec as DaSpec>::ChainParams) -> Self {
        Self {}
    }

    fn verify_relevant_tx_list(
        &self,
        _block_header: &<Self::Spec as DaSpec>::BlockHeader,
        _relevant_blobs: &RelevantBlobs<<Self::Spec as DaSpec>::BlobTransaction>,
        _relevant_proofs: RelevantProofs<
            <Self::Spec as DaSpec>::InclusionMultiProof,
            <Self::Spec as DaSpec>::CompletenessProof,
        >,
    ) -> Result<<Self::Spec as DaSpec>::ValidityCondition, Self::Error> {
        Ok(Default::default())
    }
}
