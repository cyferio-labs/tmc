use anyhow::{bail, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sov_modules_api::GenesisState;

// FHE deps
use bincode;
use tfhe::{prelude::*, CompressedPublicKey};

use crate::token::Token;
use crate::utils::TokenHolderRef;
use crate::{Bank, TokenId};

/// Initial configuration for sov-bank module.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(
    feature = "native",
    derive(schemars::JsonSchema),
    schemars(bound = "S: ::sov_modules_api::Spec", rename = "BankConfig")
)]
#[serde(bound = "S::Address: Serialize + DeserializeOwned")]
pub struct BankConfig<S: sov_modules_api::Spec> {
    /// A list of configurations for any other tokens to create at genesis
    pub tokens: Vec<TokenConfig<S>>,
    /// fhe public key
    pub fhe_public_key: Vec<u8>,
    /// fhe server key
    pub fhe_server_key: Vec<u8>,
}

/// Type for deserialized FheUint64
pub type EncryptedAmount = Vec<u8>;

/// [`TokenConfig`] specifies a configuration used when generating a token for the bank
/// module.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(
    feature = "native",
    derive(schemars::JsonSchema),
    schemars(bound = "S: ::sov_modules_api::Spec", rename = "TokenConfig")
)]
#[serde(bound = "S::Address: Serialize + DeserializeOwned")]
pub struct TokenConfig<S: sov_modules_api::Spec> {
    /// The name of the token.
    pub token_name: String,
    /// Predetermined ID of the token. Allowed only for genesis tokens.
    pub token_id: TokenId,
    /// A vector of tuples containing the initial addresses and balances (as EncryptedAmount)
    pub address_and_balances: Vec<(S::Address, EncryptedAmount)>,
    /// The addresses that are authorized to mint the token.
    pub authorized_minters: Vec<S::Address>,
}

impl<S: sov_modules_api::Spec> core::fmt::Display for TokenConfig<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let address_and_balances = self
            .address_and_balances
            .iter()
            .map(|(address, balance)| format!("({}, {:?})", address, balance))
            .collect::<Vec<String>>()
            .join(", ");

        let authorized_minters = self
            .authorized_minters
            .iter()
            .map(|minter| minter.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        write!(
            f,
            "TokenConfig {{ token_name: {}, token_id: {}, address_and_balances: [{}], authorized_minters: [{}] }}",
            self.token_name,
            self.token_id,
            address_and_balances,
            authorized_minters,
        )
    }
}

impl<S: sov_modules_api::Spec> Bank<S> {
    /// Init an instance of the bank module from the configuration `config`.
    /// For each token in the `config`, calls the [`Token::create`] function to create
    /// the token. Upon success, updates the token set if the token ID doesn't already exist.
    pub(crate) fn init_module(
        &self,
        config: &<Self as sov_modules_api::Module>::Config,
        state: &mut impl GenesisState<S>,
    ) -> Result<()> {
        // Fetch the fhe keys from genesis config and set them into state
        self.fhe_public_key.set(&config.fhe_public_key, state)?;
        self.fhe_server_key.set(&config.fhe_server_key, state)?;
        tracing::debug!("raw FHE keys have been set");
        let fhe_public_key = bincode::deserialize::<CompressedPublicKey>(&config.fhe_public_key)
            .unwrap()
            .decompress();
        tracing::debug!("FHE keys have been deserialized");

        // Disable token creation in genesis phase for now
        // let parent_prefix = self.tokens.prefix();
        // for token_config in config.tokens.iter() {
        //     let token_id = &token_config.token_id;
        //     tracing::debug!(
        //         %token_config,
        //         token_id = %token_id,
        //         "Genesis of the token");

        //     let authorized_minters = token_config
        //         .authorized_minters
        //         .iter()
        //         .map(|minter| TokenHolderRef::<'_, S>::from(&minter))
        //         .collect::<Vec<_>>();

        //     let address_and_balances = token_config
        //         .address_and_balances
        //         .iter()
        //         .map(|(address, balance)| {
        //             (TokenHolderRef::<'_, S>::from(&address), balance.clone())
        //         })
        //         .collect::<Vec<_>>();

        //     let token = Token::<S>::create_with_token_id(
        //         &token_config.token_name,
        //         &address_and_balances,
        //         &authorized_minters,
        //         token_id,
        //         parent_prefix,
        //         &fhe_public_key,
        //         state,
        //     )?;

        //     if self.tokens.get(token_id, state)?.is_some() {
        //         bail!("token ID {} already exists", token_config.token_id);
        //     }

        //     self.tokens.set(token_id, &token, state)?;
        //     tracing::debug!(
        //         token_name = %token.name,
        //         token_id = %token_id,
        //         "Token has been created"
        //     );
        // }
        Ok(())
    }
}
