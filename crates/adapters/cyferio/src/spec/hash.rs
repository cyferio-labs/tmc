use borsh::{BorshDeserialize, BorshSerialize};
use sov_rollup_interface::da::BlockHashTrait;
use sp_core::H256;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    BorshDeserialize,
    BorshSerialize,
    derive_more::From,
    derive_more::Into,
)]
pub struct CyferioHash(pub H256);

impl BlockHashTrait for CyferioHash {}

impl AsRef<[u8]> for CyferioHash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl std::fmt::Debug for CyferioHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl std::fmt::Display for CyferioHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}