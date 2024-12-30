use ethers::types::H256;
use mining::{poet::witness::*, utils::h256_to_keyset};
use std::io::Write as _;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let private_key = H256::from_slice(&hex::decode(
        "18e722f9a8eeb7fa35880bec62f3ffdfbe7616c0423359ef2424d164d5cb6d98",
    )?);
    let account = h256_to_keyset(private_key);
    let witness = PoetWitness::generate(account).await?;

    let witness_json = serde_json::to_string(&witness)?;
    let dir_path = "data";
    let file_path = format!("{}/poet_witness.json", dir_path);
    std::fs::create_dir_all(dir_path)?;
    let mut file = std::fs::File::create(&file_path)?;
    file.write_all(witness_json.as_bytes())?;

    let witness_json = std::fs::read_to_string(&file_path)?;
    let witness: PoetWitness = serde_json::from_str(&witness_json)?;

    witness.prove_elapsed_time()?;

    Ok(())
}
