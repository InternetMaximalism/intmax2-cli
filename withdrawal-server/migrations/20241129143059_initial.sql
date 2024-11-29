CREATE TYPE withdrawal_status AS ENUM (
    'requested',
    'relayed',
    'success',
    'need_claim',
    'failed'
);

COMMENT ON TYPE withdrawal_status IS 'Represents the current status of a withdrawal request';
COMMENT ON TYPE withdrawal_status.'requested' IS 'The user has requested a withdrawal';
COMMENT ON TYPE withdrawal_status.'relayed' IS 'The withdrawal has been relayed to the contract';
COMMENT ON TYPE withdrawal_status.'success' IS 'The withdrawal has been successfully processed';
COMMENT ON TYPE withdrawal_status.'need_claim' IS 'The withdrawal has been processed but requires user claim';
COMMENT ON TYPE withdrawal_status.'failed' IS 'The withdrawal has failed due to system issues';

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
