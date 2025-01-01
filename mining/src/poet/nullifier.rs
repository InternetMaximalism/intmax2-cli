use intmax2_zkp::{
    common::{
        generic_address::{GenericAddress, GenericAddressTarget},
        salt::{Salt, SaltTarget},
        transfer::{Transfer, TransferTarget},
        withdrawal::{get_withdrawal_nullifier, get_withdrawal_nullifier_circuit},
    },
    ethereum_types::{
        address::{Address, AddressTarget},
        bytes32::{Bytes32, Bytes32Target},
        u256::{U256Target, U256, U256_LEN},
        u32limb_trait::U32LimbTargetTrait,
    },
};
use plonky2::{
    field::extension::Extendable, hash::hash_types::RichField, iop::target::Target,
    plonk::circuit_builder::CircuitBuilder,
};

pub fn get_common_nullifier(
    eth_address: Address,
    token_index: u32,
    amount: U256,
    salt: Salt,
) -> Bytes32 {
    let transfer = Transfer {
        recipient: GenericAddress::from_address(eth_address),
        token_index,
        amount,
        salt,
    };
    get_withdrawal_nullifier(&transfer)
}

pub fn get_common_nullifier_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    eth_address: AddressTarget,
    token_index: Target,
    amount: U256Target,
    salt: SaltTarget,
) -> Bytes32Target {
    let generic_address = generic_address_from_address(builder, eth_address);
    let transfer = TransferTarget {
        recipient: generic_address,
        token_index,
        amount,
        salt,
    };
    get_withdrawal_nullifier_circuit(builder, &transfer)
}

pub fn generic_address_from_address<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    address: AddressTarget,
) -> GenericAddressTarget {
    let zero = builder.constant(F::default());
    let mut limbs = address.to_vec();
    limbs.resize(U256_LEN, zero);
    let _false = builder._false();

    GenericAddressTarget {
        is_pubkey: _false,
        data: U256Target::from_slice(&limbs),
    }
}
