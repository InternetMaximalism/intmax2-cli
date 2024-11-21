use intmax2_zkp::{ethereum_types::bytes32::Bytes32, mock::data::meta_data::MetaData};
use reqwest_wasm::Response;
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::{
        error::ServerError,
        pagination::{PaginationRequest, PaginationResponse},
        response::ServerCommonResponse,
    },
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

use super::{data_type::EncryptedDataType, get_encrypted_data::format_data};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetEncryptedDataAllRequest {
    pub pagination: PaginationRequest,
    pub sender: Bytes32,
    pub sorting: String,
    pub order_by: String,
}

pub async fn get_encrypted_data_all(
    server_base_url: &str,
    data_type: EncryptedDataType,
    pubkey: Bytes32,
    timestamp: u64,
) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
    let url = format!("{}/{}s/list", server_base_url, data_type);

    let timestamp_nano = timestamp * 1000_000; // convert to nano seconds
    let request = GetEncryptedDataAllRequest {
        pagination: PaginationRequest::from_sorting_value(&timestamp_nano.to_string()),
        sender: pubkey,
        sorting: "desc".to_string(),
        order_by: "date_create".to_string(),
    };
    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
    })
    .await
    .map_err(|e| {
        ServerError::NetworkError(format!("Failed to get encrypted data from server: {}", e))
    })?;

    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let result = deserialize_response(response, data_type).await?;
            Ok(result)
        }
        ResponseType::NotFound(error) => {
            log::warn!(
                "Failed to get encrypted {} data: {}",
                data_type,
                error.message
            );
            Ok(vec![])
        }
        ResponseType::ServerError(error) => {
            log::error!("Failed to get encrypted data: {}", error.message);
            Err(ServerError::ServerError(error.message))
        }
        ResponseType::UnknownError(error) => {
            log::error!("Failed to get encrypted data: {}", error);
            Err(ServerError::UnknownError(error))
        }
    }
}

async fn deserialize_response(
    response: Response,
    data_type: EncryptedDataType,
) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
    match data_type {
        EncryptedDataType::Deposit => {
            let response: ServerCommonResponse<GetDepositData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let mut result = Vec::new();
            for deposit in response.data.deposits {
                let (meta, data) = format_data(
                    &deposit.uuid,
                    &deposit.created_at,
                    &deposit.encrypted_deposit_data,
                )?;
                result.push((meta, data));
            }
            Ok(result)
        }
        EncryptedDataType::Transaction => {
            let response: ServerCommonResponse<GetTransactionData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let mut result = Vec::new();
            for transaction in response.data.transactions {
                let (meta, data) = format_data(
                    &transaction.uuid,
                    &transaction.created_at,
                    &transaction.encrypted_transaction_data,
                )?;
                result.push((meta, data));
            }
            Ok(result)
        }
        EncryptedDataType::Transfer => {
            let response: ServerCommonResponse<GetTransferData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let mut result = Vec::new();
            for transfer in response.data.transfers {
                let (meta, data) = format_data(
                    &transfer.uuid,
                    &transfer.created_at,
                    &transfer.encrypted_transfer_data,
                )?;
                result.push((meta, data));
            }
            Ok(result)
        }
        EncryptedDataType::Withdrawal => {
            let response: ServerCommonResponse<GetWithdrawalData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let mut result = Vec::new();
            for withdrawal in response.data.withdrawals {
                let (meta, data) = format_data(
                    &withdrawal.uuid,
                    &withdrawal.created_at,
                    &withdrawal.encrypted_withdrawal_data,
                )?;
                result.push((meta, data));
            }
            Ok(result)
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetDepositData {
    pagination: PaginationResponse,
    deposits: Vec<Deposit>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Deposit {
    uuid: String,
    sender: Bytes32,
    signature: String,
    encrypted_deposit_data: String,
    created_at: String,
    sorting_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTransactionData {
    pagination: PaginationResponse,
    transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    uuid: String,
    sender: Bytes32,
    encrypted_transaction_data: String,
    created_at: String,
    sorting_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTransferData {
    pagination: PaginationResponse,
    transfers: Vec<Transfer>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transfer {
    uuid: String,
    recipient: Bytes32,
    encrypted_transfer_data: String,
    created_at: String,
    sorting_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetWithdrawalData {
    pagination: PaginationResponse,
    withdrawals: Vec<Withdrawal>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Withdrawal {
    uuid: String,
    recipient: Bytes32,
    encrypted_withdrawal_data: String,
    created_at: String,
    sorting_value: String,
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait as _};

    use crate::{
        external_api::store_vault_server::server::data_type::EncryptedDataType,
        utils::init_logger::init_logger,
    };

    #[tokio::test]
    async fn test_get_encrypted_data_all() -> anyhow::Result<()> {
        init_logger();

        let mut rng = rand::thread_rng();

        let server_base_url = "http://localhost:4000/v1/backups";
        let pubkey = Bytes32::rand(&mut rng);
        let timestamp = 0;

        let data_type = EncryptedDataType::Deposit;
        let _result =
            super::get_encrypted_data_all(server_base_url, data_type, pubkey, timestamp).await?;

        Ok(())
    }
}
