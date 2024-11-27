use std::str::FromStr;

use intmax2_zkp::{
    common::salt::Salt,
    ethereum_types::{address::Address, bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait},
    utils::poseidon_hash_out::PoseidonHashOut,
};
use num_bigint::BigUint;
use wasm_bindgen::JsError;

pub fn parse_u256(input: &str) -> Result<U256, JsError> {
    let input = BigUint::from_str(input).map_err(|_| JsError::new("Failed to parse as BigUint"))?;
    let input = input
        .try_into()
        .map_err(|_| JsError::new("Failed to cast to u256"))?;
    Ok(input)
}

pub fn parse_address(input: &str) -> Result<Address, JsError> {
    let input = Address::from_hex(input).map_err(|_| JsError::new("Failed to parse as Address"))?;
    Ok(input)
}

pub fn parse_poseidon_hashout(input: &str) -> Result<PoseidonHashOut, JsError> {
    let input = Bytes32::from_hex(input).map_err(|_| JsError::new("Failed to parse as Bytes32"))?;
    Ok(input.reduce_to_hash_out())
}

pub fn parse_salt(input: &str) -> Result<Salt, JsError> {
    let input = parse_poseidon_hashout(input)?;
    Ok(Salt(input))
}
