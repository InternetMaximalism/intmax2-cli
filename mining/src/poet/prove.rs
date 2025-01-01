use intmax2_zkp::{
    circuits::{
        balance::send::tx_inclusion_circuit::{TxInclusionCircuit, TxInclusionValue},
        validity::validity_pis::ValidityPublicInputsTarget,
    },
    common::{
        block::BlockTarget,
        generic_address::{GenericAddress, GenericAddressTarget},
        public_state::PublicState,
        salt::SaltTarget,
        transfer::TransferTarget,
        trees::block_hash_tree::BlockHashMerkleProofTarget,
        withdrawal::{get_withdrawal_nullifier_circuit, WithdrawalTarget},
        witness::{tx_witness::TxWitness, update_witness::UpdateWitness},
    },
    ethereum_types::{
        address::{Address, AddressTarget},
        u256::{U256Target, U256},
        u32limb_trait::U32LimbTargetTrait,
    },
    utils::{
        poseidon_hash_out::PoseidonHashOutTarget,
        recursively_verifiable::add_proof_target_and_verify,
        trees::indexed_merkle_tree::membership::MembershipProofTarget,
    },
};
use plonky2::{
    field::{extension::Extendable, types::Field},
    hash::hash_types::RichField,
    iop::{
        target::Target,
        witness::{PartialWitness, Witness, WitnessWrite},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, VerifierCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget},
    },
};

use super::{
    history::{ProcessedWithdrawal, ReceivedDeposit},
    validation::ValidationData,
    witness::PoetValue,
};

const ACCOUNT_TREE_HEIGHT: usize = 40;
const BLOCK_HASH_TREE_HEIGHT: usize = 32;

#[derive(Debug, Clone)]
pub struct ReceivedDepositTarget {
    pub sender: AddressTarget,
    // pub recipient: U256Target,
    pub token_index: Target,
    pub amount: U256Target,
    pub salt: SaltTarget,
    pub block_timestamp: Target, // UNIX timestamp seconds when transferring this token
}

impl ReceivedDepositTarget {
    pub(crate) fn new<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        is_checked: bool,
    ) -> Self {
        let sender = AddressTarget::new(builder, is_checked);
        // let recipient = U256Target::new(builder, true);
        let token_index = builder.add_virtual_target();
        let amount = U256Target::new(builder, is_checked);
        let salt = SaltTarget::new(builder);
        let block_timestamp = builder.add_virtual_target();

        Self {
            sender,
            // recipient,
            token_index,
            amount,
            salt,
            block_timestamp,
        }
    }

    pub(crate) fn set_witness<F: Field>(
        &self,
        witness: &mut impl WitnessWrite<F>,
        value: &ReceivedDeposit,
    ) {
        self.sender.set_witness::<F, Address>(witness, value.sender);
        // self.recipient.set_witness::<F, U256>(witness, target.recipient);
        let token_index = F::from_canonical_u64(value.token_index as u64);
        witness.set_target(self.token_index, token_index);
        self.amount.set_witness::<F, U256>(witness, value.amount);
        self.salt.0.set_witness(witness, value.salt.0);
        let block_timestamp = F::from_canonical_u64(value.block_timestamp);
        witness.set_target(self.block_timestamp, block_timestamp);
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedWithdrawalTarget {
    // pub sender: U256Target,
    pub recipient: GenericAddressTarget, // NOTE: AddressTarget
    pub token_index: Target,
    pub amount: U256Target,
    pub salt: SaltTarget,
    pub block_hash: PoseidonHashOutTarget, // INTMAX block
    pub block_timestamp: Target,           // UNIX timestamp seconds when transferring this token
}

impl ProcessedWithdrawalTarget {
    pub(crate) fn new<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        is_checked: bool,
    ) -> Self {
        // let sender = U256Target::new(builder, true);
        let hint_recipient = GenericAddressTarget::new(builder, is_checked);
        // let recipient = hint_recipient.to_address(builder);
        let token_index = builder.add_virtual_target();
        let amount = U256Target::new(builder, is_checked);
        let salt = SaltTarget::new(builder);
        let block_hash = PoseidonHashOutTarget::new(builder);
        let block_timestamp = builder.add_virtual_target();

        Self {
            // sender,
            recipient: hint_recipient,
            token_index,
            amount,
            salt,
            block_hash,
            block_timestamp,
        }
    }

    pub(crate) fn set_witness<F: Field>(
        &self,
        witness: &mut impl WitnessWrite<F>,
        value: &ProcessedWithdrawal,
    ) {
        // self.sender.set_witness::<F, U256>(witness, target.sender);
        let recipient = GenericAddress::from_address(value.recipient);
        self.recipient.set_witness::<F, _>(witness, recipient);
        let token_index = F::from_canonical_u64(value.token_index as u64);
        witness.set_target(self.token_index, token_index);
        self.amount.set_witness::<F, U256>(witness, value.amount);
        self.salt.0.set_witness(witness, value.salt.0);
        let block_timestamp = F::from_canonical_u64(value.block_timestamp);
        witness.set_target(self.block_timestamp, block_timestamp);
    }
}

