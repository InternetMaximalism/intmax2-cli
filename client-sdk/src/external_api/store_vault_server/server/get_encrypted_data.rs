use intmax2_zkp::{ethereum_types::bytes32::Bytes32, mock::data::meta_data::MetaData};
use reqwest_wasm::Response;
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        encode::decode_base64,
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

use super::data_type::EncryptedDataType;

pub async fn get_encrypted_data(
    server_base_url: &str,
    data_type: EncryptedDataType,
    uuid: &str,
) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
    let url = format!("{}/{}/{}", server_base_url, data_type, uuid);

    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!("Failed to get encrypted data from server: {}", e))
        })?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let (meta, data) = deserialize_response(response, data_type).await?;
            Ok(Some((meta, data)))
        }
        ResponseType::NotFound(error) => {
            log::warn!(
                "Failed to get encrypted {} data: {}",
                data_type,
                error.message
            );
            Ok(None)
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
) -> Result<(MetaData, Vec<u8>), ServerError> {
    match data_type {
        EncryptedDataType::Deposit => {
            let response: ServerCommonResponse<GetDepositData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let (meta, data) = format_data(
                &response.data.deposit.uuid,
                &response.data.deposit.created_at,
                &response.data.deposit.encrypted_deposit_data,
            )?;
            Ok((meta, data))
        }
        EncryptedDataType::Transaction => {
            let response: ServerCommonResponse<GetTransactionData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let (meta, data) = format_data(
                &response.data.transaction.uuid,
                &response.data.transaction.created_at,
                &response.data.transaction.encrypted_transaction_data,
            )?;
            Ok((meta, data))
        }
        EncryptedDataType::Transfer => {
            let response: ServerCommonResponse<GetTransferData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let (meta, data) = format_data(
                &response.data.transfer.uuid,
                &response.data.transfer.created_at,
                &response.data.transfer.encrypted_transfer_data,
            )?;

            Ok((meta, data))
        }
        EncryptedDataType::Withdrawal => {
            let response: ServerCommonResponse<GetWithdrawalData> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Error while deserializing response: {}",
                        e
                    ))
                })?;
            let (meta, data) = format_data(
                &response.data.withdrawal.uuid,
                &response.data.withdrawal.created_at,
                &response.data.withdrawal.encrypted_withdrawal_data,
            )?;
            Ok((meta, data))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetDepositData {
    deposit: Deposit,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Deposit {
    uuid: String,
    recipient: Bytes32,
    encrypted_deposit_data: String,
    created_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTransactionData {
    transaction: Transaction,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    uuid: String,
    sender: Bytes32,
    encrypted_transaction_data: String,
    created_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTransferData {
    transfer: Transfer,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transfer {
    uuid: String,
    recipient: Bytes32,
    encrypted_transfer_data: String,
    created_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetWithdrawalData {
    withdrawal: Withdrawal,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Withdrawal {
    uuid: String,
    recipient: Bytes32,
    encrypted_withdrawal_data: String,
    created_at: String,
}

pub(super) fn format_data(
    uuid: &str,
    created_at: &str,
    data: &str,
) -> Result<(MetaData, Vec<u8>), ServerError> {
    let timestamp = chrono::DateTime::parse_from_rfc3339(created_at)
        .map_err(|e| {
            ServerError::DeserializationError(format!("Error while parsing timestamp: {}", e))
        })?
        .with_timezone(&chrono::Utc)
        .timestamp() as u64;
    let meta = MetaData {
        uuid: uuid.to_string(),
        timestamp,
        block_number: None,
    };
    let data = decode_base64(data).map_err(|e| {
        ServerError::DeserializationError(format!("Error while decoding data: {}", e))
    })?;
    Ok((meta, data))
}
