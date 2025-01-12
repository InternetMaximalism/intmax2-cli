use crate::api::state::State;
use actix_web::{
    error::ErrorUnauthorized,
    post,
    web::{Data, Json},
    Error,
};
use intmax2_interfaces::api::store_vault_server::types::{
    BatchSaveDataRequest, BatchSaveDataResponse, GetDataAllAfterRequest, GetDataAllAfterResponse,
    GetUserDataRequest, GetUserDataResponse, SaveUserDataRequest,
};

#[post("/save-user-data")]
pub async fn save_user_data(
    state: Data<State>,
    request: Json<SaveUserDataRequest>,
) -> Result<Json<()>, Error> {
    request
        .auth
        .verify(&request.content())
        .map_err(ErrorUnauthorized)?;
    state
        .store_vault_server
        .save_user_data(request.auth.pubkey, request.prev_digest, &request.data)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(()))
}

#[post("/get-user-data")]
pub async fn get_user_data(
    state: Data<State>,
    request: Json<GetUserDataRequest>,
) -> Result<Json<GetUserDataResponse>, Error> {
    request
        .auth
        .verify(&request.content())
        .map_err(ErrorUnauthorized)?;
    let data = state
        .store_vault_server
        .get_user_data(request.auth.pubkey)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetUserDataResponse { data }))
}

#[post("/batch-save")]
pub async fn batch_save_data(
    state: Data<State>,
    request: Json<BatchSaveDataRequest>,
) -> Result<Json<BatchSaveDataResponse>, Error> {
    const MAX_BATCH_SIZE: usize = 1000;
    if request.data.len() > MAX_BATCH_SIZE {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Batch size exceeds maximum limit of {}",
            MAX_BATCH_SIZE
        )));
    }
    request
        .auth
        .verify(&request.content())
        .map_err(ErrorUnauthorized)?;
    let pubkey = request.auth.pubkey;
    for entry in &request.data {
        if entry.data_type.need_auth() {
            if entry.pubkey != pubkey {
                return Err(ErrorUnauthorized(format!(
                    "Data type {:?} requires auth but given pubkey is different",
                    entry.data_type,
                )));
            }
        }
    }

    let uuids = state
        .store_vault_server
        .batch_save_data(&request.data)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(BatchSaveDataResponse { uuids }))
}

#[post("/get-all-after")]
pub async fn get_data_all_after(
    state: Data<State>,
    request: Json<GetDataAllAfterRequest>,
) -> Result<Json<GetDataAllAfterResponse>, Error> {
    request
        .auth
        .verify(&request.content())
        .map_err(ErrorUnauthorized)?;
    let data = state
        .store_vault_server
        .get_data_all_after(request.data_type, request.auth.pubkey, request.timestamp)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetDataAllAfterResponse { data }))
}

pub fn store_vault_server_scope() -> actix_web::Scope {
    actix_web::web::scope("/store-vault-server")
        .service(save_user_data)
        .service(get_user_data)
        .service(batch_save_data)
        .service(get_data_all_after)
}
