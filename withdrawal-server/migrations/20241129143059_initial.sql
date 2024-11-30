CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE withdrawal_status AS ENUM (
    'requested',
    'relayed',
    'success',
    'need_claim',
    'failed'
);

CREATE TABLE withdrawal (
    id uuid NOT NULL DEFAULT uuid_generate_v4(),
    status withdrawal_status NOT NULL DEFAULT 'requested',
    pubkey CHAR(66) NOT NULL,
    recipient CHAR(42) NOT NULL,
    single_withdrawal_proof bytea,
    chained_withdrawal jsonb NOT NULL,
    withdrawal_id int,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

CREATE INDEX idx_withdrawal_pubkey ON withdrawal(pubkey);
CREATE INDEX idx_withdrawal_recipient ON withdrawal(recipient);
