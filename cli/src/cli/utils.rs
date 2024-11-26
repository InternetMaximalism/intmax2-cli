use ethers::types::{Address, U256};
use intmax2_zkp::ethereum_types::u32limb_trait::U32LimbTrait as _;

pub fn convert_u256(input: U256) -> intmax2_zkp::ethereum_types::u256::U256 {
    let mut bytes = [0u8; 32];
    input.to_big_endian(&mut bytes);
    let amount = intmax2_zkp::ethereum_types::u256::U256::from_bytes_be(&bytes);
    amount
}

pub fn convert_address(input: Address) -> intmax2_zkp::ethereum_types::address::Address {
    let address =
        intmax2_zkp::ethereum_types::address::Address::from_bytes_be(&input.to_fixed_bytes());
    address
}
