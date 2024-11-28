use ark_bn254::{Bn254, Fr, G1Affine, G2Affine};
use ark_ec::{pairing::Pairing as _, AffineRepr as _};
use ethers::types::{Address, H256};
use intmax2_client_sdk::external_api::{
    contract::rollup_contract::RollupContract, validity_prover::ValidityProverClient,
};
use intmax2_interfaces::api::{
    block_builder::interface::BlockBuilderStatus,
    validity_prover::interface::ValidityProverClientInterface,
};
use intmax2_zkp::{
    common::{
        block_builder::{BlockProposal, UserSignature},
        signature::{
            flatten::FlatG2,
            sign::{hash_to_weight, tx_tree_root_to_message_point},
            SignatureContent,
        },
        tx::Tx,
    },
    constants::NUM_SENDERS_IN_BLOCK,
    ethereum_types::{
        account_id_packed::AccountIdPacked, bytes16::Bytes16, bytes32::Bytes32, u256::U256,
        u32limb_trait::U32LimbTrait,
    },
};
use num::BigUint;
use plonky2_bn254::fields::recover::RecoverFromX as _;

use super::{error::BlockBuilderError, internal_state::BuilderState};

#[derive(Debug, Clone)]
pub struct BlockBuilder {
    validity_prover_client: ValidityProverClient,
    rollup_contract: RollupContract,
    block_builder_private_key: H256,
    eth_allowance_for_block: ethers::types::U256,

    registration_state: BuilderState,
    non_registration_state: BuilderState,
}

// todo: remove status clone
impl BlockBuilder {
    pub fn new(
        rpc_url: &str,
        chain_id: u64,
        rollup_contract_address: Address,
        rollup_contract_deployed_block_number: u64,
        block_builder_private_key: H256,
        eth_allowance_for_block: ethers::types::U256,
        validity_prover_base_url: &str,
    ) -> Self {
        let validity_prover_client = ValidityProverClient::new(validity_prover_base_url);
        let rollup_contract = RollupContract::new(
            rpc_url,
            chain_id,
            rollup_contract_address,
            rollup_contract_deployed_block_number,
        );
        Self {
            validity_prover_client,
            rollup_contract,
            block_builder_private_key,
            eth_allowance_for_block,
            registration_state: BuilderState::new(),
            non_registration_state: BuilderState::new(),
        }
    }

    pub fn get_status(&self, is_registration_block: bool) -> BlockBuilderStatus {
        if is_registration_block {
            self.registration_state.get_status()
        } else {
            self.non_registration_state.get_status()
        }
    }

    // Send a tx request by the user.
    pub async fn send_tx_request(
        &mut self,
        is_registration_block: bool,
        pubkey: U256,
        tx: Tx,
    ) -> Result<(), BlockBuilderError> {
        let mut status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };

        if !status.is_accepting_txs() {
            return Err(BlockBuilderError::NotAcceptingTx);
        }
        if status.count_tx_requests() >= NUM_SENDERS_IN_BLOCK {
            return Err(BlockBuilderError::BlockIsFull);
        }
        if status.is_pubkey_contained(pubkey) {
            return Err(BlockBuilderError::OnlyOneSenderAllowed);
        }

        // registration check
        let block_number = self.rollup_contract.get_latest_block_number().await?;
        let account_info = self.validity_prover_client.get_account_info(pubkey).await?;
        if block_number != account_info.block_number {
            // todo: better error handling, maybe wait for the validity prover to sync
            return Err(BlockBuilderError::ValidityProverIsNotSynced(
                block_number,
                account_info.block_number,
            ));
        }

        if is_registration_block {
            if let Some(account_id) = account_info.account_id {
                return Err(BlockBuilderError::AccountAlreadyRegistered(
                    pubkey, account_id,
                ));
            }
        } else {
            if account_info.account_id.is_none() {
                return Err(BlockBuilderError::AccountNotFound(pubkey));
            }
        }

