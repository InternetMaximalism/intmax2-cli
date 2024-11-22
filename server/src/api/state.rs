use intmax2_core_sdk::external_api::{
    balance_prover::local::LocalBalanceProver, block_builder::local::LocalBlockBuilder,
    block_validity_prover::local::LocalBlockValidityProver, contract::local::LocalContract,
    store_vault_server::local::LocalStoreVaultServer,
    withdrawal_aggregator::local::LocalWithdrawalAggregator,
};

pub struct State {
    pub contract: LocalContract,
    pub store_vault_server: LocalStoreVaultServer,
    pub validity_prover: LocalBlockValidityProver,
    pub block_builder: LocalBlockBuilder,
    pub balance_prover: LocalBalanceProver,
    pub withdrawal_aggregator: LocalWithdrawalAggregator,
}

impl State {
    pub fn new() -> anyhow::Result<Self> {
        log::info!("Initializing contract");
        let contract = LocalContract::new();

        log::info!("Initializing store_vault_server");
        let store_vault_server = LocalStoreVaultServer::new();

        log::info!("Initializing validity_prover");
        let validity_prover = LocalBlockValidityProver::new(contract.0.clone());

        log::info!("Initializing block_builder");
        let block_builder = LocalBlockBuilder::new(
            contract.0.clone(),
            validity_prover.inner_block_validity_prover.clone(),
        );

        log::info!("Initializing balance_prover");
        let balance_prover = LocalBalanceProver::new()?;

        log::info!("Initializing withdrawal_aggregator");
        let withdrawal_aggregator = LocalWithdrawalAggregator::new()?;

        log::info!("Initializing provers");
        validity_prover
            .inner_block_validity_prover
            .lock()
            .unwrap()
            .validity_processor();
        withdrawal_aggregator
            .0
            .lock()
            .unwrap()
            .withdrawal_processor();

        log::info!("State initialized");
        Ok(Self {
            contract,
            store_vault_server,
            validity_prover,
            block_builder,
            balance_prover,
            withdrawal_aggregator,
        })
    }

    pub fn reset(&self) {
        self.contract.reset();
        self.store_vault_server.reset();
        self.validity_prover.reset();
        self.block_builder.reset();
        self.withdrawal_aggregator.reset();
    }
}
