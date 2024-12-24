use intmax2_zkp::ethereum_types::u256::U256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeDBError {
    #[error("Failed to connect to database: {0}")]
    ConnectionError(#[from] sqlx::Error),

    #[error("Failed to serialize/deserialize data: {0}")]
    SerializationError(#[from] bincode::Error),
}

#[derive(Error, Debug)]
pub enum HistoricalMerkleTreeError {
    #[error("Node DB Error: {0}")]
    NodeDBError(#[from] NodeDBError),

    #[error("Invalid path length: {0}")]
    WrongPathLength(u32),

    #[error("Node not found for parent hash: {0}")]
    NodeNotFoundError(String),

    #[error("Leaf not found for hash: {0}")]
    LeafNotFoundError(String),

    #[error("Leaf hash mismatch: expected {expected}, got {got}")]
    LeafHashMismatch { expected: String, got: String },
}

#[derive(Error, Debug)]
pub enum HistoricalIndexedMerkleTreeError {
    #[error("Historical Merkle Tree Error: {0}")]
    HistoricalMerkleTreeError(#[from] HistoricalMerkleTreeError),

    #[error("Key does not exist: {0}")]
    KeyDoesNotExist(U256),

    #[error("Key already exists: {0}")]
    KeyAlreadyExists(U256),

    #[error("Too many candidates")]
    TooManyCandidates,
}
