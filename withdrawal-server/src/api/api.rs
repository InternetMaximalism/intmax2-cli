use crate::api::state::State;
use actix_web::{
    get, post,
    web::{Data, Json},
    Error, Scope,
};
use intmax2_interfaces::api::withdrawal_server::{
    interface::Fee,
    types::{
        GetFeeResponse, GetWithdrawalInfoByRecipientRequest, GetWithdrawalInfoRequest,
        GetWithdrawalInfoResponse, RequestWithdrawalRequest,
    },
};
use serde_qs::actix::QsQuery;

#[get("/fee")]
pub async fn get_fee() -> Result<Json<GetFeeResponse>, Error> {
    let fees = vec![Fee {
        token_index: 0,
        constant: 0,
        coefficient: 0.0,
    }];
    Ok(Json(GetFeeResponse { fees }))
}

#[post("/request-withdrawal")]
pub async fn request_withdrawal(
    state: Data<State>,
    request: Json<RequestWithdrawalRequest>,
) -> Result<Json<()>, Error> {
    state
        .withdrawal_server
        .request_withdrawal(request.pubkey, &request.single_withdrawal_proof)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[get("/get-withdrawal-info")]
pub async fn get_withdrawal_info(
    state: Data<State>,
    query: QsQuery<GetWithdrawalInfoRequest>,
) -> Result<Json<GetWithdrawalInfoResponse>, Error> {
    let withdrawal_info = state
        .withdrawal_server
        .get_withdrawal_info(query.pubkey, query.signature.clone())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(Json(GetWithdrawalInfoResponse { withdrawal_info }))
}

#[get("/get-withdrawal-info-by-recipient")]
pub async fn get_withdrawal_info_by_recipient(
    state: Data<State>,
    query: QsQuery<GetWithdrawalInfoByRecipientRequest>,
) -> Result<Json<GetWithdrawalInfoResponse>, Error> {
    let withdrawal_info = state
        .withdrawal_server
        .get_withdrawal_info_by_recipient(query.recipient)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetWithdrawalInfoResponse { withdrawal_info }))
}

pub fn withdrawal_server_scope() -> Scope {
    actix_web::web::scope("/withdrawal-server")
        .service(request_withdrawal)
        .service(get_fee)
        .service(get_withdrawal_info)
        .service(get_withdrawal_info_by_recipient)
}
