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
cd store-vault-server
sqlx database setup
cargo run -r
```

2. Start balance-prover.
Example port: 9001
```bash
cd balance-prover
cargo run -r
```

3. Start validity-prover. 
Example port: 9002
```bash
cd validity-prover
sqlx database setup
cargo run -r
```

4. Start withdrawal-server.
Example port: 9003
```bash
cd withdrawal-server
cargo run -r
```

5. Start block-builder. 
Example port: 9004
```bash
cd block-builder
cargo run -r
```

