# Intmax2 CLI Tool

This CLI tool allows you to interact with the Intmax2 network. It includes functionalities such as:

- Generating keys
- Depositing assets (native tokens, ERC20, ERC721, ERC1155) into the rollup
- Checking balances and transaction history
- Sending rollup transactions
- Managing withdrawals (including syncing and claiming)


## Prerequisites

Rust and Cargo installed.

Please copy the `.env.example` file to `.env` and adjust it as needed:

```bash
cp .env.example .env
```

Set your Alchemy API keys for `L1_RPC_URL` and `L2_RPC_URL` in the `.env` file.

## Building the CLI

Run:

```bash
cargo build --release
```

This will produce a binary in `target/release`.

## Commands

You can see all commands and options by running:

```bash
cargo run -r -- --help
```

Available Commands:

- generate-key
- generate-from-eth-key
- deposit
- tx
- balance
- history
- withdrawal-status
- claim-withdrawals
- sync
- sync-withdrawals
- post-empty-block

Each command has its own flags and options. For example, `deposit` requires specifying `--eth-private-key` and `--private-key` along with token details.

## Examples

### 1. Generate a key pair:

```bash
cargo run -r -- generate-key
```

### 2. Deposit tokens:

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

### 3. Check your balance:

```bash
cargo run -r -- balance --private-key 0x...
```

If you see a "pending actions" error, please wait and retry.

### 4. Send a transaction:

```bash
cargo run -r -- tx \
  --private-key 0x... \
  --to 0x... \
  --amount 1 \
  --token-index 0
```

### 5. Check withdrawal status:

```bash
cargo run -r -- withdrawal-status --private-key 0x...
```

### 6. Claim withdrawals:

```bash
cargo run -r -- claim-withdrawals \
  --eth-private-key 0x... \
  --private-key 0x...
```

### 7. Check history:

```bash
cargo run -r -- history --private-key 0x...
```