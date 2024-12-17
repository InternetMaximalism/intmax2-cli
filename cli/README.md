# Intmax2 CLI Tool

This CLI tool allows you to interact with the Intmax2 network. It includes functionalities such as:

- Generating keys (from scratch or from Ethereum private keys)
- Depositing assets (native tokens, ERC20, ERC721, ERC1155) into the rollup
- Transferring assets (single and batch transfers)
- Checking balances and transaction history
- Managing withdrawals (including syncing and claiming)

## Prerequisites

- Rust and Cargo installed
- Environment variables properly configured

Please copy the `.env.example` file to `.env` and adjust it as needed:

```bash
cp .env.example .env
```

Set your Alchemy API keys for `L1_RPC_URL` and `L2_RPC_URL` in the `.env` file.

## Update

To update the CLI tool to the latest version, simply pull the latest changes from the repository:

```bash
git pull
```

After pulling the latest changes, the tool will automatically use the updated version when you run any commands.

## Commands

You can see all commands and options by running:

```bash
cargo run -r -- --help
```

Available Commands:

- `generate-key`: Generate a new key pair
- `generate-from-eth-key`: Generate a key pair from an Ethereum private key
- `transfer`: Send a single transfer transaction
- `batch-transfer`: Process multiple transfers from a CSV file
- `deposit`: Deposit assets into the rollup
- `balance`: Check account balance
- `history`: View transaction history
- `withdrawal-status`: Check withdrawal status
- `claim-withdrawals`: Claim processed withdrawals
- `sync-withdrawals`: Synchronize withdrawal data

## Examples

### 1. Generate Keys

Generate a new key pair:
```bash
cargo run -r -- generate-key
```

Generate from Ethereum private key:
```bash
cargo run -r -- generate-from-eth-key --eth-private-key 0x...
```

### 2. Deposit Assets

Native token:
```bash
cargo run -r -- deposit \
  --eth-private-key 0x... \
  --private-key 0x... \
  --token-type NATIVE \
  --amount 100000000
```

ERC20 token:
```bash
cargo run -r -- deposit \
  --eth-private-key 0x... \
  --private-key 0x... \
  --token-type ERC20 \
  --amount 20000000 \
  --token-address 0x...
```

ERC721 token:
```bash
cargo run -r -- deposit \
  --eth-private-key 0x... \
  --private-key 0x... \
  --token-type ERC721 \
  --token-address 0x... \
  --token-id 0
```

ERC1155 token:
```bash
cargo run -r -- deposit \
  --eth-private-key 0x... \
  --private-key 0x... \
  --token-type ERC1155 \
  --amount 3 \
  --token-address 0x... \
  --token-id 0
```

### 3. Transfer Assets

Single transfer:
```bash
cargo run -r -- transfer \
  --private-key 0x... \
  --to 0x... \
  --amount 100 \
  --token-index 0
```

Batch transfer (using CSV):
```bash
cargo run -r -- batch-transfer \
  --private-key 0x... \
  --csv-path "transfers.csv"
```

Example CSV format (transfers.csv):
```csv
recipient,amount,tokenIndex
0x123...,100,1
0x456...,200,2
0x789...,300,3
```

Note: The batch transfer is limited to a maximum of 5 transfers per transaction. If you need to process more transfers, please split them into multiple CSV files or transactions.

### 4. Account Management

Check balance:
```bash
cargo run -r -- balance --private-key 0x...
```

View transaction history:
```bash
cargo run -r -- history --private-key 0x...
```

### 5. Withdrawal Management

Check withdrawal status:
```bash
cargo run -r -- withdrawal-status --private-key 0x...
```

Sync withdrawals:
```bash
cargo run -r -- sync-withdrawals --private-key 0x...
```

Claim withdrawals:
```bash
cargo run -r -- claim-withdrawals \
  --eth-private-key 0x... \
  --private-key 0x...
```

Note: For all commands that require private keys, ensure you're using the correct format (0x-prefixed hexadecimal).