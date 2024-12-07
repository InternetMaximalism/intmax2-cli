use anyhow::{bail, ensure};
use clap::{Parser, Subcommand};
use ethers::types::{Address as EthAddress, H256, U256 as EthU256};
use intmax2_cli::cli::{
    deposit::deposit,
    get::{balance, history, withdrawal_status},
    send::tx,
    sync::{sync, sync_withdrawals},
    utils::post_empty_block,
};
use intmax2_client_sdk::utils::init_logger::init_logger;
use intmax2_interfaces::data::deposit_data::TokenType;
use intmax2_zkp::{
    common::{generic_address::GenericAddress, signature::key_set::KeySet},
    ethereum_types::{
        address::Address as IAddress, u256::U256 as IU256, u32limb_trait::U32LimbTrait,
    },
};
use num_bigint::BigUint;

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
        eth_private_key: H256,
        #[clap(long)]
        private_key: H256,
        #[clap(long)]
        token_type: TokenType,
        #[clap(long)]
        amount: Option<u128>,
        #[clap(long)]
        token_address: Option<EthAddress>,
        #[clap(long)]
        token_id: Option<u128>,
    },
    Sync {
        #[clap(long)]
        private_key: H256,
    },
    PostEmptyBlock,
    SyncWithdrawals {
        #[clap(long)]
        private_key: H256,
    },
    Balance {
        #[clap(long)]
        private_key: H256,
    },
    History {
        #[clap(long)]
        private_key: H256,
    },
    WithdrawalStatus {
        #[clap(long)]
        private_key: H256,
    },
    GenerateKey,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    let args = Args::parse();

    dotenv::dotenv().ok();

    match args.command {
        Commands::Tx {
            private_key,
            to,
            amount,
            token_index,
        } => {
            let to = parse_generic_address(&to)?;
            let key = h256_to_keyset(private_key);
            tx(key, to, amount.into(), token_index).await?;
        }
        Commands::Deposit {
            eth_private_key,
            private_key,
            amount,
            token_type,
            token_address,
            token_id,
        } => {
            let key = h256_to_keyset(private_key);
            let amount = amount.map(|x| x.into());
            let token_id = token_id.map(|x| x.into());
            let (amount, token_address, token_id) =
                format_token_info(token_type, amount, token_address, token_id)?;
            deposit(
                key,
                eth_private_key,
                token_type,
                amount,
                token_address,
                token_id,
            )
            .await?;
        }
        Commands::Sync { private_key } => {
            let key = h256_to_keyset(private_key);
            sync(key).await?;
        }
        Commands::SyncWithdrawals { private_key } => {
            let key = h256_to_keyset(private_key);
            sync_withdrawals(key).await?;
        }
        Commands::PostEmptyBlock => {
            post_empty_block().await?;
        }
        Commands::Balance { private_key } => {
            let key = h256_to_keyset(private_key);
            balance(key).await?;
        }
        Commands::History { private_key } => {
            let key = h256_to_keyset(private_key);
            history(key).await?;
        }
        Commands::WithdrawalStatus { private_key } => {
            let key = h256_to_keyset(private_key);
            withdrawal_status(key).await?;
        }
        Commands::GenerateKey => {
            println!("Generating key");
            let mut rng = rand::thread_rng();
            let key = KeySet::rand(&mut rng);
            let private_key = BigUint::from(key.privkey);
            let private_key: IU256 = private_key.try_into().unwrap();
            println!("Private key: {}", private_key.to_hex());
            println!("Public key: {}", key.pubkey.to_hex());
        }
    }
    Ok(())
}

fn format_token_info(
    token_type: TokenType,
    amount: Option<EthU256>,
    token_address: Option<EthAddress>,
    token_id: Option<EthU256>,
) -> anyhow::Result<(EthU256, EthAddress, EthU256)> {
    match token_type {
        TokenType::NATIVE => Ok({
            let amount = amount.ok_or_else(|| anyhow::anyhow!("Missing amount"))?;
            (amount, EthAddress::zero(), EthU256::zero())
        }),
        TokenType::ERC20 => {
            let amount = amount.ok_or_else(|| anyhow::anyhow!("Missing amount"))?;
            let token_address =
                token_address.ok_or_else(|| anyhow::anyhow!("Missing token address"))?;
            Ok((amount, token_address, EthU256::zero()))
        }
        TokenType::ERC721 => {
            if amount.is_some() {
                anyhow::bail!("Amount should not be specified");
            }
            let token_address =
                token_address.ok_or_else(|| anyhow::anyhow!("Missing token address"))?;
            let token_id = token_id.ok_or_else(|| anyhow::anyhow!("Missing token id"))?;
            Ok((EthU256::one(), token_address, token_id))
        }
        TokenType::ERC1155 => {
            let amount = amount.ok_or_else(|| anyhow::anyhow!("Missing amount"))?;
            let token_address =
                token_address.ok_or_else(|| anyhow::anyhow!("Missing token address"))?;
            let token_id = token_id.ok_or_else(|| anyhow::anyhow!("Missing token id"))?;
            Ok((amount, token_address, token_id))
        }
    }
}

fn parse_generic_address(address: &str) -> anyhow::Result<GenericAddress> {
    ensure!(address.starts_with("0x"), "Invalid prefix");
    let bytes = hex::decode(&address[2..])?;
    if bytes.len() == 20 {
        let address = IAddress::from_bytes_be(&bytes);
        return Ok(GenericAddress::from_address(address));
    } else if bytes.len() == 32 {
        let pubkey = IU256::from_bytes_be(&bytes);
        return Ok(GenericAddress::from_pubkey(pubkey));
    } else {
        bail!("Invalid length");
    }
}

fn h256_to_keyset(h256: H256) -> KeySet {
    KeySet::new(BigUint::from_bytes_be(h256.as_bytes()).into())
}
