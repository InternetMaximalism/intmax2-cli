use ark_bn254::Fr;
use intmax2_zkp::common::signature::key_set::KeySet;
use num_bigint::BigUint;
use num_traits::identities::Zero;
use sha2::{Digest, Sha512};

pub enum AccountError {
    ErrInputPrivateKeyEmpty,
    ErrInputPrivateKeyIsZero,
    ErrValidPublicKeyFail,
}

fn generate_private_key_with_recalculation(private_key: &[u8]) -> Result<KeySet, AccountError> {
    let private_key_fr: Fr = BigUint::from_bytes_be(private_key).into();
    if private_key_fr.is_zero() {
        return Err(AccountError::ErrInputPrivateKeyIsZero);
    }

    let key_set = KeySet::generate_from_provisional(private_key_fr);

    return Ok(key_set);
}

pub fn generate_intmax_account_from_ecdsa_key(ecdsa_private_key: &[u8]) -> KeySet {
    if ecdsa_private_key.len() != 32 {
        panic!("Invalid private key length");
    }

    let mut hasher = Sha512::new();
    loop {
        hasher.update(b"INTMAX");
        hasher.update(ecdsa_private_key);
        let digest = hasher.clone().finalize();

        match generate_private_key_with_recalculation(digest.as_slice()) {
            Ok(account) => {
                // TODO: Verify that the private key can actually be used to sign and verify the signature.
                return account;
            }
            Err(_) => {
                continue;
            }
        };
    }
}

#[cfg(test)]
mod test {
    use intmax2_zkp::ethereum_types::u32limb_trait::U32LimbTrait;

    use crate::client::account::generate_intmax_account_from_ecdsa_key;

    struct TestCase {
        private_key: String,
        public_key: String,
    }

    #[test]
    fn test_account() {
        let test_cases = [
            TestCase {
                private_key: "f68ff926147a67518161e65cd54a3a44c2379e4b63c74b52cfc74274d2586299"
                    .to_string(),
                public_key: "0x2f2ddf326b1b4528706ecab6ff465b15cc1f4a4a2d8ea5d39d66ffb0a91a277c"
                    .to_string(),
            },
            TestCase {
                private_key: "3db985c15e2788a9f03a797c71151571cbbd0cb2a89402f640102cb8b445e59a"
                    .to_string(),
                public_key: "0x17aebd78d4259e734ba1c9ce1b58c9adea5ab3e68c61e6251dd3016085101941"
                    .to_string(),
            },
            TestCase {
                private_key: "962bc2ea6e76fc3863906a894f3b17cce375ff298c7c5efcf0d4ce9d054e7e4e"
                    .to_string(),
                public_key: "0x1fb62949642c57749922484377541e70445881599cfb19c74066fe0f885510af"
                    .to_string(),
            },
            TestCase {
                private_key: "25be37b3ca8370a172765133f23c849905f21ed2dd90422bc8901cbbe69e3e1c"
                    .to_string(),
                public_key: "0x2c8ffeb9b3a365c0387f841973defbb203be92a509f075a0821aaeec79f7080f"
                    .to_string(),
            },
        ];

        for test_case in test_cases.iter() {
            let ecdsa_private_key = hex::decode(&test_case.private_key).unwrap();
            let account = generate_intmax_account_from_ecdsa_key(&ecdsa_private_key);
            assert_eq!(account.is_dummy, false);
            assert_eq!(account.pubkey.to_hex(), test_case.public_key);
        }
    }
}
