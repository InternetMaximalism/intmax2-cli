use actix_web::{
    get, post,
    web::{Data, Json, Path, Query},
    Error,
};

use crate::api::{
    state::State,
    store_vault_server::types::{
        GetBalanceProofResponse, GetDataResponse, GetUserDataQuery, GetUserDataResponse,
        SaveBalanceProofRequest,
    },
};

use super::types::{
    GetBalanceProofQuery, GetDataAllAfterQuery, GetDataAllAfterResponse, GetDataQuery,
    SaveDataRequest,
};

#[post("/save-balance-proof")]
pub async fn save_balance_proof(
    data: Data<State>,
    request: Json<SaveBalanceProofRequest>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    data.store_vault_server
        .save_balance_proof(request.pubkey, request.balance_proof)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[get("/get-balance-proof")]
pub async fn get_balance_proof(
    data: Data<State>,
    query: Query<GetBalanceProofQuery>,
) -> Result<Json<GetBalanceProofResponse>, Error> {
    let query = query.into_inner();
    let balance_proof = data
        .store_vault_server
        .get_balance_proof(query.pubkey, query.block_number, query.private_commitment)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetBalanceProofResponse { balance_proof }))
}

#[post("/{type}/save")]
pub async fn save_data(
    data: Data<State>,
    path: Path<String>,
    request: Json<SaveDataRequest>,
) -> Result<Json<()>, Error> {
    let t = path.into_inner();
    let request = request.into_inner();
    match t.as_str() {
        "deposit" => {
            data.store_vault_server
                .save_deposit_data(request.pubkey, request.data)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
        "transfer" => {
            data.store_vault_server
                .save_transfer_data(request.pubkey, request.data)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
        "tx" => {
            data.store_vault_server
                .save_tx_data(request.pubkey, request.data)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
        "withdrawal" => {
            data.store_vault_server
                .save_withdrawal_data(request.pubkey, request.data)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        }
        _ => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Invalid type".to_string(),
            ));
        }
    };
    Ok(Json(()))
}

#[get("/{type}/get")]
pub async fn get_data(
    data: Data<State>,
    path: Path<String>,
    query: Query<GetDataQuery>,
) -> Result<Json<GetDataResponse>, Error> {
    let t = path.into_inner();
    let query = query.into_inner();
    let data = match t.as_str() {
        "deposit" => data
            .store_vault_server
            .get_deposit_data(&query.uuid)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        "transfer" => data
            .store_vault_server
            .get_transfer_data(&query.uuid)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        "tx" => data
            .store_vault_server
            .get_tx_data(&query.uuid)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        "withdrawal" => data
            .store_vault_server
            .get_withdrawal_data(&query.uuid)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        _ => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Invalid type".to_string(),
            ));
        }
    };
    Ok(Json(GetDataResponse { data }))
}

#[get("/{type}/get-all-after")]
pub async fn get_data_all_after(
    data: Data<State>,
    path: Path<String>,
    query: Query<GetDataAllAfterQuery>,
) -> Result<Json<GetDataAllAfterResponse>, Error> {
    let t = path.into_inner();
    let query = query.into_inner();
    let data = match t.as_str() {
        "deposit" => data
            .store_vault_server
            .get_deposit_data_all_after(query.pubkey, query.timestamp)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        "transfer" => data
            .store_vault_server
            .get_transfer_data_all_after(query.pubkey, query.timestamp)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        "tx" => data
            .store_vault_server
            .get_tx_data_all_after(query.pubkey, query.timestamp)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        "withdrawal" => data
            .store_vault_server
            .get_withdrawal_data_all_after(query.pubkey, query.timestamp)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?,
        _ => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Invalid type".to_string(),
            ));
        }
    };
    Ok(Json(GetDataAllAfterResponse { data }))
}

#[post("/save-user-data")]
pub async fn save_user_data(
    data: Data<State>,
    request: Json<SaveDataRequest>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    data.store_vault_server
        .save_user_data(request.pubkey, request.data)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[get("/get-user-data")]
pub async fn get_user_data(
    data: Data<State>,
    query: Query<GetUserDataQuery>,
) -> Result<Json<GetUserDataResponse>, Error> {
    let query = query.into_inner();
    let data = data
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
        .service(get_data)
        .service(get_data_all_after)
        .service(save_user_data)
        .service(get_user_data)
}
