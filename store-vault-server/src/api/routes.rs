use std::str::FromStr;

use actix_web::{
    error::ErrorUnauthorized,
    get, post,
    web::{Data, Json, Path},
    Error,
};
use intmax2_interfaces::api::store_vault_server::{
    interface::DataType,
    types::{
        BatchGetDataQuery, BatchGetDataResponse, BatchSaveDataRequest, BatchSaveDataResponse,
        GetBalanceProofQuery, GetBalanceProofResponse, GetDataAllAfterRequestWithSignature,
        GetDataAllAfterResponse, GetDataQuery, GetDataResponse, GetUserDataRequestWithSignature,
        GetUserDataResponse, SaveBalanceProofRequest, SaveDataRequestWithSignature,
        SaveDataResponse,
    },
};
use serde_qs::actix::QsQuery;

use crate::{api::state::State, app::authorization::RequestWithSignature as _};

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
        .map_err(actix_web::error::ErrorInternalServerError)?;
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
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetBalanceProofResponse { balance_proof }))
}

#[post("/{type}/save")]
pub async fn save_data(
    state: Data<State>,
    path: Path<String>,
    request: Json<SaveDataRequestWithSignature>,
) -> Result<Json<SaveDataResponse>, Error> {
    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;

    let request = request.into_inner();
    if data_type == DataType::Tx {
        request.verify().map_err(ErrorUnauthorized)?;
    }

    let uuid = state
        .store_vault_server
        .save_data(data_type, request.pubkey, request.data)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
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
    match data_type {
        DataType::Tx => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "data_type {} is not supported",
                data_type
            )));
        }
        _ => {
            println!("No authorization required for {}", data_type);
        }
    }

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
        .map_err(actix_web::error::ErrorInternalServerError)?;

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
        .map_err(actix_web::error::ErrorInternalServerError)?;
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
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(Json(BatchGetDataResponse { data }))
}

#[post("/{type}/get-all-after")]
pub async fn get_data_all_after(
    state: Data<State>,
    path: Path<String>,
    request: Json<GetDataAllAfterRequestWithSignature>,
) -> Result<Json<GetDataAllAfterResponse>, Error> {
    let data_type = path.into_inner();
    let data_type = DataType::from_str(data_type.as_str())
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid type: {}", e)))?;
    let request = request.into_inner();
    request.auth.verify().map_err(ErrorUnauthorized)?;

    let data = state
        .store_vault_server
        .get_data_all_after(data_type, request.auth.pubkey, request.timestamp)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetDataAllAfterResponse { data }))
}

#[post("/save-user-data")]
pub async fn save_user_data(
    state: Data<State>,
    request: Json<SaveDataRequestWithSignature>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    request.verify().map_err(ErrorUnauthorized)?;

    state
        .store_vault_server
        .save_user_data(request.pubkey, request.data)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(()))
}

#[post("/get-user-data")]
pub async fn get_user_data(
    state: Data<State>,
    request: Json<GetUserDataRequestWithSignature>,
) -> Result<Json<GetUserDataResponse>, Error> {
    let request = request.into_inner();
    request.auth.verify().map_err(ErrorUnauthorized)?;

    let data = state
        .store_vault_server
        .get_user_data(request.auth.pubkey)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
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