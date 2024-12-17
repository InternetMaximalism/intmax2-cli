use ethers::types::H256;
use intmax2_client_sdk::external_api::store_vault_server::StoreVaultServerClient;
use intmax2_interfaces::{
    api::store_vault_server::interface::StoreVaultClientInterface, data::user_data::UserData,
};
use intmax2_zkp::common::signature::key_set::KeySet;
use num_bigint::BigUint;
use serde::Deserialize;

#[derive(Deserialize)]
struct EnvVar {
    pub user_private_key: H256,
    pub store_vault_server_base_url: String,
}

#[tokio::test]
async fn test_store_vault_server() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let config = envy::from_env::<EnvVar>().unwrap();

    let key = KeySet::new(BigUint::from_bytes_be(config.user_private_key.as_bytes()).into());
    let store_vault_server = StoreVaultServerClient::new(&config.store_vault_server_base_url);

    let user_data = UserData::new(key.pubkey).encrypt(key.pubkey);
    store_vault_server
        .save_user_data(key.pubkey, user_data)
        .await?;
    Ok(())
}
