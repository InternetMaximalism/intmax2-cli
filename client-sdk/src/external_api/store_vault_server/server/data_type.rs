use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum EncryptedDataType {
    Deposit,
    Transfer,
    Transaction,
    Withdrawal,
}

impl Display for EncryptedDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptedDataType::Deposit => write!(f, "deposit"),
            EncryptedDataType::Transfer => write!(f, "transfer"),
            EncryptedDataType::Transaction => write!(f, "transaction"),
            EncryptedDataType::Withdrawal => write!(f, "withdrawal"),
        }
    }
}
