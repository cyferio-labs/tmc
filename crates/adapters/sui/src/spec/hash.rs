use borsh::{BorshDeserialize, BorshSerialize};
use sov_rollup_interface::common::HexHash;
use sov_rollup_interface::da::BlockHashTrait;
use std::fmt::{Debug, Formatter};
use crate::spec::block::u64_to_bytes;

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
pub struct SuiHash(pub [u8; 32]);

impl BlockHashTrait for SuiHash {}

impl Debug for SuiHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", HexHash::new(self.0))
    }
}

impl core::fmt::Display for SuiHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", HexHash::new(self.0))
    }
}

impl AsRef<[u8]> for SuiHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for SuiHash {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let hash: [u8; 32] = value.try_into().map_err(|e: Vec<u8>| {
            anyhow::anyhow!("Vec<u8> should have length 32: but it has {}", e.len())
        })?;
        Ok(SuiHash(hash))
    }
}

impl TryFrom<u64> for SuiHash {
    type Error = anyhow::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let hash = u64_to_bytes(value);
        Ok(SuiHash(hash))
    }
}
