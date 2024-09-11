use borsh::{BorshDeserialize, BorshSerialize};
use sov_rollup_interface::da::DaSpec;
pub mod address;
pub mod block;
pub mod hash;
pub mod header;
pub mod transaction;

// #[cfg(feature = "native")]
use crate::validity_condition::{SuiValidityCond, SuiValidityCondChecker};

// use crate::validity_condition::SuiValidityCond;

#[derive(
    Default,
    serde::Serialize,
    serde::Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
)]
pub struct SuiDaLayerSpec;

impl DaSpec for SuiDaLayerSpec {
    type SlotHash = hash::SuiHash;

    type BlockHeader = header::SuiHeader;

    type BlobTransaction = transaction::SuiBlobTransaction;

    type Address = address::SuiAddress;

    type ValidityCondition = SuiValidityCond;

    #[cfg(feature = "native")]
    type Checker = SuiValidityCondChecker<SuiValidityCond>;

    type InclusionMultiProof = [u8; 32];

    type CompletenessProof = ();

    type ChainParams = ();
}
