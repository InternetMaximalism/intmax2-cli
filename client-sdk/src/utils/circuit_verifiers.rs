use std::path::{Path, PathBuf};

use intmax2_zkp::circuits::{
    balance::balance_processor::BalanceProcessor, validity::validity_processor::ValidityProcessor,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{circuit_data::VerifierCircuitData, config::PoseidonGoldilocksConfig},
    util::serialization::DefaultGateSerializer,
};

const VALIDITY_VD_BYTES: &[u8] =
    include_bytes!("../../circuit_data/validity_verifier_circuit_data.bin");
const BALANCE_VD_BYTES: &[u8] =
    include_bytes!("../../circuit_data/balance_verifier_circuit_data.bin");

fn circuit_data_path() -> PathBuf {
    PathBuf::from("circuit_data")
}

fn balance_circuit_data_path() -> PathBuf {
    circuit_data_path().join("balance_verifier_circuit_data.bin")
}

fn validity_circuit_data_path() -> PathBuf {
    circuit_data_path().join("validity_verifier_circuit_data.bin")
}

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct CircuitVerifiers {
    balance_vd: VerifierCircuitData<F, C, D>,
    validity_vd: VerifierCircuitData<F, C, D>,
}

impl CircuitVerifiers {
    // Construct the circuit verifiers from the processors.
    pub fn construct() -> Self {
        let validity_processor = ValidityProcessor::new();
        let balance_processor = BalanceProcessor::new(&validity_processor.get_verifier_data());
        Self {
            balance_vd: balance_processor.get_verifier_data(),
            validity_vd: validity_processor.validity_circuit.data.verifier_data(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        save_verifier_circuit_data(&balance_circuit_data_path(), &self.balance_vd)?;
        save_verifier_circuit_data(&validity_circuit_data_path(), &self.validity_vd)?;
        Ok(())
    }

    pub fn load() -> Self {
        let balance_vd = deserialize_verifier_circuit_data(BALANCE_VD_BYTES.to_vec()).unwrap();
        let validity_vd = deserialize_verifier_circuit_data(VALIDITY_VD_BYTES.to_vec()).unwrap();
        Self {
            balance_vd,
            validity_vd,
        }
    }

    pub fn get_balance_vd(&self) -> VerifierCircuitData<F, C, D> {
        self.balance_vd.clone()
    }

    pub fn get_validity_vd(&self) -> VerifierCircuitData<F, C, D> {
        self.validity_vd.clone()
    }
}

fn save_verifier_circuit_data(
    path: &Path,
    vd: &VerifierCircuitData<F, C, D>,
) -> anyhow::Result<()> {
    let gate_serializer = DefaultGateSerializer;
    let circuit_bytes = vd
        .to_bytes(&gate_serializer)
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut circuit_file = std::fs::File::create(path)?;
    std::io::Write::write_all(&mut circuit_file, &circuit_bytes)?;
    Ok(())
}

fn deserialize_verifier_circuit_data(
    data: Vec<u8>,
) -> anyhow::Result<VerifierCircuitData<F, C, D>> {
    let gate_serializer = DefaultGateSerializer;
    let vd =
        VerifierCircuitData::from_bytes(data, &gate_serializer).map_err(|e| anyhow::anyhow!(e))?;
    Ok(vd)
}

#[cfg(test)]
mod tests {

    #[test]
    #[ignore]
    fn test_save_circuit_verifiers() {
        let circuit_verifiers = super::CircuitVerifiers::construct();
        circuit_verifiers.save().unwrap();
    }

    #[test]
    fn test_load_circuit_verifiers() {
        let _circuit_verifiers = super::CircuitVerifiers::load();
    }
}
