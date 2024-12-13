# Install dependency 
- Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- Install forge (if you launch local network)
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

- Install sqlx-cli 
```bash
cargo install sqlx-cli
```

- Install wasm-pack
```
cargo install wasm-pack
```

# Preparation 

Launch local network 
```bash
anvil  
```

Contract deployment
```bash
cargo test -r -p tests deploy_contracts -- --nocapture
```

Launch database
```bash
docker run --name postgres -e POSTGRES_PASSWORD=password -p 5432:5432 -d postgres
```

# Start server

1. Start Store-vault-server. 
Example port: 9000
```bash
cd store-vault-server && sqlx database setup && cargo run -r
```

2. Start balance-prover.
Example port: 9001
```bash
cd balance-prover && cargo run -r
```

3. Start validity-prover. 
Example port: 9002
```bash
cd validity-prover && sqlx database setup && cargo run -r
```

4. Start withdrawal-server.
Example port: 9003
```bash
cd withdrawal-server && sqlx database setup && cargo run -r
```

5. Start block-builder. 
Example port: 9004
```bash
cd block-builder && cargo run -r
```

## CLI 
Please refer to [the examples of cli ](cli/README.md#examples)

# Reset DB

```bash
(cd store-vault-server && sqlx database reset -y && cd ../validity-prover && sqlx database reset -y && cd ../withdrawal-server && sqlx database reset -y)
```