#[derive(Debug, Clone)]
pub struct ValidationDataTarget {
    pub latest_validity_pis: ValidityPublicInputsTarget,
    pub deposit_validity_pis: ValidityPublicInputsTarget,
    pub deposit_block_merkle_proof: BlockHashMerkleProofTarget,
    pub withdrawal_validity_pis: ValidityPublicInputsTarget,
    pub withdrawal_block_merkle_proof: BlockHashMerkleProofTarget,
}

impl ValidationDataTarget {
    pub(crate) fn new<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        is_checked: bool,
    ) -> Self {
        let latest_validity_pis = ValidityPublicInputsTarget::new(builder, is_checked);
        let deposit_validity_pis = ValidityPublicInputsTarget::new(builder, is_checked);
        let withdrawal_validity_pis = ValidityPublicInputsTarget::new(builder, is_checked);
        let deposit_block_merkle_proof =
            BlockHashMerkleProofTarget::new(builder, BLOCK_HASH_TREE_HEIGHT);
        let withdrawal_block_merkle_proof =
            BlockHashMerkleProofTarget::new(builder, BLOCK_HASH_TREE_HEIGHT);

        Self {
            latest_validity_pis,
            deposit_validity_pis,
            deposit_block_merkle_proof,
            withdrawal_validity_pis,
            withdrawal_block_merkle_proof,
        }
    }

    pub(crate) fn set_witness<F: RichField, W: Witness<F>>(
        &self,
        witness: &mut W,
        value: &ValidationData,
    ) {
        self.latest_validity_pis
            .set_witness::<F, W>(witness, &value.latest_validity_pis);
        self.deposit_validity_pis
            .set_witness::<F, W>(witness, &value.deposit_validity_pis);
        self.deposit_block_merkle_proof
            .set_witness(witness, &value.deposit_block_merkle_proof);
        self.withdrawal_validity_pis
            .set_witness::<F, W>(witness, &value.withdrawal_validity_pis);
        self.withdrawal_block_merkle_proof
            .set_witness(witness, &value.withdrawal_block_merkle_proof);
    }
}

#[derive(Debug, Clone)]
pub struct PoetTarget {
    pub deposit_source: AddressTarget,
    pub intermediate: U256Target,
    pub withdrawal_destination: AddressTarget,
    pub proof_data: ValidationDataTarget,
    pub deposit_transfer: ReceivedDepositTarget,
    pub withdrawal_transfer: ProcessedWithdrawalTarget,
    pub deposit_block: BlockTarget,
    pub withdrawal_block: BlockTarget,
    pub account_membership_proof_just_before_withdrawal: MembershipProofTarget,
}

