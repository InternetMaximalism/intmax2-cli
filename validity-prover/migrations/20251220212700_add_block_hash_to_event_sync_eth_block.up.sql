ALTER TABLE event_sync_eth_block
    ADD COLUMN IF NOT EXISTS block_hash BYTEA;
