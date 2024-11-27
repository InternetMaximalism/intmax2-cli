use intmax2_interfaces::api::error::ServerError;
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::retry::with_retry;

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    #[serde(default)]
    message: Option<String>,
}

pub async fn post_request<B: Serialize, R: DeserializeOwned>(
    base_url: &str,
    endpoint: &str,
    body: &B,
) -> Result<R, ServerError> {
    let url = format!("{}{}", base_url, endpoint);
    log::info!(
        "Posting url={} with body={}",
        url,
        serde_json::to_string(body).unwrap()
    );
    let response =
        with_retry(|| async { reqwest::Client::new().post(&url).json(body).send().await })
            .await
            .map_err(|e| ServerError::NetworkError(e.to_string()))?;
    handle_response(response).await
}

pub async fn get_request<Q, R>(
    base_url: &str,
    endpoint: &str,
    query: Option<Q>,
) -> Result<R, ServerError>
where
    Q: serde::Serialize,
    R: DeserializeOwned,
{
    let url = format!("{}{}", base_url, endpoint);
    log::info!(
        "Posting url={} with query={}",
        url,
        query
            .as_ref()
            .map(|q| serde_json::to_string(&q).unwrap())
            .unwrap_or("".to_string())
    );

    let response = if let Some(params) = query {
        with_retry(|| async { reqwest::Client::new().get(&url).query(&params).send().await }).await
    } else {
        with_retry(|| async { reqwest::Client::new().get(&url).send().await }).await
    }
    .map_err(|e| ServerError::NetworkError(e.to_string()))?;

    handle_response(response).await
}

async fn handle_response<R: DeserializeOwned>(response: Response) -> Result<R, ServerError> {
    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        let error_message = match serde_json::from_str::<ErrorResponse>(&error_text) {
            Ok(error_resp) => error_resp.message.unwrap_or_else(|| error_resp.error),
            Err(_) => error_text,
        };
        return Err(ServerError::ServerError(status.into(), error_message));
    }
    response
        .json::<R>()
        .await
        .map_err(|e| ServerError::DeserializationError(e.to_string()))
}
