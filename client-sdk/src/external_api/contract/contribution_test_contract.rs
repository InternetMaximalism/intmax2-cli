use super::{
    convert::{convert_b256_to_bytes32, convert_bytes32_to_b256},
    error::BlockchainError,
    handlers::send_transaction_with_gas_bump,
    utils::{get_provider_with_signer, NormalProvider},
};
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    sol,
};
use intmax2_zkp::ethereum_types::bytes32::Bytes32;

sol!(
    #[sol(rpc)]
    ContributionTest,
    "abi/ContributionTest.json",
);

#[derive(Debug, Clone)]
pub struct ContributionTestContract {
    pub provider: NormalProvider,
    pub address: Address,
}

impl ContributionTestContract {
    pub fn new(provider: NormalProvider, address: Address) -> Self {
        Self { provider, address }
    }

    pub async fn deploy(provider: NormalProvider, private_key: B256) -> anyhow::Result<Self> {
        let signer = get_provider_with_signer(&provider, private_key);
        let contract = ContributionTest::deploy(signer).await?;
        let address = *contract.address();
        Ok(Self { provider, address })
    }

    pub async fn record_contribution(
        &self,
        signer_private_key: B256,
        gas_limit: Option<u64>,
        tag: Bytes32,
        user: Address,
        amount: U256,
    ) -> Result<B256, BlockchainError> {
        let signer = get_provider_with_signer(&self.provider, signer_private_key);
        let contract = ContributionTest::new(self.address, signer.clone());
        let mut tx_request = contract
            .recordContribution(convert_bytes32_to_b256(tag), user, amount)
            .into_transaction_request();
        if let Some(gas_limit) = gas_limit {
            tx_request.set_gas_limit(gas_limit);
        }
        send_transaction_with_gas_bump(signer, tx_request, "record_contribution").await
    }

    pub async fn latest_tag(&self) -> Result<Bytes32, BlockchainError> {
        let contract = ContributionTest::new(self.address, self.provider.clone());
        let tag = contract.latestTag().call().await?;
        Ok(convert_b256_to_bytes32(tag))
    }

    pub async fn latest_user(&self) -> Result<Address, BlockchainError> {
        let contract = ContributionTest::new(self.address, self.provider.clone());
        let user = contract.latestUser().call().await?;
        Ok(user)
    }

    pub async fn latest_amount(&self) -> Result<U256, BlockchainError> {
        let contract = ContributionTest::new(self.address, self.provider.clone());
        let amount = contract.latestAmount().call().await?;
        Ok(amount)
    }
}
