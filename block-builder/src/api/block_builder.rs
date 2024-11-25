use ark_bn254::{Bn254, Fr, G1Affine, G2Affine};
use ark_ec::{pairing::Pairing as _, AffineRepr as _};
use ethers::types::{Address, H256};
use hashbrown::HashMap;
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
            utils::get_pubkey_hash,
            SignatureContent,
        },
        trees::tx_tree::TxTree,
        tx::Tx,
    },
    constants::{NUM_SENDERS_IN_BLOCK, TX_TREE_HEIGHT},
    ethereum_types::{
        account_id_packed::AccountIdPacked, bytes16::Bytes16, bytes32::Bytes32, u256::U256,
        u32limb_trait::U32LimbTrait,
    },
};
use num::BigUint;
use plonky2_bn254::fields::recover::RecoverFromX as _;

use super::error::BlockBuilderError;

#[derive(Debug, Clone)]
pub struct BlockBuilder {
    validity_prover_client: ValidityProverClient,
    rollup_contract: RollupContract,
    block_builder_private_key: H256,
    eth_allowance_for_block: ethers::types::U256,

    status: BlockBuilderStatus,
    senders: HashMap<U256, usize>, // pubkey -> position in tx_requests
    tx_requests: Vec<(U256, Tx)>,
    memo: Option<ProposalMemo>,
    signatures: Vec<UserSignature>,
}

