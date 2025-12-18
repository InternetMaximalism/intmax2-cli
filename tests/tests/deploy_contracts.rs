use alloy::primitives::{Address, B256, U256};
use intmax2_client_sdk::external_api::contract::{
    block_builder_registry::BlockBuilderRegistryContract,
    contribution_test_contract::ContributionTestContract,
    erc1155_contract::ERC1155Contract,
    erc20_contract::ERC20Contract,
    erc721_contract::ERC721Contract,
    liquidity_contract::LiquidityContract,
    mock_l2_scroll_messenger::MockL2ScrollMessengerContract,
    rollup_contract::RollupContract,
    utils::{get_address_from_private_key, get_provider_with_fallback},
    withdrawal_contract::WithdrawalContract,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct EnvVar {
    pub rpc_url: String,
    pub deployer_private_key: B256,
}

#[tokio::test]
#[ignore]
async fn deploy_contracts() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = envy::from_env::<EnvVar>().unwrap();

    let provider = get_provider_with_fallback(std::slice::from_ref(&config.rpc_url)).unwrap();
    let deployer = get_address_from_private_key(config.deployer_private_key);

    let rollup_contract =
        RollupContract::deploy(provider.clone(), config.deployer_private_key).await?;
    let liquidity_contract =
        LiquidityContract::deploy(provider.clone(), config.deployer_private_key).await?;
    let contribution_contract =
        ContributionTestContract::deploy(provider.clone(), config.deployer_private_key).await?;
    let scroll_messenger_contract =
        MockL2ScrollMessengerContract::deploy(provider.clone(), config.deployer_private_key)
            .await?;
    rollup_contract
        .initialize(
            config.deployer_private_key,
            deployer,
            scroll_messenger_contract.address,
            liquidity_contract.address,
            contribution_contract.address,
            U256::from(0),
            U256::from(0),
            U256::from(0),
        )
        .await?;

    println!("Rollup contract address: {:?}", rollup_contract.address);

    let registry_contract =
        BlockBuilderRegistryContract::deploy(provider.clone(), config.deployer_private_key).await?;

    println!("registry contract address: {:?}", registry_contract.address);

    let withdrawal_contract =
        WithdrawalContract::deploy(provider.clone(), config.deployer_private_key).await?;
    let placeholder_address_1 = Address::random();
    let placeholder_address_2 = Address::random();
    withdrawal_contract
        .initialize(
            config.deployer_private_key,
            deployer,
            scroll_messenger_contract.address,
            placeholder_address_1,
            liquidity_contract.address,
            rollup_contract.address,
            contribution_contract.address,
            vec![U256::from(0), U256::from(1), U256::from(2)],
        )
        .await?;
    println!(
        "withdrawal contract address: {:?}",
        withdrawal_contract.address
    );

    liquidity_contract
        .initialize(
            config.deployer_private_key,
            deployer,
            placeholder_address_2,
            rollup_contract.address,
            withdrawal_contract.address,
            placeholder_address_2,
            placeholder_address_2,
            contribution_contract.address,
            vec![],
        )
        .await?;

    println!(
        "Liquidity contract address: {:?}",
        liquidity_contract.address
    );

    println!(
        "Mock L2 Scroll Messenger contract address: {:?}",
        scroll_messenger_contract.address
    );

    println!(
        "Contribution contract address: {:?}",
        contribution_contract.address
    );

    let erc20_token =
        ERC20Contract::deploy(provider.clone(), config.deployer_private_key, deployer).await?;
    println!("erc20 contract address: {:?}", erc20_token.address);

    let erc721_token =
        ERC721Contract::deploy(provider.clone(), config.deployer_private_key).await?;
    println!("erc721 contract address: {:?}", erc721_token.address);

    let erc1155_token =
        ERC1155Contract::deploy(provider.clone(), config.deployer_private_key).await?;
    // mint some token
    erc1155_token.setup(config.deployer_private_key).await?;

    println!("erc1155 contract address: {:?}", erc1155_token.address);

    Ok(())
}