        // update state
        status.append_tx_request(pubkey, tx);
        if is_registration_block {
            self.registration_state = status;
        } else {
            self.non_registration_state = status;
        }
        Ok(())
    }

    // Construct a block with the given tx requests by the block builder.
    pub fn construct_block(
        &mut self,
        is_registration_block: bool,
    ) -> Result<(), BlockBuilderError> {
        log::info!(
            "construct_block is_registration_block: {}",
            is_registration_block
        );
        let mut status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };

        if !status.is_accepting_txs() {
            return Err(BlockBuilderError::NotAcceptingTx);
        }

        // update state
        status.propose_block();
        if is_registration_block {
            self.registration_state = status;
        } else {
            self.non_registration_state = status;
        }
        Ok(())
    }

    // Query the constructed proposal by the user.
    pub fn query_proposal(
        &self,
        is_registration_block: bool,
        pubkey: U256,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, BlockBuilderError> {
        let status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };
        if status.is_pausing() {
            return Err(BlockBuilderError::BlockBuilderIsPausing);
        }
        if status.is_accepting_txs() {
            if !status.is_request_contained(pubkey, tx) {
                return Err(BlockBuilderError::TxRequestNotFound);
            }
        }
        return Ok(status.query_proposal(pubkey, tx));
    }

    // Post the signature by the user.
    pub fn post_signature(
        &mut self,
        is_registration_block: bool,
        tx: Tx,
        signature: UserSignature,
    ) -> Result<(), BlockBuilderError> {
        log::info!(
            "post_signature is_registration_block: {}",
            is_registration_block
        );
        let mut status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };
        if !status.is_proposing_block() {
            return Err(BlockBuilderError::NotProposing);
        }
        if status.is_request_contained(signature.pubkey, tx) {
            return Err(BlockBuilderError::TxRequestNotFound);
        }
        let memo = status.get_proposal_memo().unwrap();
        signature
            .verify(memo.tx_tree_root, memo.pubkey_hash)
            .map_err(|e| BlockBuilderError::InvalidSignature(e.to_string()))?;

        // update state
        status.append_signature(signature);
        if is_registration_block {
            self.registration_state = status;
        } else {
            self.non_registration_state = status;
        }
        Ok(())
    }

    pub async fn num_tx_requests(
        &self,
        is_registration_block: bool,
    ) -> Result<usize, BlockBuilderError> {
        let status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };
        Ok(status.count_tx_requests())
    }

    // Post the block with the given signatures.
    pub async fn post_block(
        &mut self,
        is_registration_block: bool,
    ) -> Result<(), BlockBuilderError> {
        log::info!(
            "post_block is_registration_block: {}",
            is_registration_block
        );
        let mut status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };
        if !status.is_proposing_block() {
            return Err(BlockBuilderError::NotProposing);
        }
        let memo = status.get_proposal_memo().unwrap();
        let mut account_id_packed = None;

        if is_registration_block {
            for pubkey in memo.pubkeys.iter() {
                if pubkey.is_dummy_pubkey() {
                    // ignore dummy pubkey
                    continue;
                }
                let account_info = self
                    .validity_prover_client
                    .get_account_info(*pubkey)
                    .await?;
                if account_info.account_id.is_some() {
                    // This is unrecoverable so abandon the block
                    self.reset(is_registration_block);
                    return Err(BlockBuilderError::AccountAlreadyRegistered(
                        *pubkey,
                        account_info.account_id.unwrap(),
                    ));
                }
            }
        } else {
            let mut account_ids = Vec::new();
            for pubkey in memo.pubkeys.iter() {
                if pubkey.is_dummy_pubkey() {
                    account_ids.push(1); // dummy account id
                    continue;
                }
                let account_info = self
                    .validity_prover_client
                    .get_account_info(*pubkey)
                    .await?;
                if account_info.account_id.is_none() {
                    // This is unrecoverable so abandon the block
                    self.reset(is_registration_block);
                    return Err(BlockBuilderError::AccountNotFound(*pubkey));
                }
                account_ids.push(account_info.account_id.unwrap());
            }
            account_id_packed = Some(AccountIdPacked::pack(&account_ids));
        }

        let account_id_hash = account_id_packed.map_or(Bytes32::default(), |ids| ids.hash());
        let mut sender_with_signatures = memo
            .pubkeys
            .iter()
            .map(|pubkey| SenderWithSignature {
                sender: *pubkey,
                signature: None,
            })
            .collect::<Vec<_>>();

        let signatures = status.get_signatures().unwrap();
        for signature in signatures.iter() {
            let tx_index = memo
                .pubkeys
                .iter()
                .position(|pubkey| pubkey == &signature.pubkey)
                .unwrap(); // safe
            sender_with_signatures[tx_index].signature = Some(signature.signature.clone());
        }
        let signature = construct_signature(
            memo.tx_tree_root,
            memo.pubkey_hash,
            account_id_hash,
            is_registration_block,
            &sender_with_signatures,
        );

        // call contract
        if is_registration_block {
            let trimmed_pubkeys = memo
                .pubkeys
                .into_iter()
                .filter(|pubkey| !pubkey.is_dummy_pubkey())
                .collect::<Vec<_>>();
            self.rollup_contract
                .post_registration_block(
                    self.block_builder_private_key,
                    self.eth_allowance_for_block,
                    memo.tx_tree_root,
                    signature.sender_flag,
                    signature.agg_pubkey,
                    signature.agg_signature,
                    signature.message_point,
                    trimmed_pubkeys,
                )
                .await?;
        } else {
            self.rollup_contract
                .post_non_registration_block(
                    self.block_builder_private_key,
                    self.eth_allowance_for_block,
                    memo.tx_tree_root,
                    signature.sender_flag,
                    signature.agg_pubkey,
                    signature.agg_signature,
                    signature.message_point,
                    memo.pubkey_hash,
                    account_id_packed.unwrap().to_trimmed_bytes(),
                )
                .await?;
        };
        status.finalize_block();
        if is_registration_block {
            self.registration_state = status;
        } else {
            self.non_registration_state = status;
        }
        Ok(())
    }

    pub fn start_accepting_txs(
        &mut self,
        is_registration_block: bool,
    ) -> Result<(), BlockBuilderError> {
        log::info!(
            "start_accepting_txs is_registration_block: {}",
            is_registration_block
        );
        let mut status = if is_registration_block {
            self.registration_state.clone()
        } else {
            self.non_registration_state.clone()
        };
        if !status.is_pausing() {
            return Err(BlockBuilderError::ShouldBePausing);
        }
        status.start_accepting_txs();
        if is_registration_block {
            self.registration_state = status;
        } else {
            self.non_registration_state = status;
        }
        Ok(())
    }

    pub async fn post_empty_block(&mut self) -> Result<(), BlockBuilderError> {
        log::info!("post_empty_block");
        let is_registration_block = false;
        self.start_accepting_txs(is_registration_block)?;
        self.construct_block(is_registration_block)?;
        self.post_block(is_registration_block).await?;
        Ok(())
    }

    /// Reset the block builder.
    pub fn reset(&mut self, is_registration_block: bool) {
        log::info!("reset");
        if is_registration_block {
            self.registration_state = BuilderState::new();
        } else {
            self.non_registration_state = BuilderState::new();
        }
    }
}

