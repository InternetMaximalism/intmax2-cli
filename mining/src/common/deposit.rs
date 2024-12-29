use ethers::utils::keccak256;
use intmax2_zkp::{common::salt::Salt, utils::poseidon_hash_out::PoseidonHashOut};

const DEPOSIT_SALT_PREFIX: &str =
    "bf21c6520d666a4167f35c091393809e314f62a8e5cb1c166dd4dcac3abe53ad";

pub fn generate_deterministic_salt(
    ethereum_private_key: &str,
    ethereum_transaction_count: u64,
) -> Salt {
    let deposit_salt_prefix_bytes = hex::decode(DEPOSIT_SALT_PREFIX).unwrap();
    let ethereum_private_key_bytes = hex::decode(&ethereum_private_key[2..]).unwrap();
    let prefixed_private_key = vec![deposit_salt_prefix_bytes, ethereum_private_key_bytes].concat();

    let hashed_private_key = keccak256(prefixed_private_key);
    let nonce_bytes = ethereum_transaction_count.to_be_bytes();
    let prefixed_salt_pre_image = vec![&hashed_private_key[..], &nonce_bytes[..]].concat();
    let prefixed_salt = keccak256(prefixed_salt_pre_image);
    let mut prefixed_salt_vec: Vec<u64> = vec![];
    for i in 0..prefixed_salt.len() / 8 {
        prefixed_salt_vec.push(u64::from_be_bytes(
            prefixed_salt[8 * i..8 * (i + 1)].try_into().unwrap(),
        ));
    }

    Salt(PoseidonHashOut::from_u64_slice(&prefixed_salt_vec))
}
