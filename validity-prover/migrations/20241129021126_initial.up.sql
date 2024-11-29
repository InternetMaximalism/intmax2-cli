CREATE TABLE sync_state (
    id SERIAL PRIMARY KEY,
    sync_eth_block_number BIGINT
);

CREATE TABLE full_blocks (
    block_number INTEGER PRIMARY KEY,
    eth_block_number BIGINT NOT NULL,
    eth_tx_index BIGINT NOT NULL,
    full_block JSONB NOT NULL
);

CREATE TABLE deposit_leaf_events (
    deposit_index INTEGER PRIMARY KEY,
    deposit_hash BYTEA NOT NULL,
    eth_block_number BIGINT NOT NULL,
    eth_tx_index BIGINT NOT NULL
);

CREATE INDEX idx_deposit_leaf_events_deposit_hash ON deposit_leaf_events(deposit_hash);
CREATE INDEX idx_deposit_leaf_events_block_tx ON deposit_leaf_events(eth_block_number, eth_tx_index);
CREATE INDEX idx_full_blocks_block_tx ON full_blocks(eth_block_number, eth_tx_index);