impl PoetTarget {
    pub(crate) fn new<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        is_checked: bool,
    ) -> Self {
        let deposit_source = AddressTarget::new(builder, is_checked);
        let intermediate = U256Target::new(builder, is_checked);
        let withdrawal_destination = AddressTarget::new(builder, is_checked);
        let proof_data = ValidationDataTarget::new(builder, is_checked);
        let deposit_transfer = ReceivedDepositTarget::new(builder, is_checked);
        let withdrawal_transfer = ProcessedWithdrawalTarget::new(builder, is_checked);
        let deposit_block = BlockTarget::new(builder, true);
        let withdrawal_block = BlockTarget::new(builder, true);
        let account_membership_proof_just_before_withdrawal =
            MembershipProofTarget::new(builder, ACCOUNT_TREE_HEIGHT, is_checked);

        // TODO: constrain

        Self {
            deposit_source,
            intermediate,
            withdrawal_destination,
            proof_data,
            deposit_transfer,
            withdrawal_transfer,
            deposit_block,
            withdrawal_block,
            account_membership_proof_just_before_withdrawal,
        }
    }

    pub fn set_witness<F: RichField, W: Witness<F>>(&self, witness: &mut W, value: &PoetValue) {
        self.deposit_source
            .set_witness::<F, Address>(witness, value.deposit_source);
        self.intermediate
            .set_witness::<F, U256>(witness, value.intermediate);
        self.withdrawal_destination
            .set_witness::<F, Address>(witness, value.withdrawal_destination);
        self.proof_data.set_witness(witness, &value.proof_data);
        self.deposit_transfer
            .set_witness(witness, &value.deposit_transfer);
        self.withdrawal_transfer
            .set_witness(witness, &value.withdrawal_transfer);
        self.deposit_block
            .set_witness(witness, &value.deposit_block);
        self.withdrawal_block
            .set_witness(witness, &value.withdrawal_block);
        self.account_membership_proof_just_before_withdrawal
            .set_witness(
                witness,
                &value.account_membership_proof_just_before_withdrawal,
            );
    }
}

#[derive(Debug, Clone)]
pub struct PoetPublicInput {}

impl PoetPublicInput {
    pub fn to_u32_vec(&self) -> Vec<u32> {
        // let result = [
        //     self.recipient.to_u32_vec(),
        //     vec![self.token_index],
        //     self.amount.to_u32_vec(),
        //     self.nullifier.to_u32_vec(),
        //     self.block_hash.to_u32_vec(),
        //     vec![self.block_number],
        // ]
        // .concat();
        // assert_eq!(result.len(), WITHDRAWAL_LEN);
        vec![]
    }

    pub fn from_u32_slice(_slice: &[u32]) -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct PoetPublicInputTarget {}

impl PoetPublicInputTarget {
    pub fn to_vec(&self) -> Vec<Target> {
        vec![]
    }

    pub fn from_slice(_slice: &[Target]) -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct PoetWithPlonky2ProofTarget<const D: usize> {
    pub proof_of_elapsed_time: PoetTarget,
    pub single_withdrawal_proof: ProofWithPublicInputsTarget<D>,
    pub tx_inclusion_proof: ProofWithPublicInputsTarget<D>,
}

impl<const D: usize> PoetWithPlonky2ProofTarget<D> {
    pub(crate) fn new<F: RichField + Extendable<D>, C: GenericConfig<D, F = F> + 'static>(
        builder: &mut CircuitBuilder<F, D>,
        single_withdrawal_circuit_vd: &VerifierCircuitData<F, C, D>,
        tx_inclusion_circuit_vd: &VerifierCircuitData<F, C, D>,
        is_checked: bool,
    ) -> Self
    where
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let proof_of_elapsed_time = PoetTarget::new(builder, is_checked);
        // let validity_proof = builder.add_virtual_proof_with_pis(validity_circuit_common_data);
        let single_withdrawal_proof =
            add_proof_target_and_verify(single_withdrawal_circuit_vd, builder);
        let tx_inclusion_proof = add_proof_target_and_verify(tx_inclusion_circuit_vd, builder);

        Self {
            proof_of_elapsed_time,
            single_withdrawal_proof,
            tx_inclusion_proof,
        }
    }
}

