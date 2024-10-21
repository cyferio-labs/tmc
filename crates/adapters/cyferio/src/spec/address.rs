use serde::{Deserialize, Serialize};
use sp_core::crypto::AccountId32;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Copy, Debug, PartialEq, Clone, Eq, Hash)]
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
        let account_id = AccountId32::from_str(s)?;
        Ok(Self(account_id))
    }
}

impl AsRef<[u8]> for CyferioAddress {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

