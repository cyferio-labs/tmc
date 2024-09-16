#[cfg(feature = "native")]
use core::str::FromStr;
use std::collections::HashSet;
use std::default;
use std::fmt::Formatter;

use anyhow::bail;
use serde::{Deserialize, Serialize};
use sov_modules_api::impl_hash32_type;
use sov_modules_api::prelude::*;
use sov_state::namespaces::User;
use sov_state::Prefix;
use thiserror::Error;

// FHE deps
use bincode;
use tfhe::{prelude::*, set_server_key, CompressedFheUint64, FheUint64, PublicKey, CompressedServerKey};

/// Type alias to store an amount of token.
// pub type Amount = u64;
pub type EncryptedAmount = Vec<u8>; // as CompressedFheUint64 ciphertext

use crate::call::prefix_from_address_with_parent;
use crate::utils::{Payable, TokenHolder, TokenHolderRef};

impl_hash32_type!(TokenId, TokenIdBech32, "token_");

/// Structure that stores information specifying
/// a given `amount` (type [`Amount`]) of coins stored at a `token_id`
/// (type [`crate::TokenId`]).
#[cfg_attr(feature = "native", derive(clap::Parser), derive(schemars::JsonSchema))]
#[derive(
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
pub struct Coins {
    /// The number of tokens
    pub amount: EncryptedAmount,
    /// The ID of the token
    pub token_id: TokenId,
}

/// The errors that might arise when parsing a `Coins` struct from a string.
#[cfg(feature = "native")]
#[derive(Debug, Error)]
pub enum CoinsFromStrError {
    /// The amount could not be deserialized as tfhe ciphertext.
    #[error("Could not parse {input} as a valid amount: {err}")]
    InvalidAmount { input: String, err: anyhow::Error },
    /// The input string was malformed, so the `amount` substring could not be extracted.
    #[error("No amount was provided. Make sure that your input is in the format: amount,token_id. Example: 100,sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqmlyjd6")]
    NoAmountProvided,
    /// The token ID could not be parsed as a valid address.
    #[error("Could not parse {input} as a valid address: {err}")]
    InvalidTokenAddress { input: String, err: anyhow::Error },
    /// The input string was malformed, so the `token_id` substring could not be extracted.
    #[error("No token ID was provided. Make sure that your input is in the format: amount,token_id. Example: 100,sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqmlyjd6")]
    NoTokenAddressProvided,
}

#[cfg(feature = "native")]
impl FromStr for Coins {
    type Err = CoinsFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ',');

        let amount_str = parts.next().ok_or(CoinsFromStrError::NoAmountProvided)?;
        let token_id_str = parts
            .next()
            .ok_or(CoinsFromStrError::NoTokenAddressProvided)?;

        // Check if amount can parse from &str to Vec<u8>
        let amount_byte: Result<Vec<u8>, _> = amount_str
            .split(',')
            .map(|s| s.trim().parse::<u8>())
            .collect();

        // Check if amount can deserialize from Vec<u8> into CompressedFheUint64
        let _: CompressedFheUint64 =
            bincode::deserialize(&amount_byte.clone().unwrap()).map_err(|e| {
                CoinsFromStrError::InvalidAmount {
                    input: amount_str.into(),
                    err: e.into(),
                }
            })?;

        let token_id = TokenId::from_str(token_id_str).map_err(|err| {
            CoinsFromStrError::InvalidTokenAddress {
                input: token_id_str.into(),
                err,
            }
        })?;

        Ok(Self {
            amount: amount_byte.unwrap(),
            token_id,
        })
    }
}
impl std::fmt::Display for Coins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // implement Display for Coins
        write!(f, "token_id={} amount={:?}", self.token_id, self.amount)
    }
}

/// This struct represents a token in the sov-bank module.
#[derive(borsh::BorshDeserialize, borsh::BorshSerialize, Debug, PartialEq, Clone)]
pub struct Token<S: sov_modules_api::Spec> {
    /// Name of the token.
    pub(crate) name: String,
    /// Total supply of the coins.
    pub(crate) total_supply: EncryptedAmount,
    /// Mapping from user address to user balance.
    pub(crate) balances: sov_modules_api::StateMap<TokenHolder<S>, EncryptedAmount>,

    /// Vector containing the authorized minters
    /// Empty vector indicates that the token supply is frozen
    /// Non-empty vector indicates members of the vector can mint.
    /// Freezing a token requires emptying the vector
    /// NOTE: This is explicit, so if a creator doesn't add themselves, then they can't mint
    pub(crate) authorized_minters: Vec<TokenHolder<S>>,
}

