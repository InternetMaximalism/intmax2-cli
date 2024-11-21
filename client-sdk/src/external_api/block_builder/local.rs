use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use intmax2_zkp::{
    common::{
        block_builder::{BlockProposal, UserSignature},
        signature::flatten::FlatG2,
        tx::Tx,
    },
    ethereum_types::u256::U256,
    mock::{
        block_builder::BlockBuilder as InnerBlockBuilder,
        block_validity_prover::BlockValidityProver, contract::MockContract,
    },
};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};

use crate::external_api::common::error::ServerError;

use super::interface::{BlockBuilderInterface, FeeProof};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct LocalBlockBuilder {
    pub contract: Arc<Mutex<MockContract>>,
    pub validity_prover: Arc<Mutex<BlockValidityProver<F, C, D>>>,
    pub inner_block_builder: Arc<Mutex<InnerBlockBuilder>>,
}

// Methods called by the block builder
impl LocalBlockBuilder {
    pub fn new(
        contract: Arc<Mutex<MockContract>>,
        validity_prover: Arc<Mutex<BlockValidityProver<F, C, D>>>,
    ) -> Self {
        let inner_block_builder = Arc::new(Mutex::new(InnerBlockBuilder::new()));
        Self {
            contract,
            validity_prover,
            inner_block_builder,
        }
    }

    pub fn construct_block(&self) -> Result<(), ServerError> {
        self.inner_block_builder
            .lock()
            .unwrap()
            .construct_block()
            .map_err(|e| {
                ServerError::InternalError(format!("Failed to construct block: {}", e.to_string()))
            })?;
        Ok(())
    }

    pub fn post_block(&self) -> Result<(), ServerError> {
        let mut contract = self.contract.lock().unwrap();
        let validity_prover = self.validity_prover.lock().unwrap();
        self.inner_block_builder
            .lock()
            .unwrap()
            .post_block(&mut contract, &validity_prover)
            .map_err(|e| {
                ServerError::InternalError(format!("Failed to post block: {}", e.to_string()))
            })?;
        Ok(())
    }

    pub fn post_empty_block(&self) -> Result<(), ServerError> {
        let mut contract = self.contract.lock().unwrap();
        let validity_prover = self.validity_prover.lock().unwrap();
        self.inner_block_builder
            .lock()
            .unwrap()
            .post_empty_block(&mut contract, &validity_prover)
            .map_err(|e| {
                ServerError::InternalError(format!("Failed to post empty block: {}", e.to_string()))
            })?;
        Ok(())
    }

    pub fn reset(&self) {
        self.inner_block_builder.lock().unwrap().reset();
    }
}

#[async_trait(?Send)]
impl BlockBuilderInterface for LocalBlockBuilder {
    async fn send_tx_request(
        &self,
        _block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
        _fee_proof: Option<FeeProof>,
    ) -> Result<(), ServerError> {
        let validity_prover = self.validity_prover.lock().unwrap();
        self.inner_block_builder
            .lock()
            .unwrap()
            .send_tx_request(&validity_prover, pubkey, tx)
            .map_err(|e| {
                ServerError::InternalError(format!("Failed to send tx request: {}", e.to_string()))
            })?;
        Ok(())
    }

    async fn query_proposal(
        &self,
        _block_builder_url: &str,
        pubkey: U256,
        _tx: Tx,
    ) -> Result<Option<BlockProposal>, ServerError> {
        let proposal = self
            .inner_block_builder
            .lock()
            .unwrap()
            .query_proposal(pubkey)
            .map_err(|e| {
                ServerError::InternalError(format!("Failed to query proposal: {}", e.to_string()))
            })?;
        Ok(proposal)
    }

    async fn post_signature(
        &self,
        _block_builder_url: &str,
        pubkey: U256,
        _tx: Tx,
        signature: FlatG2,
    ) -> Result<(), ServerError> {
        let user_signatre = UserSignature { pubkey, signature };
        self.inner_block_builder
            .lock()
            .unwrap()
            .post_signature(user_signatre)
            .map_err(|e| {
                ServerError::InternalError(format!("Failed to post signature: {}", e.to_string()))
            })?;
        Ok(())
    }
}
