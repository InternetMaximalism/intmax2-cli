use ethers::types::{Address as EthAddress, H256, U256 as EthU256};
use intmax2_interfaces::data::deposit_data::TokenType;
use intmax2_zkp::common::signature::key_set::KeySet;
use num_bigint::BigUint;

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

pub fn format_token_info(
    token_type: TokenType,
    amount: Option<EthU256>,
    token_address: Option<EthAddress>,
    token_id: Option<EthU256>,
) -> Result<(EthU256, EthAddress, EthU256), FormatTokenInfoError> {
    match token_type {
        TokenType::NATIVE => Ok({
            let amount = amount.ok_or_else(|| FormatTokenInfoError::MissingAmount)?;
            (amount, EthAddress::zero(), EthU256::zero())
        }),
        TokenType::ERC20 => {
            let amount = amount.ok_or_else(|| FormatTokenInfoError::MissingAmount)?;
            let token_address =
                token_address.ok_or_else(|| FormatTokenInfoError::MissingTokenAddress)?;
            Ok((amount, token_address, EthU256::zero()))
        }
        TokenType::ERC721 => {
            if amount.is_some() {
                return Err(FormatTokenInfoError::AmountShouldNotBeSpecified.into());
            }
            let token_address =
                token_address.ok_or_else(|| FormatTokenInfoError::MissingTokenAddress)?;
            let token_id = token_id.ok_or_else(|| FormatTokenInfoError::MissingTokenId)?;
            Ok((EthU256::one(), token_address, token_id))
        }
        TokenType::ERC1155 => {
            let amount = amount.ok_or_else(|| FormatTokenInfoError::MissingAmount)?;
            let token_address =
                token_address.ok_or_else(|| FormatTokenInfoError::MissingTokenAddress)?;
            let token_id = token_id.ok_or_else(|| FormatTokenInfoError::MissingTokenId)?;
            Ok((amount, token_address, token_id))
        }
    }
}

pub fn h256_to_keyset(h256: H256) -> KeySet {
    KeySet::new(BigUint::from_bytes_be(h256.as_bytes()).into())
}
