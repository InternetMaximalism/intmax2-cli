use base64::prelude::*;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::circuit_data::VerifierCircuitData;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::CompressedProofWithPublicInputs;
use plonky2::plonk::proof::ProofWithPublicInputs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;

pub fn decode_base64(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_STANDARD.decode(encoded)
}

pub fn encode_base64(data: &[u8]) -> String {
    BASE64_STANDARD.encode(data)
}

pub fn encode_plonky2_proof(
    proof: ProofWithPublicInputs<F, C, D>,
    circuit_data: &VerifierCircuitData<F, C, D>,
) -> String {
    let compressed_proof = proof
        .compress(
            &circuit_data.verifier_only.circuit_digest,
            &circuit_data.common,
        )
        .expect("Failed to compress proof");

    BASE64_STANDARD.encode(&compressed_proof.to_bytes())
}

pub fn decode_plonky2_proof(
    encoded_proof: &str,
    circuit_data: &VerifierCircuitData<F, C, D>,
) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
    let decoded_proof = BASE64_STANDARD.decode(&encoded_proof)?;
    let compressed_proof =
        CompressedProofWithPublicInputs::from_bytes(decoded_proof, &circuit_data.common)?;

    compressed_proof.decompress(
        &circuit_data.verifier_only.circuit_digest,
        &circuit_data.common,
    )
}
