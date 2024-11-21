use reqwest_wasm::{Response, StatusCode};

use crate::external_api::common::{
    error::ServerError,
    response::{ErrorDetail, ServerErrorResponse},
};

#[derive(Debug)]
pub enum ResponseType {
    Success(Response),
    NotFound(ErrorDetail),
    ServerError(ErrorDetail),
    UnknownError(String),
}

pub async fn handle_response(response: Response) -> Result<ResponseType, ServerError> {
    match response.status() {
        StatusCode::OK => Ok(ResponseType::Success(response)),
        StatusCode::NOT_FOUND => {
            let error = deserialize_error_detail(response).await?;
            Ok(ResponseType::NotFound(error))
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            let error = deserialize_error_detail(response).await?;
            Ok(ResponseType::ServerError(error))
        }
        _ => {
            let error = response.text().await.map_err(|e| {
                ServerError::DeserializationError(format!(
                    "Error while deserializing unknown response {}",
                    e
                ))
            })?;
            Ok(ResponseType::UnknownError(error))
        }
    }
}

async fn deserialize_error_detail(response: Response) -> Result<ErrorDetail, ServerError> {
    let response = response
        .json::<ServerErrorResponse>()
        .await
        .map_err(|e| ServerError::DeserializationError(format!("{}", e)))?;
    Ok(response.error)
}
