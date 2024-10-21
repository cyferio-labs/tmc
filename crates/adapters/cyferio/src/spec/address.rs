use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use sp_core::crypto::AccountId32;


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct CyferioAddress(AccountId32);

impl sov_rollup_interface::BasicAddress for CyferioAddress {}

impl From<AccountId32> for CyferioAddress {
    fn from(account_id: AccountId32) -> Self {
        Self(account_id)
    }
}

impl FromStr for CyferioAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let account_id =
            AccountId32::from_str(s).map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;
        Ok(CyferioAddress(account_id))
    }
}

impl AsRef<[u8]> for CyferioAddress {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl fmt::Display for CyferioAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&[u8]> for CyferioAddress {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid address length"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(AccountId32::new(arr)))
    }
}
