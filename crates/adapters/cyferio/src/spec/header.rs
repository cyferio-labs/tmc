use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::BlockHeaderTrait;
use sp_runtime::traits::Header as SubstrateHeader;

use super::hash::CyferioHash;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CyferioHeader {
    pub number: u32,
    pub parent_hash: CyferioHash,
    pub state_root: CyferioHash,
    pub extrinsics_root: CyferioHash,
    pub digest: Vec<u8>,
}

impl BlockHeaderTrait for CyferioHeader {
    type Hash = CyferioHash;

    fn prev_hash(&self) -> Self::Hash {
        self.parent_hash
    }

    fn hash(&self) -> Self::Hash {
        // Implement the hashing logic for the header
        // This is a placeholder implementation
        let mut hasher = sha2::Sha256::new();
        hasher.update(&self.number.to_le_bytes());
        hasher.update(self.parent_hash.as_ref());
        hasher.update(self.state_root.as_ref());
        hasher.update(self.extrinsics_root.as_ref());
        hasher.update(&self.digest);
        CyferioHash(hasher.finalize().into())
    }

    fn height(&self) -> u64 {
        self.number as u64
    }

    fn time(&self) -> sov_rollup_interface::da::Time {
        // Implement the time logic for the header
        // This is a placeholder implementation
        sov_rollup_interface::da::Time::from_secs(0)
    }
}

