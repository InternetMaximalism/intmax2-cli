use ethers::types::H256;
use mining::{poet::witness::*, utils::h256_to_keyset};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let private_key = H256::from_slice(&hex::decode(
        "18e722f9a8eeb7fa35880bec62f3ffdfbe7616c0423359ef2424d164d5cb6d98",
    )?);
    let account = h256_to_keyset(private_key);
    let witness = PoetWitness::generate(account).await?;
    witness.prove_elapsed_time()?;

    Ok(())
}
