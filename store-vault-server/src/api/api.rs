use std::str::FromStr;

use actix_web::{
    get, post,
    web::{Data, Json, Path},
    Error,
};
use intmax2_interfaces::api::store_vault_server::{
    interface::DataType,
    types::{
        BatchGetDataQuery, BatchGetDataResponse, BatchSaveDataRequest, BatchSaveDataResponse, GetBalanceProofQuery, GetBalanceProofResponse, GetDataAllAfterQuery, GetDataAllAfterResponse, GetDataQuery, GetDataResponse, GetUserDataQuery, GetUserDataResponse, SaveBalanceProofRequest, SaveDataRequest, SaveDataResponse
    },
};
use serde_qs::actix::QsQuery;

use crate::api::state::State;

#[post("/save-balance-proof")]
pub async fn save_balance_proof(
    state: Data<State>,
    request: Json<SaveBalanceProofRequest>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    state
        .store_vault_server
        .save_balance_proof(request.pubkey, request.balance_proof)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[get("/get-balance-proof")]
pub async fn get_balance_proof(
    state: Data<State>,
    query: QsQuery<GetBalanceProofQuery>,
) -> Result<Json<GetBalanceProofResponse>, Error> {
    let query = query.into_inner();
    let balance_proof = state
        .store_vault_server
        .get_balance_proof(query.pubkey, query.block_number, query.private_commitment)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetBalanceProofResponse { balance_proof }))
}

#[post("/{type}/save")]
pub async fn save_data(
    state: Data<State>,
    path: Path<String>,
    request: Json<SaveDataRequest>,
) -> Result<Json<SaveDataResponse>, Error> {
    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;
    let request = request.into_inner();
    let uuid = state
        .store_vault_server
        .save_data(data_type, request.pubkey, request.data)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(SaveDataResponse { uuid }))
}

#[post("/{type}/batch-save")]
pub async fn batch_save_data(
    state: Data<State>,
    path: Path<String>,
    request: Json<BatchSaveDataRequest>,
) -> Result<Json<BatchSaveDataResponse>, Error> {
    const MAX_BATCH_SIZE: usize = 1000;

    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;

    let requests = request.into_inner().requests;

    if requests.len() > MAX_BATCH_SIZE {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Batch size exceeds maximum limit of {}",
            MAX_BATCH_SIZE
        )));
    }

    let uuids = state
        .store_vault_server
        .batch_save_data(data_type, requests)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(Json(BatchSaveDataResponse { uuids }))
}

#[get("/{type}/get")]
pub async fn get_data(
    state: Data<State>,
    path: Path<String>,
    query: QsQuery<GetDataQuery>,
) -> Result<Json<GetDataResponse>, Error> {
    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;
    let query = query.into_inner();
    let data = state
        .store_vault_server
        .get_data(data_type, &query.uuid)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetDataResponse { data }))
}

#[get("/{type}/batch-get")]
pub async fn batch_get_data(
    state: Data<State>,
    path: Path<String>,
    query: QsQuery<BatchGetDataQuery>,
) -> Result<Json<BatchGetDataResponse>, Error> {
    const MAX_BATCH_SIZE: usize = 1000;

    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;

    let uuids = query.uuids.clone();

    if uuids.len() > MAX_BATCH_SIZE {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Batch size exceeds maximum limit of {}",
            MAX_BATCH_SIZE
        )));
    }

    let data = state
        .store_vault_server
        .batch_get_data(data_type, &uuids)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(Json(BatchGetDataResponse { data }))
}

#[get("/{type}/get-all-after")]
pub async fn get_data_all_after(
    state: Data<State>,
    path: Path<String>,
    query: QsQuery<GetDataAllAfterQuery>,
) -> Result<Json<GetDataAllAfterResponse>, Error> {
    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;
    let query = query.into_inner();
    let data = state
        .store_vault_server
        .get_data_all_after(data_type, query.pubkey, query.timestamp)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetDataAllAfterResponse { data }))
}

#[post("/save-user-data")]
pub async fn save_user_data(
    state: Data<State>,
    request: Json<SaveDataRequest>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    state
        .store_vault_server
        .save_user_data(request.pubkey, request.data)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[get("/get-user-data")]
pub async fn get_user_data(
    state: Data<State>,
    query: QsQuery<GetUserDataQuery>,
) -> Result<Json<GetUserDataResponse>, Error> {
    let query = query.into_inner();
    let data = state
        .store_vault_server
        .get_user_data(query.pubkey)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetUserDataResponse { data }))
}

pub fn store_vault_server_scope() -> actix_web::Scope {
    actix_web::web::scope("/store-vault-server")
        .service(save_balance_proof)
        .service(get_balance_proof)
        .service(save_data)
        .service(batch_save_data)
        .service(get_data)
        .service(batch_get_data)
        .service(get_data_all_after)
        .service(save_user_data)
        .service(get_user_data)
}
