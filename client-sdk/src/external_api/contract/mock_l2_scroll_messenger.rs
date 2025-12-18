use super::{
    error::BlockchainError,
    handlers::send_transaction_with_gas_bump,
    utils::{get_provider_with_signer, NormalProvider},
};
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, B256, U256},
    sol,
};

sol!(
    #[sol(rpc)]
    MockL2ScrollMessenger,
    "abi/MockL2ScrollMessenger.json",
);

#[derive(Debug, Clone)]
pub struct MockL2ScrollMessengerContract {
    pub provider: NormalProvider,
    pub address: Address,
}

impl MockL2ScrollMessengerContract {
    pub fn new(provider: NormalProvider, address: Address) -> Self {
        Self { provider, address }
    }

    pub async fn deploy(provider: NormalProvider, private_key: B256) -> anyhow::Result<Self> {
        let signer = get_provider_with_signer(&provider, private_key);
        let contract = MockL2ScrollMessenger::deploy(signer).await?;
        let address = *contract.address();
        Ok(Self { provider, address })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn relay_message(
        &self,
        signer_private_key: B256,
        from: Address,
        to: Address,
        value: U256,
        nonce: U256,
        message: Bytes,
        gas_limit: Option<u64>,
    ) -> Result<(), BlockchainError> {
        let signer = get_provider_with_signer(&self.provider, signer_private_key);
        let contract = MockL2ScrollMessenger::new(self.address, signer.clone());
        let mut tx_request = contract
            .relayMessage(from, to, value, nonce, message)
            .into_transaction_request();
        if let Some(gas_limit) = gas_limit {
            tx_request.set_gas_limit(gas_limit);
        }
        send_transaction_with_gas_bump(signer, tx_request, "relay_message").await?;
        Ok(())
    }
}
