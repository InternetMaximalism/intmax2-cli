use crate::api::state::State;
use actix_web::{
    error::ErrorUnauthorized,
    post,
    web::{Data, Json},
    Error,
};
use intmax2_interfaces::{
    api::store_vault_server::types::{
        GetDataAllAfterRequest, GetDataAllAfterResponse, GetSenderProofSetRequest,
        GetSenderProofSetResponse, GetUserDataRequest, GetUserDataResponse, SaveDataBatchRequest,
        SaveDataBatchResponse, SaveSenderProofSetRequest, SaveUserDataRequest,
    },
    utils::signature::{Signable, WithAuth},
};

#[post("/save-user-data")]
pub async fn save_user_data(
    state: Data<State>,
    request: Json<WithAuth<SaveUserDataRequest>>,
) -> Result<Json<()>, Error> {
    request
        .inner
        .verify(&request.auth)
        .map_err(ErrorUnauthorized)?;
    let pubkey = request.auth.pubkey;
    let request = &request.inner;
    state
        .store_vault_server
        .save_user_data(pubkey, request.prev_digest, &request.data)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(()))
}

#[post("/get-user-data")]
pub async fn get_user_data(
    state: Data<State>,
    request: Json<WithAuth<GetUserDataRequest>>,
) -> Result<Json<GetUserDataResponse>, Error> {
    request
        .inner
        .verify(&request.auth)
        .map_err(ErrorUnauthorized)?;
    let pubkey = request.auth.pubkey;
    let data = state
        .store_vault_server
        .get_user_data(pubkey)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetUserDataResponse { data }))
}

#[post("/save-sender-proof-set")]
pub async fn save_sender_proof_set(
    state: Data<State>,
    request: Json<WithAuth<SaveSenderProofSetRequest>>,
) -> Result<Json<()>, Error> {
    request
        .inner
        .verify(&request.auth)
        .map_err(ErrorUnauthorized)?;
    let data = &request.inner.data;
    let pubkey = request.auth.pubkey;
    state
        .store_vault_server
        .save_sender_proof_set(pubkey, data)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(()))
}

#[post("/get-sender-proof-set")]
pub async fn get_sender_proof_set(
    state: Data<State>,
    request: Json<WithAuth<GetSenderProofSetRequest>>,
) -> Result<Json<GetSenderProofSetResponse>, Error> {
    request
        .inner
        .verify(&request.auth)
        .map_err(ErrorUnauthorized)?;
    let pubkey = request.auth.pubkey;
    let data = state
        .store_vault_server
        .get_sender_proof_set(pubkey)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetSenderProofSetResponse { data }))
}

#[post("/save-data-batch")]
pub async fn batch_save_data(
    state: Data<State>,
    request: Json<WithAuth<SaveDataBatchRequest>>,
) -> Result<Json<SaveDataBatchResponse>, Error> {
    request
        .inner
        .verify(&request.auth)
        .map_err(ErrorUnauthorized)?;
    let pubkey = request.auth.pubkey;
    let entries = &request.inner.data;

    const MAX_BATCH_SIZE: usize = 1000;
    if entries.len() > MAX_BATCH_SIZE {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Batch size exceeds maximum limit of {}",
            MAX_BATCH_SIZE
        )));
    }

    for entry in entries {
        if entry.data_type.need_auth() && entry.pubkey != pubkey {
            return Err(ErrorUnauthorized(format!(
                "Data type {:?} requires auth but given pubkey is different",
                entry.data_type,
            )));
        }
    }
    let uuids = state
        .store_vault_server
        .batch_save_data(entries)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(SaveDataBatchResponse { uuids }))
}

#[post("/get-data-all-after")]
pub async fn get_data_all_after(
    state: Data<State>,
    request: Json<WithAuth<GetDataAllAfterRequest>>,
) -> Result<Json<GetDataAllAfterResponse>, Error> {
    request
        .inner
        .verify(&request.auth)
        .map_err(ErrorUnauthorized)?;
    let pubkey = request.auth.pubkey;
    let request = &request.inner;
    let data = state
        .store_vault_server
        .get_data_all_after(request.data_type, pubkey, request.timestamp)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(Json(GetDataAllAfterResponse { data }))
}

pub fn store_vault_server_scope() -> actix_web::Scope {
    actix_web::web::scope("/store-vault-server")
        .service(save_user_data)
        .service(get_user_data)
        .service(save_sender_proof_set)
        .service(get_sender_proof_set)
        .service(batch_save_data)
        .service(get_data_all_after)
}
