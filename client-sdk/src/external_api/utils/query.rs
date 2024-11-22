use intmax2_interfaces::api::error::ServerError;

use super::retry::with_retry;

pub async fn post_request<T: serde::Serialize, U: serde::de::DeserializeOwned>(
    base_url: &str,
    endpoint: &str,
    body: &T,
) -> Result<U, ServerError> {
    let url = format!("{}{}", base_url, endpoint);
    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(&url)
            .json(body)
            .send()
            .await
    })
    .await
    .map_err(|e| ServerError::NetworkError(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ServerError::ServerError(response.status().to_string()));
    }

    response
        .json::<U>()
        .await
        .map_err(|e| ServerError::DeserializationError(e.to_string()))
}

pub async fn get_request<T, Q>(
    base_url: &str,
    endpoint: &str,
    query: Option<Q>,
) -> Result<T, ServerError>
where
    T: serde::de::DeserializeOwned,
    Q: serde::Serialize,
{
    let url = format!("{}{}", base_url, endpoint);

    let response = if let Some(params) = query {
        with_retry(|| async {
            reqwest_wasm::Client::new()
                .get(&url)
                .query(&params)
                .send()
                .await
        })
        .await
    } else {
        with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await }).await
    }
    .map_err(|e| ServerError::NetworkError(e.to_string()))?;

    if response.status().is_success() {
        response
            .json::<T>()
            .await
            .map_err(|e| ServerError::DeserializationError(e.to_string()))
    } else {
        Err(ServerError::ServerError(response.status().to_string()))
    }
}