#[derive(Debug)]
pub struct PoetWithPlonky2ProofCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    const D: usize,
> {
    pub(crate) data: CircuitData<F, C, D>,
    pub target: PoetWithPlonky2ProofTarget<D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F> + 'static, const D: usize>
    PoetWithPlonky2ProofCircuit<F, C, D>
{
    pub fn new(
        single_withdrawal_circuit_vd: &VerifierCircuitData<F, C, D>,
        tx_inclusion_circuit_vd: &VerifierCircuitData<F, C, D>,
    ) -> Self
    where
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let _true = builder._true();

        let poet_target = PoetWithPlonky2ProofTarget::new(
            &mut builder,
            single_withdrawal_circuit_vd,
            tx_inclusion_circuit_vd,
            true,
        );

        let withdrawal =
            WithdrawalTarget::from_slice(&poet_target.single_withdrawal_proof.public_inputs);

        let withdrawal_transfer = &poet_target.proof_of_elapsed_time.withdrawal_transfer;
        let withdrawal_recipient = withdrawal_transfer.recipient.to_address(&mut builder);
        withdrawal_recipient.conditional_assert_eq(&mut builder, withdrawal.recipient, _true);
        let withdrawal_nullifier = get_withdrawal_nullifier_circuit(
            &mut builder,
            &TransferTarget {
                recipient: withdrawal_transfer.recipient,
                token_index: withdrawal_transfer.token_index,
                amount: withdrawal_transfer.amount.clone(),
                salt: withdrawal_transfer.salt.clone(),
            },
        );
        withdrawal
            .nullifier
            .conditional_assert_eq(&mut builder, withdrawal_nullifier, _true);

        builder.conditional_assert_eq(
            _true.target,
            withdrawal.block_number,
            poet_target
                .proof_of_elapsed_time
                .withdrawal_block
                .block_number,
        );

        let public_inputs = PoetPublicInputTarget {};
        builder.register_public_inputs(&public_inputs.to_vec());

        let data = builder.build();

        Self {
            data,
            target: poet_target,
        }
    }

    pub fn prove(
        &self,
        poet_value: &PoetValue,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
        tx_inclusion_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>>
    where
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let mut witness = PartialWitness::new();
        self.target
            .proof_of_elapsed_time
            .set_witness(&mut witness, poet_value);
        witness.set_proof_with_pis_target(
            &self.target.single_withdrawal_proof,
            single_withdrawal_proof,
        );
        witness.set_proof_with_pis_target(&self.target.tx_inclusion_proof, tx_inclusion_proof);

        let proof = self.data.prove(witness)?;

        Ok(proof)
    }

    pub fn verify(&self, proof: ProofWithPublicInputs<F, C, D>) -> anyhow::Result<()> {
        self.data.verify(proof)
    }

    pub fn circuit_data(&self) -> &CircuitData<F, C, D> {
        &self.data
    }
}

// prev_public_state: &PublicState,
// tx_witness: &TxWitness,
// update_witness: &UpdateWitness<F, C, D>,
pub fn prove_tx_inclusion_by_public_state<F, C, const D: usize>(
    tx_inclusion_circuit: &TxInclusionCircuit<F, C, D>,
    validity_vd: &VerifierCircuitData<F, C, D>,
    prev_public_state: &PublicState,
    tx_witness: &TxWitness,
    update_witness: &UpdateWitness<F, C, D>,
) -> anyhow::Result<ProofWithPublicInputs<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
{
    let update_validity_pis = update_witness.validity_pis();
    anyhow::ensure!(
        update_validity_pis == tx_witness.validity_pis,
        "validity proof pis mismatch"
    );
    let sender_tree = tx_witness.get_sender_tree();
    let sender_leaf = sender_tree.get_leaf(tx_witness.tx_index as u64);
    let sender_merkle_proof = sender_tree.prove(tx_witness.tx_index as u64);
    let tx_inclusion_value = TxInclusionValue::new(
        validity_vd,
        sender_leaf.sender,
        &prev_public_state,
        &update_witness.validity_proof,
        &update_witness.block_merkle_proof,
        &update_witness.prev_account_membership_proof()?,
        tx_witness.tx_index,
        &tx_witness.tx,
        &tx_witness.tx_merkle_proof,
        &sender_leaf,
        &sender_merkle_proof,
    )
    .map_err(|e| anyhow::anyhow!("failed to create tx inclusion value: {}", e))?;

    tx_inclusion_circuit
        .prove(&tx_inclusion_value)
        .map_err(|e| anyhow::anyhow!("failed to prove tx inclusion: {}", e))
}

