use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::BlockHeaderTrait;

use super::hash::CyferioHash;

const KATE_START_TIME: i64 = 1686066440;
const KATE_SECONDS_PER_BLOCK: i64 = 20;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CyferioHeader {
    pub number: u32,
    pub parent_hash: CyferioHash,
    pub state_root: CyferioHash,
    pub extrinsics_root: CyferioHash,
    pub digest: Vec<u8>,
    pub timestamp: u64,
}

impl CyferioHeader {
    fn new(header: CyferioHeader) -> Self {
        Self {
            number: header.number,
            parent_hash: header.parent_hash,
            state_root: header.state_root,
            extrinsics_root: header.extrinsics_root,
            digest: header.digest,
            timestamp: header.timestamp,
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
                .saturating_mul(self.timestamp as i64)
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
            digest: Vec::new(),
            timestamp: 0,
            // 可以根据需要设置其他默认值
        }
    }
}
