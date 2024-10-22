use super::hash::CyferioHash;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::BlockHeaderTrait;
use subxt::config::substrate::Digest;
use subxt_core::config::substrate::SubstrateHeader;
use subxt_core::config::substrate::BlakeTwo256;

const KATE_START_TIME: i64 = 1686066440;
const KATE_SECONDS_PER_BLOCK: i64 = 20;

#[subxt::subxt(runtime_metadata_path = "./src/metadata.scale")]
pub mod substrate {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CyferioHeader {
    pub number: u32,
    pub parent_hash: CyferioHash,
    pub state_root: CyferioHash,
    pub extrinsics_root: CyferioHash,
    pub digest: Digest,
}

impl CyferioHeader {
    pub fn new(
        number: u32,
        parent_hash: CyferioHash,
        state_root: CyferioHash,
        extrinsics_root: CyferioHash,
        digest: Digest,
    ) -> Self {
        Self {
            number,
            parent_hash,
            state_root,
            extrinsics_root,
            digest,
        }
    }
}

impl BlockHeaderTrait for CyferioHeader {
    type Hash = CyferioHash;

    fn prev_hash(&self) -> Self::Hash {
        self.parent_hash
    }

    fn hash(&self) -> Self::Hash {
        self.state_root.clone()
    }

    fn height(&self) -> u64 {
        self.number as u64
    }

    fn time(&self) -> sov_rollup_interface::da::Time {
        sov_rollup_interface::da::Time::from_secs(
            KATE_SECONDS_PER_BLOCK
                .saturating_mul(self.number as i64)
                .saturating_add(KATE_START_TIME),
        )
    }
}

impl Default for CyferioHeader {
    fn default() -> Self {
        Self {
            number: 0,
            parent_hash: CyferioHash::default(),
            state_root: CyferioHash::default(),
            extrinsics_root: CyferioHash::default(),
            digest: Digest::default(),
        }
    }
}


#[cfg(feature = "native")]
impl From<&SubstrateHeader<u32, BlakeTwo256>> for CyferioHeader {
    fn from(header: &SubstrateHeader<u32, BlakeTwo256>) -> Self {
        CyferioHeader::new(
            header.number,
            CyferioHash::from(header.parent_hash.0),
            CyferioHash::from(header.state_root.0),
            CyferioHash::from(header.extrinsics_root.0),
            header.digest.clone(),
        )
    }
}