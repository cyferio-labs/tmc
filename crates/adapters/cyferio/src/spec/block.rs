use super::header::CyferioHeader;
use super::transaction::CyferioBlobTransaction;
use crate::validity_condition::CyferioValidityCond;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::{DaProof, RelevantBlobs, RelevantProofs};

#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Clone)]
pub struct CyferioBlock {
    pub header: CyferioHeader,
    pub validity_cond: CyferioValidityCond,
    pub batch_blobs: Vec<CyferioBlobTransaction>,
    pub proof_blobs: Vec<CyferioBlobTransaction>,
}

impl CyferioBlock {
    pub fn as_relevant_blobs(&self) -> RelevantBlobs<CyferioBlobTransaction> {
        RelevantBlobs {
            proof_blobs: self.proof_blobs.clone(),
            batch_blobs: self.batch_blobs.clone(),
        }
    }

    pub fn get_relevant_proofs(&self) -> RelevantProofs<[u8; 32], ()> {
        RelevantProofs {
            batch: DaProof {
                inclusion_proof: Default::default(),
                completeness_proof: Default::default(),
            },
            proof: DaProof {
                inclusion_proof: Default::default(),
                completeness_proof: Default::default(),
            },
        }
    }
}