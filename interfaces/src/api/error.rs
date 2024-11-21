#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Unknown error: {0}")]
    UnknownError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Proof Decode error: {0}")]
    ProofDecodeError(String),

    #[error("Proof verification error: {0}")]
    ProofVerificationError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
