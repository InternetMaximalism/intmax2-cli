use actix_web::{
    get, post,
    web::{Data, Json},
    Error,
};
use intmax2_interfaces::api::block_builder::types::{
    GetBlockBuilderStatusQuery, GetBlockBuilderStatusResponse, PostSignatureRequest,
    QueryProposalRequest, QueryProposalResponse, TxRequestRequest,
};
use intmax2_zkp::common::block_builder::UserSignature;
use serde_qs::actix::QsQuery;

use crate::api::state::State;

// todo: remove in production
#[post("/post-empty-block")]
pub async fn post_empty_block(state: Data<State>) -> Result<Json<()>, Error> {
    state
        .evoke_force_post()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[get("/status")]
pub async fn get_status(
    state: Data<State>,
    query: QsQuery<GetBlockBuilderStatusQuery>,
) -> Result<Json<GetBlockBuilderStatusResponse>, Error> {
    let status = state
        .block_builder
        .read()
        .await
        .get_status(query.is_registration_block);
    Ok(Json(GetBlockBuilderStatusResponse { status }))
}

#[post("/tx-request")]
pub async fn tx_request(
    state: Data<State>,
    request: Json<TxRequestRequest>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    state
        .block_builder
        .write()
        .await
        .send_tx_request(request.is_registration_block, request.pubkey, request.tx)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

#[post("/query-proposal")]
pub async fn query_proposal(
    state: Data<State>,
    request: Json<QueryProposalRequest>,
) -> Result<Json<QueryProposalResponse>, Error> {
    let request = request.into_inner();
    let block_proposal = state
        .block_builder
        .read()
        .await
        .query_proposal(request.is_registration_block, request.pubkey, request.tx)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(QueryProposalResponse { block_proposal }))
}

#[post("/post-signature")]
pub async fn post_signature(
    state: Data<State>,
    request: Json<PostSignatureRequest>,
) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    let user_signature = UserSignature {
        pubkey: request.pubkey,
        signature: request.signature,
    };
    state
        .block_builder
        .write()
        .await
        .post_signature(request.is_registration_block, request.tx, user_signature)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

pub fn block_builder_scope() -> actix_web::Scope {
    actix_web::web::scope("/block-builder")
        .service(post_empty_block)
        .service(get_status)
        .service(tx_request)
        .service(query_proposal)
        .service(post_signature)
}
