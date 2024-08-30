use core::fmt::{Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use fastcrypto::ed25519::Ed25519PublicKey;
use std::convert::TryFrom;

#[derive(Serialize, Deserialize,Copy,Debug, PartialEq, Clone, Eq, Hash)]
pub struct SuiAddress([u8; 32]);


impl sov_rollup_interface::BasicAddress for SuiAddress {}

impl Display for SuiAddress {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let hash = H256(self.0);
        write!(f, "{hash}")
    }
}

impl AsRef<[u8]> for SuiAddress {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<[u8; 32]> for SuiAddress {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl FromStr for SuiAddress {
    type Err = <H256 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let h_256 = H256::from_str(s)?;

        Ok(Self(h_256.to_fixed_bytes()))
    }
}

impl<'a> TryFrom<&'a [u8]> for SuiAddress {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self(<[u8; 32]>::try_from(value)?))
    }
}


impl From<&Ed25519PublicKey> for SuiAddress {
    fn from(pk: &Ed25519PublicKey) -> Self {
        let pk_bytes: &[u8] = pk.as_ref();
        let array: [u8; 32] = pk_bytes.try_into().expect("Public key must be 32 bytes");
        Self(array)
    }
}