struct SenderWithSignature {
    sender: U256,
    signature: Option<FlatG2>,
}

fn construct_signature(
    tx_tree_root: Bytes32,
    pubkey_hash: Bytes32,
    account_id_hash: Bytes32,
    is_registration_block: bool,
    sender_with_signatures: &[SenderWithSignature],
) -> SignatureContent {
    assert_eq!(sender_with_signatures.len(), NUM_SENDERS_IN_BLOCK);
    let sender_flag_bits = sender_with_signatures
        .iter()
        .map(|s| s.signature.is_some())
        .collect::<Vec<_>>();
    let sender_flag = Bytes16::from_bits_be(&sender_flag_bits);
    let agg_pubkey = sender_with_signatures
        .iter()
        .map(|s| {
            let weight = hash_to_weight(s.sender, pubkey_hash);
            if s.signature.is_some() {
                let pubkey_g1: G1Affine = G1Affine::recover_from_x(s.sender.into());
                (pubkey_g1 * Fr::from(BigUint::from(weight))).into()
            } else {
                G1Affine::zero()
            }
        })
        .fold(G1Affine::zero(), |acc: G1Affine, x: G1Affine| {
            (acc + x).into()
        });
    let agg_signature = sender_with_signatures
        .iter()
        .map(|s| {
            if let Some(signature) = s.signature.clone() {
                signature.into()
            } else {
                G2Affine::zero()
            }
        })
        .fold(G2Affine::zero(), |acc: G2Affine, x: G2Affine| {
            (acc + x).into()
        });
    // message point
    let message_point = tx_tree_root_to_message_point(tx_tree_root);
    assert!(
        Bn254::pairing(agg_pubkey, message_point)
            == Bn254::pairing(G1Affine::generator(), agg_signature)
    );
    SignatureContent {
        tx_tree_root,
        is_registration_block,
        sender_flag,
        pubkey_hash,
        account_id_hash,
        agg_pubkey: agg_pubkey.into(),
        agg_signature: agg_signature.into(),
        message_point: message_point.into(),
    }
}
