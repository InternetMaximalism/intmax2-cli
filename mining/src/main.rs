use mining::poet::witness::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let witness = generate_witness_of_elapsed_time().await?;
    prove_elapsed_time(witness)?;

    Ok(())
}
