#!/bin/bash

cargo install sqlx-cli

for dir in store-vault-server withdrawal-server validity-prover; do
    if [ -f "$dir/.env.example" ]; then
        cp "$dir/.env.example" "$dir/.env"
        echo "Copied .env.example to .env in $dir"
        
        cd "$dir"
        echo "Running sqlx database setup in $dir"
        sqlx database setup
        cd ..
    else
        echo "No .env.example found in $dir"
    fi
done

cargo sqlx prepare --workspace
