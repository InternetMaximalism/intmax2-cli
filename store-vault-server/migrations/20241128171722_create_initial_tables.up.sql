CREATE TABLE encrypted_user_data (
    pubkey VARCHAR(66) PRIMARY KEY,
    encrypted_data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE balance_proofs (
    id SERIAL PRIMARY KEY,
    pubkey VARCHAR(66) NOT NULL,
    block_number INTEGER NOT NULL,
    private_commitment VARCHAR(66) NOT NULL,
    proof_data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(pubkey, block_number, private_commitment)
);

CREATE TABLE encrypted_data (
    id SERIAL PRIMARY KEY,
    data_type INTEGER NOT NULL,
    pubkey VARCHAR(66) NOT NULL,
    uuid TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    block_number INTEGER,
    encrypted_data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(uuid)
);

CREATE INDEX idx_balance_proofs_pubkey ON balance_proofs(pubkey);
CREATE INDEX idx_balance_proofs_block_number ON balance_proofs(block_number);
CREATE INDEX idx_encrypted_data_pubkey ON encrypted_data(pubkey);
CREATE INDEX idx_encrypted_data_timestamp ON encrypted_data(timestamp);
