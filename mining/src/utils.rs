use ethers::types::H256;
use intmax2_zkp::common::signature::key_set::KeySet;
use num_bigint::BigUint;

pub fn h256_to_keyset(h256: H256) -> KeySet {
    KeySet::new(BigUint::from_bytes_be(h256.as_bytes()).into())
}
