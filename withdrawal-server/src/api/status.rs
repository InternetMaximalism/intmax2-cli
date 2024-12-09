use intmax2_interfaces::api::withdrawal_server::interface::WithdrawalStatus;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "withdrawal_status")]
#[sqlx(rename_all = "snake_case")]
pub enum SqlWithdrawalStatus {
    Requested,
    Relayed,
    Success,
    NeedClaim,
    Failed,
}

impl From<WithdrawalStatus> for SqlWithdrawalStatus {
    fn from(status: WithdrawalStatus) -> Self {
        match status {
            WithdrawalStatus::Requested => SqlWithdrawalStatus::Requested,
            WithdrawalStatus::Relayed => SqlWithdrawalStatus::Relayed,
            WithdrawalStatus::Success => SqlWithdrawalStatus::Success,
            WithdrawalStatus::NeedClaim => SqlWithdrawalStatus::NeedClaim,
            WithdrawalStatus::Failed => SqlWithdrawalStatus::Failed,
        }
    }
}

impl Into<WithdrawalStatus> for SqlWithdrawalStatus {
    fn into(self) -> WithdrawalStatus {
        match self {
            SqlWithdrawalStatus::Requested => WithdrawalStatus::Requested,
            SqlWithdrawalStatus::Relayed => WithdrawalStatus::Relayed,
            SqlWithdrawalStatus::Success => WithdrawalStatus::Success,
            SqlWithdrawalStatus::NeedClaim => WithdrawalStatus::NeedClaim,
            SqlWithdrawalStatus::Failed => WithdrawalStatus::Failed,
        }
    }
}
