use bytes::Bytes;
use serde::{Deserialize, Serialize};

use sov_rollup_interface::da::{BlobReaderTrait, CountedBufReader};

use super::address::SuiAddress;
use super::hash::SuiHash;

use serde_json::Value;
use sui_sdk::types::base_types::TransactionDigest;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SuiEnv {
    MainNet,
    DevNet,
    TestNet,
    LocalNet,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkIdentifier {
    pub blockchain: String,
    pub network: SuiEnv,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionIdentifier {
    pub hash: TransactionDigest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelatedTransaction {
    network_identifier: NetworkIdentifier,
    transaction_identifier: TransactionIdentifier,
    direction: Direction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub transaction_identifier: TransactionIdentifier,
    // pub operations: Operations,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_transactions: Vec<RelatedTransaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SuiBlobTransaction {
    blob: CountedBufReader<Bytes>,
    hash: SuiHash,
    sender: SuiAddress,
}

impl BlobReaderTrait for SuiBlobTransaction {
    type Address = SuiAddress;
    type BlobHash = SuiHash;

    fn sender(&self) -> SuiAddress {
        self.sender.clone()
    }

    fn hash(&self) -> SuiHash {
        self.hash
    }

    fn verified_data(&self) -> &[u8] {
        self.blob.accumulator()
    }

    fn total_len(&self) -> usize {
        self.blob.total_len()
    }

    #[cfg(feature = "native")]
    fn advance(&mut self, num_bytes: usize) -> &[u8] {
        self.blob.advance(num_bytes);
        self.verified_data()
    }
}
