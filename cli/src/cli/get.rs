use chrono::DateTime;
use colored::{ColoredString, Colorize as _};
use intmax2_client_sdk::client::history::HistoryEntry;
use intmax2_interfaces::data::deposit_data::TokenType;
use intmax2_zkp::{
    common::{deposit::Deposit, signature::key_set::KeySet, trees::asset_tree::AssetLeaf},
    ethereum_types::u32limb_trait::U32LimbTrait,
    utils::leafable::Leafable as _,
};

use crate::cli::client::get_client;

use super::error::CliError;

pub async fn balance(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    let pending_info = client.sync(key.clone()).await?;

    println!("Pending deposits: {}", pending_info.pending_deposits.len());
    println!(
        "Pending transfers: {}",
        pending_info.pending_transfers.len()
    );

    let user_data = client.get_user_data(key).await?;
    let mut balances: Vec<(u32, AssetLeaf)> = user_data.balances().0.into_iter().collect();
    balances.sort_by_key(|(i, _leaf)| *i);

    println!("Balances:");
    for (i, leaf) in balances.iter() {
        let (token_type, address, token_id) =
            client.liquidity_contract.get_token_info(*i as u32).await?;
        println!("\t Token #{}:", i);
        println!("\t\t Amount: {}", leaf.amount);
        println!("\t\t Type: {}", token_type.to_string());

        match token_type {
            TokenType::NATIVE => {}
            TokenType::ERC20 => {
                println!("\t\t Address: {}", address);
            }
            TokenType::ERC721 => {
                println!("\t\t Address: {}", address);
                println!("\t\t Token ID: {}", token_id);
            }
            TokenType::ERC1155 => {
                println!("\t\t Address: {}", address);
                println!("\t\t Token ID: {}", token_id);
            }
        }
    }
    Ok(())
}

pub async fn withdrawal_status(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    let withdrawal_info = client.get_withdrawal_info(key).await?;
    println!("Withdrawal status:");
    for (i, withdrawal_info) in withdrawal_info.iter().enumerate() {
        let withdrawal = withdrawal_info.contract_withdrawal.clone();
        println!(
            "#{}: recipient: {}, token_index: {}, amount: {}, status: {}",
            i,
            withdrawal.recipient,
            withdrawal.token_index,
            withdrawal.amount,
            withdrawal_info.status
        );
    }
    Ok(())
}

pub async fn history(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    let history = client.fetch_history(key).await?;
    println!("History:");
    for entry in history {
        print_history_entry(&entry)?;
        println!();
    }
    Ok(())
}

fn get_status_string(is_included: bool, is_rejected: bool) -> ColoredString {
    match (is_included, is_rejected) {
        (true, false) => "Status: ✓ Included".bright_green(),
        (false, true) => "Status: ✗ Rejected".bright_red(),
        (false, false) => "Status: ⋯ Pending".yellow(),
        (true, true) => "Status: ! Invalid State".bright_red(),
    }
}

fn format_timestamp(timestamp: u64) -> String {
    let naive = DateTime::from_timestamp(timestamp as i64, 0).unwrap();
    naive.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn print_history_entry(entry: &HistoryEntry) -> Result<(), CliError> {
    match entry {
        HistoryEntry::Deposit {
            token_type,
            token_address,
            token_id,
            token_index,
            amount,
            pubkey_salt_hash,
            is_included,
            is_rejected,
            meta,
        } => {
            let status = get_status_string(*is_included, *is_rejected);
            let time = format_timestamp(meta.timestamp);
            let deposit_hash = token_index.map(|idx| {
                let deposit = Deposit {
                    pubkey_salt_hash: *pubkey_salt_hash,
                    token_index: idx,
                    amount: *amount,
                };
                deposit.hash()
            });

            println!(
                "{} [{}]",
                "DEPOSIT".bright_green().bold(),
                time.bright_blue(),
            );
            println!("  UUID: {}", meta.uuid);
            println!(
                "  Block: {}",
                meta.block_number
                    .map_or("N/A".to_string(), |b| b.to_string())
            );
            println!(
                "  Token: {} ({:?})",
                token_type.to_string().yellow(),
                token_type
            );
            println!("  Address: {}", token_address.to_string().cyan());
            println!("  ID: {}", token_id.to_string().white());
            println!(
                "  Index: {}",
                token_index
                    .map_or("N/A".to_string(), |idx| idx.to_string())
                    .white()
            );
            println!("  Amount: {}", amount.to_string().bright_green());
            println!(
                "  Deposit Hash: {}",
                deposit_hash.map_or("N/A".to_string(), |h| h.to_string())
            );
            println!("  {}", status);
        }
        HistoryEntry::Receive {
            amount,
            token_index,
            from,
            is_included,
            is_rejected,
            meta,
        } => {
            let status = get_status_string(*is_included, *is_rejected);
            let time = format_timestamp(meta.timestamp);

            println!(
                "{} [{}]",
                "RECEIVE".bright_purple().bold(),
                time.bright_blue(),
            );
            println!("  UUID: {}", meta.uuid);
            println!(
                "  Block: {}",
                meta.block_number
                    .map_or("N/A".to_string(), |b| b.to_string())
            );
            println!("  From: {}", from.to_hex().yellow());
            println!("  Token Index: {}", token_index.to_string().white());
            println!("  Amount: {}", amount.to_string().bright_green());
            println!("  {}", status);
        }
        HistoryEntry::Send {
            transfers,
            is_included,
            is_rejected,
            meta,
        } => {
            let status = get_status_string(*is_included, *is_rejected);
            let time = format_timestamp(meta.timestamp);

            println!("{} [{}]", "SEND".bright_red().bold(), time.bright_blue(),);
            println!("  UUID: {}", meta.uuid);
            println!(
                "  Block: {}",
                meta.block_number
                    .map_or("N/A".to_string(), |b| b.to_string())
            );
            println!("  Transfers:");
            for (i, t) in transfers.iter().enumerate() {
                println!("    {}: {}", i + 1, t.to_string().white());
            }
            println!("  {}", status);
        }
    }
    Ok(())
}