#[cfg(test)]
mod tests {
    use intmax2_client_sdk::client::strategy::tx;
    use intmax2_zkp::{
        circuits::{
            balance::{
                balance_processor::BalanceProcessor,
                send::tx_inclusion_circuit::{self, TxInclusionCircuit},
            },
            validity::validity_processor::ValidityProcessor,
            withdrawal::single_withdrawal_circuit::SingleWithdrawalCircuit,
        },
        common::witness::update_witness::UpdateWitness,
    };
    use plonky2::{
        field::goldilocks_field::GoldilocksField,
        plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
    };

    use crate::poet::witness::PoetValue;

    use super::{prove_tx_inclusion_by_public_state, PoetWithPlonky2ProofCircuit};

    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    #[test]
    fn test_poet_with_plonky2_proof_circuit() {
        let validity_processor = ValidityProcessor::<F, C, D>::new();
        let validity_vd = validity_processor.get_verifier_data();
        let balance_processor = BalanceProcessor::new(&validity_vd);
        let balance_validity_vd = balance_processor.get_verifier_data();
        // let withdrawal_processor = WithdrawalProcessor::<F, C, D>::new(&balance_validity_vd.common);
        let single_withdrawal_circuit =
            SingleWithdrawalCircuit::<F, C, D>::new(&balance_validity_vd.common);
        let single_withdrawal_circuit_vd = single_withdrawal_circuit.data.verifier_data();
        let tx_inclusion_circuit = balance_processor
            .balance_transition_processor
            .sender_processor
            .tx_inclusion_circuit;
        let tx_inclusion_circuit_vd = tx_inclusion_circuit.data.verifier_data();
        let poet_with_plonky2_proof_circuit = PoetWithPlonky2ProofCircuit::<F, C, D>::new(
            &single_withdrawal_circuit_vd,
            &tx_inclusion_circuit_vd,
        );

        let dir_path = "data";
        let file_path = format!("{}/poet_witness.json", dir_path);
        let witness_json = std::fs::read_to_string(&file_path).unwrap();
        let poet_value: PoetValue = serde_json::from_str(&witness_json).unwrap();

        let file_path = format!("{}/single_withdrawal_proof.json", dir_path);
        let single_withdrawal_proof_json = std::fs::read_to_string(&file_path).unwrap();
        let single_withdrawal_proof: ProofWithPublicInputs<F, C, D> =
            serde_json::from_str(&single_withdrawal_proof_json).unwrap();

        // let withdrawal_block_public_state =
        //     &poet_value.proof_data.withdrawal_validity_pis.public_state;

        // let tx_inclusion_proof = prove_tx_inclusion_by_public_state(
        //     &tx_inclusion_circuit,
        //     &validity_vd,
        //     &withdrawal_block_public_state,
        //     &poet_value.withdrawal_tx_witness,
        //     &poet_value.withdrawal_update_witness,
        // )
        // .unwrap();

        // let file_path = format!("{}/tx_inclusion_proof.json", dir_path);
        // let tx_inclusion_proof_json = serde_json::to_string(&tx_inclusion_proof).unwrap();
        // std::fs::write(&file_path, tx_inclusion_proof_json).unwrap();

        let file_path = format!("{}/tx_inclusion_proof.json", dir_path);
        let tx_inclusion_proof_json = std::fs::read_to_string(&file_path).unwrap();
        let tx_inclusion_proof: ProofWithPublicInputs<F, C, D> =
            serde_json::from_str(&tx_inclusion_proof_json).unwrap();

        let proof = poet_with_plonky2_proof_circuit
            .prove(&poet_value, &single_withdrawal_proof, &tx_inclusion_proof)
            .unwrap();

        poet_with_plonky2_proof_circuit.verify(proof).unwrap();
    }
}