#[derive(Debug, Clone)]
struct ProposalMemo {
    tx_tree_root: Bytes32,
    pubkeys: Vec<U256>, // padded pubkeys
    pubkey_hash: Bytes32,
    proposals: Vec<BlockProposal>,
}

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
            status: BlockBuilderStatus::Pausing,
            senders: HashMap::new(),
            tx_requests: Vec::new(),
            memo: None,
            signatures: Vec::new(),
        }
    }

    pub fn get_status(&self) -> BlockBuilderStatus {
        self.status
    }

    fn is_request_contained(&self, pubkey: U256, tx: Tx) -> bool {
        let position = match self.senders.get(&pubkey) {
            Some(p) => *p,
            None => return false,
        };
        let (_, tx_req) = self.tx_requests[position];
        return tx_req == tx;
    }

    // Send a tx request by the user.
    pub async fn send_tx_request(&mut self, pubkey: U256, tx: Tx) -> Result<(), BlockBuilderError> {
        if !self.status.is_accepting_tx() {
            return Err(BlockBuilderError::NotAcceptingTx);
        }
        if self.tx_requests.len() >= NUM_SENDERS_IN_BLOCK {
            return Err(BlockBuilderError::BlockIsFull);
        }
        // duplication check
        if self.senders.contains_key(&pubkey) {
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
        if self.status == BlockBuilderStatus::AcceptingRegistrationTxs {
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
        self.senders.insert(pubkey, self.tx_requests.len());
        self.tx_requests.push((pubkey, tx));
        Ok(())
    }

    // Construct a block with the given tx requests by the block builder.
    pub fn construct_block(&mut self) -> Result<(), BlockBuilderError> {
        if !self.status.is_accepting_tx() {
            return Err(BlockBuilderError::NotAcceptingTx);
        }

        // sort and pad txs
        let mut sorted_txs = self.tx_requests.clone();
        sorted_txs.sort_by(|a, b| b.0.cmp(&a.0));
        sorted_txs.resize(NUM_SENDERS_IN_BLOCK, (U256::dummy_pubkey(), Tx::default()));

        let pubkeys = sorted_txs.iter().map(|tx| tx.0).collect::<Vec<_>>();
        let pubkey_hash = get_pubkey_hash(&pubkeys);

        let mut tx_tree = TxTree::new(TX_TREE_HEIGHT);
        for (_, tx) in sorted_txs.iter() {
            tx_tree.push(tx.clone());
        }
        let tx_tree_root: Bytes32 = tx_tree.get_root().into();

        let mut proposals = Vec::new();
        for (pubkey, _tx) in self.tx_requests.iter() {
            let tx_index = sorted_txs.iter().position(|(p, _)| p == pubkey).unwrap() as u32;
            let tx_merkle_proof = tx_tree.prove(tx_index as u64);
            proposals.push(BlockProposal {
                tx_tree_root,
                tx_index,
                tx_merkle_proof,
                pubkeys: pubkeys.clone(),
                pubkeys_hash: pubkey_hash,
            });
        }

        let memo = ProposalMemo {
            tx_tree_root,
            pubkeys,
            pubkey_hash,
            proposals,
        };
        match self.status {
            BlockBuilderStatus::AcceptingRegistrationTxs => {
                self.status = BlockBuilderStatus::ProposingRegistrationBlock;
            }
            BlockBuilderStatus::AcceptingNonRegistrationTxs => {
                self.status = BlockBuilderStatus::ProposingNonRegistrationBlock;
            }
            _ => unreachable!(),
        }
        self.memo = Some(memo);

        Ok(())
    }

    // Query the constructed proposal by the user.
    pub fn query_proposal(
        &self,
        pubkey: U256,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, BlockBuilderError> {
        match self.status {
            BlockBuilderStatus::Pausing => {
                return Err(BlockBuilderError::BlockBuilderIsPausing);
            }
            BlockBuilderStatus::AcceptingRegistrationTxs
            | BlockBuilderStatus::AcceptingNonRegistrationTxs => {
                if self.is_request_contained(pubkey, tx) {
                    // not constructed yet
                    return Ok(None);
                } else {
                    return Err(BlockBuilderError::TxRequestNotFound);
                }
            }
            BlockBuilderStatus::ProposingRegistrationBlock
            | BlockBuilderStatus::ProposingNonRegistrationBlock => {
                // continue
            }
        }
        let position = self.senders.get(&pubkey).unwrap(); // safe
        let proposal = &self.memo.as_ref().unwrap().proposals[*position];
        Ok(Some(proposal.clone()))
    }

    // Post the signature by the user.
    pub fn post_signature(
        &mut self,
        tx: Tx,
        signature: UserSignature,
    ) -> Result<(), BlockBuilderError> {
        if !self.status.is_proposing() {
            return Err(BlockBuilderError::NotProposing);
        }
        if self.is_request_contained(signature.pubkey, tx) {
            return Err(BlockBuilderError::TxRequestNotFound);
        }
        let memo = self.memo.as_ref().unwrap();
        signature
            .verify(memo.tx_tree_root, memo.pubkey_hash)
            .map_err(|e| BlockBuilderError::InvalidSignature(e.to_string()))?;
        self.signatures.push(signature);
        Ok(())
    }

    // Post the block with the given signatures.
    pub async fn post_block(&mut self) -> Result<(), BlockBuilderError> {
        let mut account_id_packed = None;
        let is_registration_block = match self.status {
            BlockBuilderStatus::ProposingRegistrationBlock => {
                for pubkey in self.memo.as_ref().unwrap().pubkeys.iter() {
                    if pubkey.is_dummy_pubkey() {
                        // ignore dummy pubkey
                        continue;
                    }
                    let account_info = self
                        .validity_prover_client
                        .get_account_info(*pubkey)
                        .await?;
                    if account_info.account_id.is_some() {
                        return Err(BlockBuilderError::AccountAlreadyRegistered(
                            *pubkey,
                            account_info.account_id.unwrap(),
                        ));
                    }
                }
                true
            }
            BlockBuilderStatus::ProposingNonRegistrationBlock => {
                let mut account_ids = Vec::new();
                for pubkey in self.memo.as_ref().unwrap().pubkeys.iter() {
                    let account_info = self
                        .validity_prover_client
                        .get_account_info(*pubkey)
                        .await?;
                    if account_info.account_id.is_none() {
                        return Err(BlockBuilderError::AccountNotFound(*pubkey));
                    }
                    account_ids.push(account_info.account_id.unwrap());
                }
                account_id_packed = Some(AccountIdPacked::pack(&account_ids));
                false
            }
            _ => {
                return Err(BlockBuilderError::NotProposing);
            }
        };

        let account_id_hash = account_id_packed.map_or(Bytes32::default(), |ids| ids.hash());
        let memo = self.memo.clone().unwrap();
        let mut sender_with_signatures = memo
            .pubkeys
            .iter()
            .map(|pubkey| SenderWithSignature {
                sender: *pubkey,
                signature: None,
            })
            .collect::<Vec<_>>();

        for signature in self.signatures.iter() {
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
        self.reset();
        Ok(())
    }

    pub fn start_registration_block(&mut self) -> Result<(), BlockBuilderError> {
        if self.status != BlockBuilderStatus::Pausing {
            return Err(BlockBuilderError::ShouldBePausing);
        }
        self.status = BlockBuilderStatus::AcceptingRegistrationTxs;
        Ok(())
    }

    pub fn start_non_registration_block(&mut self) -> Result<(), BlockBuilderError> {
        if self.status != BlockBuilderStatus::Pausing {
            return Err(BlockBuilderError::ShouldBePausing);
        }
        self.status = BlockBuilderStatus::AcceptingNonRegistrationTxs;
        Ok(())
    }

    pub async fn post_empty_block(&mut self) -> Result<(), BlockBuilderError> {
        self.start_non_registration_block()?;
        self.construct_block()?;
        self.post_block().await?;
        Ok(())
    }

    /// Reset the block builder.
    pub fn reset(&mut self) {
        *self = Self {
            validity_prover_client: self.validity_prover_client.clone(),
            rollup_contract: self.rollup_contract.clone(),
            block_builder_private_key: self.block_builder_private_key,
            eth_allowance_for_block: self.eth_allowance_for_block,
            status: BlockBuilderStatus::Pausing,
            senders: HashMap::new(),
            tx_requests: Vec::new(),
            memo: None,
            signatures: Vec::new(),
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

// #[cfg(test)]
// mod tests {
//     use super::BlockBuilder;
//     use intmax2_zkp::common::{signature::key_set::KeySet, tx::Tx};
//     use plonky2::{
//         field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig,
//     };

//     type F = GoldilocksField;
//     type C = PoseidonGoldilocksConfig;
//     const D: usize = 2;

//     #[test]
//     fn block_builder() {
//         let mut rng = rand::thread_rng();
//         let mut block_builder = BlockBuilder::new();
//         let mut validity_prover = BlockValidityProver::<F, C, D>::new();
//         let mut contract = MockContract::new();

//         let user = KeySet::rand(&mut rng);

//         for _ in 0..3 {
//             let tx = Tx::rand(&mut rng);

//             // send tx request
//             block_builder
//                 .send_tx_request(&validity_prover, user.pubkey, tx.clone())
//                 .unwrap();

//             // Block builder constructs a block
//             block_builder.construct_block().unwrap();

//             // query proposal and verify
//             let proposal = block_builder.query_proposal(user.pubkey).unwrap().unwrap();
//             proposal.verify(tx).unwrap(); // verify the proposal
//             let signature = proposal.sign(user);

//             // post signature
//             block_builder.post_signature(signature).unwrap();

//             // post block
//             block_builder
//                 .post_block(&mut contract, &validity_prover)
//                 .unwrap();

//             validity_prover.sync(&contract).unwrap();
//         }
//     }
// }
