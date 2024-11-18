use clap::{Parser, Subcommand};
use cli::{balance, deposit, get_base_url, sync, tx};
use ethers::types::H256;
use intmax2_core_sdk::utils::init_logger;
use intmax2_zkp::{
    common::signature::key_set::KeySet,
    ethereum_types::{u256::U256, u32limb_trait::U32LimbTrait as _},
};
use num_bigint::BigUint;

pub mod cli;
pub mod external_api;
pub mod state_manager;

#[derive(Parser)]
#[clap(name = "intmax2_cli")]
#[clap(about = "Intmax2 CLI tool")]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Tx {
        #[clap(long)]
        block_builder_url: String,
        #[clap(long)]
        private_key: H256,
        #[clap(long)]
        to: H256,
        #[clap(long)]
        amount: u128,
        #[clap(long)]
        token_index: u32,
    },
    Deposit {
        #[clap(long)]
        rpc_url: String,
        #[clap(long)]
        eth_private_key: H256,
        #[clap(long)]
        private_key: H256,
        #[clap(long)]
        amount: u128,
        #[clap(long)]
        token_index: u32,
    },
    Sync {
        #[clap(long)]
        private_key: H256,
    },
    Balance {
        #[clap(long)]
        private_key: H256,
    },
    PostEmptyAndSync,
    PostAndSync,
    GenerateKey,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger::init_logger();
    let args = Args::parse();

    match &args.command {
        Commands::Tx {
            block_builder_url,
            private_key,
            to,
            amount,
            token_index,
        } => {
            let to = h256_to_u256(*to);
            let amount = u128_to_u256(*amount);
            tx(block_builder_url, *private_key, to, amount, *token_index).await?;
        }
        Commands::Deposit {
            rpc_url,
            eth_private_key,
            private_key,
            amount,
            token_index,
        } => {
            let amount = u128_to_u256(*amount);
            let token_index = *token_index;
            deposit(rpc_url, *eth_private_key, *private_key, amount, token_index).await?;
        }
        Commands::Sync { private_key } => {
            sync(*private_key).await?;
        }
        Commands::Balance { private_key } => {
            balance(*private_key).await?;
        }
        Commands::GenerateKey => {
            println!("Generating key");
            let mut rng = rand::thread_rng();
            let key = KeySet::rand(&mut rng);
            let private_key = BigUint::from(key.privkey);
            let private_key: U256 = private_key.try_into().unwrap();
            println!("Private key: {}", private_key.to_hex());
            println!("Public key: {}", key.pubkey.to_hex());
        }
        Commands::PostEmptyAndSync => {
            state_manager::post_empty_block(&get_base_url()).await?;
            state_manager::sync_validity_proof(&get_base_url()).await?;
        }
        Commands::PostAndSync => {
            state_manager::post_block(&get_base_url()).await?;
            state_manager::sync_validity_proof(&get_base_url()).await?;
        }
    }

    Ok(())
}

fn u128_to_u256(u128: u128) -> intmax2_zkp::ethereum_types::u256::U256 {
    BigUint::from(u128).try_into().unwrap()
}

fn h256_to_u256(h256: H256) -> intmax2_zkp::ethereum_types::u256::U256 {
    intmax2_zkp::ethereum_types::u256::U256::from_bytes_be(h256.as_bytes())
}
