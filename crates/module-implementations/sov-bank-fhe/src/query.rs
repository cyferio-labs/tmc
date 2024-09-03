//! Defines rpc queries exposed by the bank module, along with the relevant types
use jsonrpsee::core::RpcResult;
use sov_modules_api::macros::rpc_gen;
use sov_modules_api::prelude::UnwrapInfallible;
use sov_modules_api::ApiStateAccessor;

use crate::{get_token_id, Bank, EncryptedAmount, TokenId};

/// Structure returned by the `balance_of` rpc method.
#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
pub enum BalanceResponse {
    Plaintext(Option<u64>),
    Ciphertext(Option<EncryptedAmount>),
}

/// Structure returned by the `supply_of` rpc method.
#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
pub enum TotalSupplyResponse {
    Plaintext(Option<u64>),
    Ciphertext(Option<EncryptedAmount>),
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
pub struct FhePublicKeyResponse {
    pub public_key: Option<Vec<u8>>,
}

#[rpc_gen(client, server, namespace = "fheBank")]
impl<S: sov_modules_api::Spec> Bank<S> {
    #[rpc_method(name = "balanceOf")]
    /// Rpc method that returns the balance of the user at the address `user_address` for the token
    /// stored at the address `token_id`.
    pub fn balance_of(
        &self,
        version: Option<u64>,
        user_address: S::Address,
        token_id: TokenId,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<BalanceResponse> {
        let amount = if let Some(v) = version {
            self.get_balance_of(&user_address, token_id, &mut state.get_archival_at(v))
        } else {
            self.get_balance_of(&user_address, token_id, state)
        }
        .unwrap_infallible();
        Ok(BalanceResponse::Plaintext(amount))
    }

    #[rpc_method(name = "rawBalanceOf")]
    /// Rpc method that returns the raw balance of the user at the address `user_address` for the token
    /// stored at the address `token_id`.
    pub fn raw_balance_of(
        &self,
        version: Option<u64>,
        user_address: S::Address,
        token_id: TokenId,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<BalanceResponse> {
        let amount = if let Some(v) = version {
            self.get_raw_balance_of(&user_address, token_id, &mut state.get_archival_at(v))
        } else {
            self.get_raw_balance_of(&user_address, token_id, state)
        }
        .unwrap_infallible();
        Ok(BalanceResponse::Ciphertext(amount))
    }

    #[rpc_method(name = "supplyOf")]
    /// Rpc method that returns the supply of a token stored at the address `token_id`.
    pub fn supply_of(
        &self,
        version: Option<u64>,
        token_id: TokenId,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<TotalSupplyResponse> {
        let amount = if let Some(v) = version {
            self.get_total_supply_of(&token_id, &mut state.get_archival_at(v))
        } else {
            self.get_total_supply_of(&token_id, state)
        }
        .unwrap_infallible();
        Ok(TotalSupplyResponse::Plaintext(amount))
    }

    #[rpc_method(name = "rawSupplyOf")]
    /// Rpc method that returns the raw supply of a token stored at the address `token_id`.
    pub fn raw_supply_of(
        &self,
        version: Option<u64>,
        token_id: TokenId,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<TotalSupplyResponse> {
        let amount = if let Some(v) = version {
            self.get_raw_total_supply_of(&token_id, &mut state.get_archival_at(v))
        } else {
            self.get_raw_total_supply_of(&token_id, state)
        }
        .unwrap_infallible();
        Ok(TotalSupplyResponse::Ciphertext(amount))
    }

    #[rpc_method(name = "tokenId")]
    /// RPC method that returns the token ID for a given token name, sender, and salt.
    pub fn token_id(
        &self,
        token_name: String,
        sender: S::Address,
        salt: u64,
    ) -> RpcResult<TokenId> {
        Ok(get_token_id::<S>(&token_name, &sender, salt))
    }

    #[rpc_method(name = "FhePublicKey")]
    /// RPC method that returns the FHE public key of the bank.
    pub fn fhe_public_key(
        &self,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<FhePublicKeyResponse> {
        let public_key = self.get_fhe_public_key(state).unwrap();
        Ok(FhePublicKeyResponse { public_key })
    }
}
