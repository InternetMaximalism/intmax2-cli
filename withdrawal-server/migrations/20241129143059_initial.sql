CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE withdrawal_status AS ENUM (
    'requested',
    'relayed',
    'success',
    'need_claim',
    'failed'
);

CREATE TABLE withdrawals (
    id uuid NOT NULL DEFAULT uuid_generate_v4(),
    status withdrawal_status NOT NULL DEFAULT 'requested',
    pubkey CHAR(66) NOT NULL,
    recipient CHAR(42) NOT NULL,
    withdrawal_hash CHAR(66) NOT NULL,
    contract_withdrawal jsonb NOT NULL,
    single_withdrawal_proof bytea,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

CREATE INDEX idx_withdrawals_pubkey ON withdrawals(pubkey);
CREATE INDEX idx_withdrawals_recipient ON withdrawals(recipient);
