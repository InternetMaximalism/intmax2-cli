
## Start server

1. Start Postgress

```bash
docker run --name postgres-store-vault -e POSTGRES_PASSWORD=password -e POSTGRES_DB=store_vault_server -p 5432:5432 -d postgres
```

2. Start server

```bash
cargo run -r
```