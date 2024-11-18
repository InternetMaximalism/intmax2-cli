use anyhow::{bail, ensure};
use clap::{Parser, Subcommand};
use cli::{balance, deposit, get_base_url, sync, sync_withdrawals, tx};
use ethers::types::H256;
use intmax2_core_sdk::utils::init_logger;
use intmax2_zkp::{
    common::{generic_address::GenericAddress, signature::key_set::KeySet},
    ethereum_types::{address::Address, u256::U256, u32limb_trait::U32LimbTrait as _},
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
        private_key: H256,
        #[clap(long)]
        to: String,
        #[clap(long)]
        amount: u128,
        #[clap(long)]
        token_index: u32,
    },
    Deposit {
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
    SyncWithdrawals {
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
            private_key,
            to,
            amount,
            token_index,
        } => {
            let to = parse_generic_address(to)?;
            let amount = u128_to_u256(*amount);
            let key = h256_to_keyset(*private_key);
            tx(key, to, amount, *token_index).await?;
        }
        Commands::Deposit {
            private_key,
            amount,
            token_index,
        } => {
            let amount = u128_to_u256(*amount);
            let token_index = *token_index;
            let key = h256_to_keyset(*private_key);
            deposit(key, amount, token_index).await?;
        }
        Commands::Sync { private_key } => {
            let key = h256_to_keyset(*private_key);
            sync(key).await?;
        }
        Commands::SyncWithdrawals { private_key } => {
            let key = h256_to_keyset(*private_key);
            sync_withdrawals(key).await?;
        }
        Commands::Balance { private_key } => {
            let key = h256_to_keyset(*private_key);
            balance(key).await?;
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

fn parse_generic_address(address: &str) -> anyhow::Result<GenericAddress> {
    ensure!(address.starts_with("0x"), "Invalid prefix");
    let bytes = hex::decode(&address[2..])?;
    if bytes.len() == 20 {
        let address = Address::from_bytes_be(&bytes);
        return Ok(GenericAddress::from_address(address));
    } else if bytes.len() == 32 {
        let pubkey = U256::from_bytes_be(&bytes);
        return Ok(GenericAddress::from_pubkey(pubkey));
    } else {
        bail!("Invalid length");
    }
}

fn u128_to_u256(u128: u128) -> intmax2_zkp::ethereum_types::u256::U256 {
    BigUint::from(u128).try_into().unwrap()
}

fn h256_to_keyset(h256: H256) -> KeySet {
    KeySet::new(BigUint::from_bytes_be(h256.as_bytes()).into())
}
