use ethers::types::H256;
use mining::{poet::witness::*, utils::h256_to_keyset};
use std::io::Write as _;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let private_key = H256::from_slice(&hex::decode(
        "01ae298b60235d81150162d4b97630f5f7fb39433c848522ed32aa6f7ec1564b",
    )?);
    let account = h256_to_keyset(private_key);
    let witness = PoetValue::generate(account).await?;

    let witness_json = serde_json::to_string(&witness)?;
    let dir_path = "data";
    let file_path = format!("{}/poet_witness.json", dir_path);
    std::fs::create_dir_all(dir_path)?;
    let mut file = std::fs::File::create(&file_path)?;
    file.write_all(witness_json.as_bytes())?;

    // let validity_proof_json = serde_json::to_string(&validity_proof)?;
    // let file_path = format!("{}/validity_proof.json", dir_path);
    // std::fs::create_dir_all(dir_path)?;
    // let mut file = std::fs::File::create(&file_path)?;
    // file.write_all(validity_proof_json.as_bytes())?;

    // let dir_path = "data";
    // let file_path = format!("{}/poet_witness.json", dir_path);
    // let witness_json = std::fs::read_to_string(&file_path)?;
    // let witness: PoetValue = serde_json::from_str(&witness_json)?;

    witness.prove_elapsed_time()?;

    Ok(())
}
