use super::header::SuiHeader;
use crate::spec::hash::SuiHash;
use crate::spec::transaction::SuiBlobTransaction;
use crate::spec::transaction::Transaction;
use crate::validity_condition::SuiValidityCond;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sov_rollup_interface::da::{DaProof, RelevantBlobs, RelevantProofs};
#[cfg(feature = "native")]
use sov_rollup_interface::services::da::SlotData;
use std::fmt::Debug;
use sui_sdk::types::messages_checkpoint::CheckpointDigest;

pub type BlockHeight = u64;
pub type BlockHash = CheckpointDigest;

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub struct BlockIdentifier {
    pub index: BlockHeight,
    pub hash: BlockHash,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub block_identifier: BlockIdentifier,
    pub parent_block_identifier: BlockIdentifier,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

// #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
// pub struct SuiBlock {
//     pub block: Block,
//     #[serde(default, skip_serializing_if = "Vec::is_empty")]
//     transactions: Vec<SuiBlobTransaction>, // 交易列表
// }

impl Default for SuiHeader {
    fn default() -> Self {
        SuiHeader::new(SuiHeader {
            prev_hash: SuiHash::try_from(0u64).unwrap(),
            hash: SuiHash::try_from(1u64).unwrap(),
            height: 0u64,
        })
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Clone)]
pub struct SuiBlock {
    pub header: SuiHeader,
    pub validity_cond: SuiValidityCond,
    /// Rollup's batch namespace.
    pub batch_blobs: Vec<SuiBlobTransaction>,
    /// Rollup's proof namespace.
    pub proof_blobs: Vec<SuiBlobTransaction>,
}

#[cfg(feature = "native")]
impl SlotData for SuiBlock {
    type BlockHeader = SuiHeader;
    type Cond = SuiValidityCond;

    fn hash(&self) -> [u8; 32] {
        self.header.hash.0
    }

    fn header(&self) -> &Self::BlockHeader {
        &self.header
    }

    fn validity_condition(&self) -> SuiValidityCond {
        self.validity_cond
    }
}

impl SuiBlock {
    pub fn as_relevant_blobs(&self) -> RelevantBlobs<SuiBlobTransaction> {
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

pub fn u64_to_bytes(value: u64) -> [u8; 32] {
    let value = value.to_be_bytes();
    let mut result = [0u8; 32];
    result[..value.len()].copy_from_slice(&value);
    result
}
