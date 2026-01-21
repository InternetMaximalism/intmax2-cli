use crate::cli::error::CliError;
use intmax2_interfaces::{
    data::deposit_data::TokenType,
    utils::{
        key::{KeyPair, PrivateKey, ViewPair},
        key_derivation::derive_keypair_from_spend_key,
    },
};
use intmax2_zkp::ethereum_types::{
    address::Address, bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait as _,
};
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum FormatTokenInfoError {
    #[error("Missing amount")]
    MissingAmount,
    #[error("Missing token address")]
    MissingTokenAddress,
    #[error("Missing token id")]
    MissingTokenId,
    #[error("Amount should not be specified")]
    AmountShouldNotBeSpecified,
}

pub struct TokenInput {
    pub token_type: TokenType,
    pub amount: Option<U256>,
    pub token_address: Option<Address>,
    pub token_id: Option<U256>,
}

pub fn format_token_info(input: TokenInput) -> Result<(U256, Address, U256), FormatTokenInfoError> {
    match input.token_type {
        TokenType::NATIVE => Ok((
            input.amount.ok_or(FormatTokenInfoError::MissingAmount)?,
            Address::zero(),
            U256::zero(),
        )),
        TokenType::ERC20 => Ok((
            input.amount.ok_or(FormatTokenInfoError::MissingAmount)?,
            input
                .token_address
                .ok_or(FormatTokenInfoError::MissingTokenAddress)?,
            U256::zero(),
        )),
        TokenType::ERC721 => {
            if input.amount.is_some() {
                return Err(FormatTokenInfoError::AmountShouldNotBeSpecified);
            }
            Ok((
                U256::one(),
                input
                    .token_address
                    .ok_or(FormatTokenInfoError::MissingTokenAddress)?,
                input.token_id.ok_or(FormatTokenInfoError::MissingTokenId)?,
            ))
        }
        TokenType::ERC1155 => Ok((
            input.amount.ok_or(FormatTokenInfoError::MissingAmount)?,
            input
                .token_address
                .ok_or(FormatTokenInfoError::MissingTokenAddress)?,
            input.token_id.ok_or(FormatTokenInfoError::MissingTokenId)?,
        )),
    }
}

pub fn privkey_to_keypair(privkey: Bytes32) -> KeyPair {
    let is_legacy = std::env::var("LEGACY_ACCOUNT")
        .map(|s| s == "true")
        .unwrap_or(false);
    derive_keypair_from_spend_key(PrivateKey(privkey.into()), is_legacy)
}

pub fn viewkey_to_viewpair(
    viewkey: &str,
) -> Result<ViewPair, intmax2_interfaces::utils::key::Error> {
    ViewPair::from_str(viewkey)
}

/// Resolves ViewPair from either private_key or view_key option
pub fn resolve_view_pair(
    private_key: Option<Bytes32>,
    view_key: Option<String>,
) -> Result<ViewPair, CliError> {
    match (private_key, view_key) {
        (Some(pk), _) => Ok(privkey_to_keypair(pk).into()),
        (None, Some(vk)) => {
            viewkey_to_viewpair(&vk).map_err(|e| CliError::ParseError(e.to_string()))
        }
        (None, None) => Err(CliError::ParseError(
            "Either --private-key or --view-key must be provided".to_string(),
        )),
    }
}
