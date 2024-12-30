use intmax2_zkp::{
    circuits::validity::validity_pis::ValidityPublicInputsTarget,
    common::{salt::SaltTarget, trees::block_hash_tree::BlockHashMerkleProofTarget},
    ethereum_types::{
        address::{Address, AddressTarget},
        u256::{U256Target, U256},
        u32limb_trait::U32LimbTargetTrait,
    },
    utils::{
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
    history::{ReceivedDeposit, Withdrawal},
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

        Self {
            sender,
            // recipient,
            token_index,
            amount,
            salt,
        }
    }

    pub(crate) fn set_witness<F: Field>(
        &self,
        witness: &mut impl WitnessWrite<F>,
        value: &ReceivedDeposit,
    ) {
        self.sender.set_witness::<F, Address>(witness, value.sender);
        // self.recipient.set_witness::<F, U256>(witness, target.recipient);
        self.amount.set_witness::<F, U256>(witness, value.amount);
        self.salt.0.set_witness(witness, value.salt.0);
    }
}

#[derive(Debug, Clone)]
pub struct WithdrawalTarget {
    // pub sender: U256Target,
    pub recipient: AddressTarget,
    pub token_index: Target,
    pub amount: U256Target,
    pub salt: SaltTarget,
}

impl WithdrawalTarget {
    pub(crate) fn new<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        is_checked: bool,
    ) -> Self {
        // let sender = U256Target::new(builder, true);
        let recipient = AddressTarget::new(builder, is_checked);
        let token_index = builder.add_virtual_target();
        let amount = U256Target::new(builder, is_checked);
        let salt = SaltTarget::new(builder);

        Self {
            // sender,
            recipient,
            token_index,
            amount,
            salt,
        }
    }

    pub(crate) fn set_witness<F: Field>(
        &self,
        witness: &mut impl WitnessWrite<F>,
        value: &Withdrawal,
    ) {
        // self.sender.set_witness::<F, U256>(witness, target.sender);
        self.recipient
            .set_witness::<F, Address>(witness, value.recipient);
        self.amount.set_witness::<F, U256>(witness, value.amount);
        self.salt.0.set_witness(witness, value.salt.0);
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
    pub withdrawal_transfer: WithdrawalTarget,
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
        let withdrawal_transfer = WithdrawalTarget::new(builder, is_checked);
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
}

impl<const D: usize> PoetWithPlonky2ProofTarget<D> {
    pub(crate) fn new<F: RichField + Extendable<D>, C: GenericConfig<D, F = F> + 'static>(
        builder: &mut CircuitBuilder<F, D>,
        single_withdrawal_circuit_vd: &VerifierCircuitData<F, C, D>,
        is_checked: bool,
    ) -> Self
    where
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let proof_of_elapsed_time = PoetTarget::new(builder, is_checked);
        // let validity_proof = builder.add_virtual_proof_with_pis(validity_circuit_common_data);
        let single_withdrawal_proof =
            add_proof_target_and_verify(single_withdrawal_circuit_vd, builder);

        Self {
            proof_of_elapsed_time,
            single_withdrawal_proof,
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
    pub poet_target: PoetWithPlonky2ProofTarget<D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F> + 'static, const D: usize>
    PoetWithPlonky2ProofCircuit<F, C, D>
{
    pub fn new(single_withdrawal_circuit_vd: &VerifierCircuitData<F, C, D>) -> Self
    where
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let poet_target =
            PoetWithPlonky2ProofTarget::new(&mut builder, single_withdrawal_circuit_vd, true);

        let public_inputs = PoetPublicInputTarget {};
        builder.register_public_inputs(&public_inputs.to_vec());

        let data = builder.build();

        Self { data, poet_target }
    }

    pub fn prove(
        &self,
        poet_value: &PoetValue,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>>
    where
        <C as GenericConfig<D>>::Hasher: AlgebraicHasher<F>,
    {
        let mut witness = PartialWitness::new();
        self.poet_target
            .proof_of_elapsed_time
            .set_witness(&mut witness, poet_value);
        witness.set_proof_with_pis_target(
            &self.poet_target.single_withdrawal_proof,
            single_withdrawal_proof,
        );

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

#[cfg(test)]
mod tests {
    use intmax2_zkp::circuits::{
        balance::balance_processor::BalanceProcessor,
        validity::validity_processor::ValidityProcessor,
        withdrawal::single_withdrawal_circuit::SingleWithdrawalCircuit,
    };
    use plonky2::{
        field::goldilocks_field::GoldilocksField,
        plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
    };

    use crate::poet::witness::PoetValue;

    use super::PoetWithPlonky2ProofCircuit;

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
        let poet_with_plonky2_proof_circuit =
            PoetWithPlonky2ProofCircuit::<F, C, D>::new(&single_withdrawal_circuit_vd);

        let dir_path = "data";
        let file_path = format!("{}/poet_witness.json", dir_path);
        let witness_json = std::fs::read_to_string(&file_path).unwrap();
        let poet_value: PoetValue = serde_json::from_str(&witness_json).unwrap();

        let file_path = format!("{}/single_withdrawal_proof.json", dir_path);
        let single_withdrawal_proof_json = std::fs::read_to_string(&file_path).unwrap();
        let single_withdrawal_proof: ProofWithPublicInputs<F, C, D> =
            serde_json::from_str(&single_withdrawal_proof_json).unwrap();

        let proof = poet_with_plonky2_proof_circuit
            .prove(&poet_value, &single_withdrawal_proof)
            .unwrap();

        poet_with_plonky2_proof_circuit.verify(proof).unwrap();
    }
}