impl<S: sov_modules_api::Spec> Token<S> {
    /// Transfer the amount `amount` of tokens from the address `from` to the address `to`.
    /// First checks that there is enough token of that type stored in `from`. If so, update
    /// the balances of the `from` and `to` accounts.
    pub(crate) fn transfer(
        &self,
        from: TokenHolderRef<'_, S>,
        to: TokenHolderRef<'_, S>,
        amount: &EncryptedAmount,
        fhe_public_key: &PublicKey,
        state: &mut impl StateAccessor,
    ) -> anyhow::Result<()> {
        // Do nothing if the sender and receiver are the same
        if from == to {
            return Ok(());
        }

        // Encrypt zero for later use
        let encrypted_zero = FheUint64::try_encrypt(0 as u64, fhe_public_key)?;

        // Deserialize the balances
        let from_balance = match self.balances.get(&from, state)? {
            Some(balance) => bincode::deserialize::<CompressedFheUint64>(&balance)?.decompress(),
            None => bail!("Sender {} does not have a balance", from),
        };
        let to_balance = match self.balances.get(&to, state)? {
            Some(balance) => bincode::deserialize::<CompressedFheUint64>(&balance)?.decompress(),
            None => encrypted_zero.clone(), // if the balance is not found, set it to zero
        };

        // Deserialize the encrypted amount to be transferred
        let amount = bincode::deserialize::<CompressedFheUint64>(amount)?.decompress();

        // Check if the sender has enough balance
        let can_transfer = &from_balance.gt(&amount);

        // If the sender has insufficient balance, transfer zero amount
        let transfer_amount = can_transfer.select(&amount, &encrypted_zero);

        // Update the balances and serialize them
        let from_balance = {
            let new_balance = from_balance - &transfer_amount;
            bincode::serialize(&new_balance.compress())?
        };
        let to_balance = {
            let new_balance = to_balance + &transfer_amount;
            bincode::serialize(&new_balance.compress())?
        };

        // Store the new balances in the state
        self.balances.set(&from, &from_balance, state)?;
        self.balances.set(&to, &to_balance, state)?;
        Ok(())
    }

    /// Freezing a token requires emptying the authorized_minter vector
    /// authorized_minter: Vec<Address> is used to determine if the token is frozen or not
    /// If the vector is empty when the function is called, this means the token is already frozen
    pub(crate) fn freeze(&mut self, sender: TokenHolderRef<'_, S>) -> anyhow::Result<()> {
        let sender = sender.as_token_holder();
        if self.authorized_minters.is_empty() {
            bail!("Token {} is already frozen", self.name)
        }
        self.is_authorized_minter(sender)?;
        self.authorized_minters = vec![];
        Ok(())
    }

    /// Mints a given `amount` of token sent by `sender` to the specified `mint_to_address`.
    /// Checks that the `authorized_minters` set is not empty for the token and that the `sender`
    /// is an `authorized_minter`. If so, update the balances of token for the `mint_to_address` by
    /// adding the minted tokens. Updates the `total_supply` of that token.
    pub(crate) fn mint(
        &mut self,
        authorizer: TokenHolderRef<'_, S>,
        mint_to_identity: TokenHolderRef<'_, S>,
        amount: &EncryptedAmount,
        fhe_public_key: &PublicKey,
        state: &mut impl StateAccessor,
    ) -> anyhow::Result<()> {
        if self.authorized_minters.is_empty() {
            bail!("Attempt to mint frozen token {}", self.name)
        }

        self.is_authorized_minter(authorizer)?;

        // Encrypt zero for later use
        let encrypted_zero = FheUint64::try_encrypt(0 as u64, fhe_public_key)?;

        // get and deserialize the balance of the mint_to_identity
        let to_balance = match self.balances.get(&mint_to_identity, state)? {
            Some(balance) => bincode::deserialize::<CompressedFheUint64>(&balance)?.decompress(),
            None => encrypted_zero.clone(), // if the balance is not found, set it to zero
        };

        // Check if the mint amount is valid
        let encrypted_zero = FheUint64::try_encrypt(0 as u64, fhe_public_key)?;
        let amount = bincode::deserialize::<CompressedFheUint64>(amount)?.decompress();
        let valid_amount = &amount.gt(&encrypted_zero);

        // If the mint amount is invalid, mint zero amount
        let mint_amount = valid_amount.select(&amount, &encrypted_zero);

        // Update the balances
        let to_balance = {
            let new_balance = to_balance + &mint_amount;
            bincode::serialize(&new_balance.compress())?
        };
        self.balances.set(&mint_to_identity, &to_balance, state)?;

        // Update the total supply
        self.total_supply = {
            let total_supply =
                bincode::deserialize::<CompressedFheUint64>(&self.total_supply)?.decompress();
            let new_total_supply = total_supply + &mint_amount;
            bincode::serialize(&new_total_supply.compress())?
        };
        Ok(())
    }

