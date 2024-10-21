use super::hash::CyferioHash;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::BlockHeaderTrait;
use subxt::blocks::Block;
use subxt::config::substrate::Digest;
use codec::Decode;
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
    pub timestamp: u64,
}

impl CyferioHeader {
    pub fn new(
        number: u32,
        parent_hash: CyferioHash,
        state_root: CyferioHash,
        extrinsics_root: CyferioHash,
        digest: Digest,
        timestamp: u64,
    ) -> Self {
        Self {
            number,
            parent_hash,
            state_root,
            extrinsics_root,
            digest,
            timestamp,
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
            digest: Digest::default(),
            timestamp: 0,
            // 可以根据需要设置其他默认值
        }
    }
}

// impl TryFrom<Block<substrate::Header, substrate::UncheckedExtrinsic>> for CyferioHeader {
//     type Error = anyhow::Error;

//     fn try_from(block: Block<substrate::Header, substrate::UncheckedExtrinsic>) -> Result<Self, Self::Error> {
//         let header = block.header();
//         let parent_hash = CyferioHash::from(header.parent_hash.0);
//         let state_root = CyferioHash::from(header.state_root.0);
//         let extrinsics_root = CyferioHash::from(header.extrinsics_root.0);
//         let number = header.number;
//         let digest = header.digest.clone();

//         // Extract timestamp from extrinsics
//         let mut timestamp: u64 = 0;
//         for ext in block.extrinsics() {
//             if let Ok(call) = substrate::Call::decode(&mut ext.as_ref()) {
//                 if let substrate::Call::Timestamp(substrate::timestamp::Call::set { now }) = call {
//                     timestamp = now;
//                     break;
//                 }
//             }
//         }

//         Ok(CyferioHeader::new(
//             number,
//             parent_hash,
//             state_root,
//             extrinsics_root,
//             digest,
//             timestamp,
//         ))
//     }
// }
