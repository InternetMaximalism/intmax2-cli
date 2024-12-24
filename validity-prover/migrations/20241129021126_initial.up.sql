-- Observer tables
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

-- Validity prover tables
CREATE TABLE validity_state (
    id SERIAL PRIMARY KEY,
    last_block_number INTEGER NOT NULL
);

CREATE TABLE validity_proofs (
    block_number INTEGER PRIMARY KEY,
    proof JSONB NOT NULL
);

CREATE TABLE account_trees (
    block_number INTEGER PRIMARY KEY,
    tree_data JSONB NOT NULL
);

CREATE TABLE block_hash_trees (
    block_number INTEGER PRIMARY KEY,
    tree_data JSONB NOT NULL
);

CREATE TABLE deposit_hash_trees (
    block_number INTEGER PRIMARY KEY,
    tree_data JSONB NOT NULL
);

CREATE TABLE tx_tree_roots (
    tx_tree_root BYTEA PRIMARY KEY,
    block_number INTEGER NOT NULL
);

CREATE TABLE sender_leaves (
    block_number INTEGER PRIMARY KEY,
    leaves JSONB NOT NULL
);


--- Merkle tree tables
CREATE TABLE IF NOT EXISTS current_node_hashes (
    tag int NOT NULL,
    bit_path bytea NOT NULL,
    hash_value bytea NOT NULL,
    PRIMARY KEY (tag, bit_path)
);

CREATE TABLE IF NOT EXISTS hash_nodes (
    tag int NOT NULL,
    parent_hash bytea NOT NULL,
    left_hash  bytea NOT NULL,
    right_hash bytea NOT NULL,
    PRIMARY KEY (tag, parent_hash)
);

CREATE TABLE IF NOT EXISTS current_leaf_hashes (
    tag int NOT NULL,
    position bigint NOT NULL,
    leaf_hash bytea NOT NULL,
    PRIMARY KEY (tag, position)
);

CREATE TABLE IF NOT EXISTS leaves (
    tag int NOT NULL,
    leaf_hash bytea NOT NULL,
    leaf bytea NOT NULL,
    PRIMARY KEY (tag, leaf_hash)
);

CREATE TABLE IF NOT EXISTS root_history (
    tag int NOT NULL,
    i int NOT NULL,
    root bytea NOT NULL,
    PRIMARY KEY (tag, i)
);


CREATE INDEX idx_deposit_leaf_events_deposit_hash ON deposit_leaf_events(deposit_hash);
CREATE INDEX idx_deposit_leaf_events_block_tx ON deposit_leaf_events(eth_block_number, eth_tx_index);
CREATE INDEX idx_full_blocks_block_tx ON full_blocks(eth_block_number, eth_tx_index);
