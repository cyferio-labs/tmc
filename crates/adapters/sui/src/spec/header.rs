use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::BlockHeaderTrait;

const KATE_START_TIME: i64 = 1686066440;
const KATE_SECONDS_PER_BLOCK: i64 = 20;

use super::hash::SuiHash;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SuiHeader {
    pub prev_hash: SuiHash,
    pub hash: SuiHash,
    pub height: u64,
}

impl SuiHeader {
    pub fn new(header: SuiHeader) -> Self {
        Self {
            prev_hash: header.prev_hash,
            hash: header.hash,
            height: header.height,
        }
    }
}

impl BlockHeaderTrait for SuiHeader {
    type Hash = SuiHash;

    fn prev_hash(&self) -> Self::Hash {
        SuiHash::try_from(self.prev_hash.clone()).expect("Corrupted `prev_hash` in database")
    }

    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }

    fn height(&self) -> u64 {
        self.height
    }

    fn time(&self) -> sov_rollup_interface::da::Time {
        sov_rollup_interface::da::Time::from_secs(
            KATE_SECONDS_PER_BLOCK
                .saturating_mul(self.height as i64)
                .saturating_add(KATE_START_TIME),
        )
    }
}
