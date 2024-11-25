use std::sync::{Arc, RwLock};

use super::block_builder::BlockBuilder;

pub struct State {
    block_builder: Arc<RwLock<BlockBuilder>>,
}

impl State {}