    fn is_authorized_minter(&self, sender: TokenHolderRef<'_, S>) -> anyhow::Result<()> {
        for minter in self.authorized_minters.iter() {
            if sender == minter.as_token_holder() {
                return Ok(());
            }
        }

        bail!(
            "Sender {} is not an authorized minter of token {}",
            sender,
            self.name
        )
    }

    /// Creates a token from a given set of parameters.
    /// The `token_name`, `originator`  (as a `u8` slice), and the `salt` (`u64` number) are used as an input
    /// to an hash function that computes the token ID. Then the initial accounts and balances are populated
    /// from the `identities_and_balances` slice and the `total_supply` of tokens is updated each time.
    /// Returns a tuple containing the computed `token_id` and the created `token` object.
    pub(crate) fn create(
        token_name: &str,
        identities_and_balances: &[(TokenHolderRef<'_, S>, EncryptedAmount)],
        authorized_minters: &[TokenHolderRef<'_, S>],
        originator: impl Payable<S>,
        salt: u64,
        parent_prefix: &Prefix,
        fhe_public_key: &PublicKey,
        compressed_fhe_server_key: &CompressedServerKey,
        state: &mut impl StateReaderAndWriter<User>,
    ) -> anyhow::Result<(TokenId, Self)> {
        let token_id = super::get_token_id::<S>(token_name, originator, salt);
        let token = Self::create_with_token_id(
            token_name,
            identities_and_balances,
            authorized_minters,
            &token_id,
            parent_prefix,
            fhe_public_key,
            compressed_fhe_server_key,
            state,
        )?;
        Ok((token_id, token))
    }

    /// Shouldn't be used directly, only by genesis call
    pub(crate) fn create_with_token_id(
        token_name: &str,
        identities_and_balances: &[(TokenHolderRef<'_, S>, EncryptedAmount)],
        authorized_minters: &[TokenHolderRef<'_, S>],
        token_id: &TokenId,
        parent_prefix: &Prefix,
        fhe_public_key: &PublicKey,
        compressed_fhe_server_key: &CompressedServerKey,
        state: &mut impl StateReaderAndWriter<User>,
    ) -> anyhow::Result<Token<S>> {
        let token_prefix = prefix_from_address_with_parent(parent_prefix, token_id);
        let balances = sov_modules_api::StateMap::new(token_prefix);
        let mut total_supply = FheUint64::default();

        // set GPU server key here for FHE computation
        {
            let gpu_server_key = compressed_fhe_server_key.clone().decompress_to_gpu();
            set_server_key(gpu_server_key);
    
            let encrypted_zero = FheUint64::try_encrypt(0 as u64, fhe_public_key)?;
            total_supply = encrypted_zero;
            for (address, balance) in identities_and_balances.iter() {
                balances.set(address, balance, state)?;
                total_supply = {
                    let balance = bincode::deserialize::<CompressedFheUint64>(balance)?.decompress();
                    // TODO: add total supply overflow check
                    total_supply + &balance
                }
            }
        }

        // set CPU server key here for compression operation for FHE ciphertext
        let mut serialized_total_supply = Vec::new();
        {
            let cpu_server_key = compressed_fhe_server_key.decompress();
            set_server_key(cpu_server_key);

            serialized_total_supply = bincode::serialize(&total_supply.compress())?;
        }

        let authorized_minters = unique_minters(authorized_minters);

        Ok(Token::<S> {
            name: token_name.to_owned(),
            total_supply: serialized_total_supply,
            balances,
            authorized_minters,
        })
    }
}

fn unique_minters<S: Spec>(minters: &[TokenHolderRef<'_, S>]) -> Vec<TokenHolder<S>> {
    // IMPORTANT:
    // We can't just put `authorized_minters` into a `HashSet` because the order of the elements in the `HashSet`` is not guaranteed.
    // The algorithm below ensures that the order of the elements in the `auth_minter_list` is deterministic (both in zk and native execution).
    let mut indices = HashSet::new();
    let mut auth_minter_list = Vec::new();

    for item in minters.iter() {
        if indices.insert(item) {
            auth_minter_list.push(item.into());
        }
    }

    auth_minter_list
}
