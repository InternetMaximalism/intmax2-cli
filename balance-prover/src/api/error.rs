#[derive(Debug, thiserror::Error)]
pub enum BalanceProverError {
    #[error("ProveSpentError: {0}")]
    ProveSpentError(String),
    #[error("ProveSendError: {0}")]
    ProveSendError(String),
    #[error("ProveReceiveDepositError: {0}")]
    ProveReceiveDepositError(String),
    #[error("ProveReceiveTransferError: {0}")]
    ProveReceiveTransferError(String),
    #[error("ProveUpdateError: {0}")]
    ProveUpdateError(String),
    #[error("ProveSingleWithdrawalError: {0}")]
    ProveSingleWithdrawalError(String),
}
