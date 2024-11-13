use clap::{Parser, Subcommand};
use cli::{balance, deposit, sync, tx};
use ethers::types::{H256, U256};
use intmax2_core_sdk::utils::init_logger;
use intmax2_zkp::{
    common::signature::key_set::KeySet,
    ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait as _},
};
use num_bigint::BigUint;

pub mod cli;

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
        amount: U256,
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
        amount: U256,
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
            let amount = u256_convert(*amount);
            tx(block_builder_url, *private_key, to, amount, *token_index).await?;
        }
        Commands::Deposit {
            rpc_url,
            eth_private_key,
            private_key,
            amount,
            token_index,
        } => {
            let amount = u256_convert(*amount);
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
            let private_key_hex = format!("{:x}", private_key);
            println!("Private key: 0x{}", private_key_hex);
            println!("Public key: {}", key.pubkey.to_hex());
        }
    }

    Ok(())
}

fn u256_to_bytes32(u256: U256) -> Bytes32 {
    let mut bytes = [0u8; 32];
    u256.to_big_endian(&mut bytes);
    Bytes32::from_bytes_be(&bytes)
}

fn u256_convert(u256: U256) -> intmax2_zkp::ethereum_types::u256::U256 {
    u256_to_bytes32(u256).into()
}

fn h256_to_u256(h256: H256) -> intmax2_zkp::ethereum_types::u256::U256 {
    intmax2_zkp::ethereum_types::u256::U256::from_bytes_be(h256.as_bytes())
}
