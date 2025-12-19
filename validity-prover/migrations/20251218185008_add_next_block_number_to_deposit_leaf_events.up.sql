ALTER TABLE deposit_leaf_events
    ADD COLUMN IF NOT EXISTS next_block_number INTEGER;
