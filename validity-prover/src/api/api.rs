use crate::api::state::State;
use actix_web::{
    get,
    web::{Data, Json},
    Error,
};
use intmax2_interfaces::api::validity_prover::types::{
    GetAccountInfoQuery, GetAccountInfoResponse, GetBlockMerkleProofQuery,
    GetBlockMerkleProofResponse, GetBlockNumberByTxTreeRootQuery,
    GetBlockNumberByTxTreeRootResponse, GetBlockNumberResponse, GetDepositInfoQuery,
    GetDepositInfoResponse, GetDepositMerkleProofQuery, GetDepositMerkleProofResponse,
    GetNextDepositIndexResponse, GetSenderLeavesQuery, GetSenderLeavesResponse,
    GetUpdateWitnessQuery, GetUpdateWitnessResponse, GetValidityPisQuery, GetValidityPisResponse,
};
use serde_qs::actix::QsQuery;

#[get("/block-number")]
pub async fn get_block_number(state: Data<State>) -> Result<Json<GetBlockNumberResponse>, Error> {
    let block_number = state
        .validity_prover
        .get_block_number()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetBlockNumberResponse { block_number }))
}

#[get("/next-deposit-index")]
pub async fn get_next_deposit_index(
    state: Data<State>,
) -> Result<Json<GetNextDepositIndexResponse>, Error> {
    let deposit_index = state
        .validity_prover
        .get_next_deposit_index()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetNextDepositIndexResponse { deposit_index }))
}

#[get("/get-account-info")]
pub async fn get_account_info(
    state: Data<State>,
    query: QsQuery<GetAccountInfoQuery>,
) -> Result<Json<GetAccountInfoResponse>, Error> {
    let query = query.into_inner();
    if let Some(block_number) = query.block_number {
        let account_info = state
            .validity_prover
            .get_account_info_by_block_number(block_number, query.pubkey)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        return Ok(Json(GetAccountInfoResponse { account_info }));
    }

    let account_info = state
        .validity_prover
        .get_account_info(query.pubkey)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetAccountInfoResponse { account_info }))
}

#[get("/get-update-witness")]
pub async fn get_update_witness(
    state: Data<State>,
    query: QsQuery<GetUpdateWitnessQuery>,
) -> Result<Json<GetUpdateWitnessResponse>, Error> {
    let query = query.into_inner();
    let update_witness = state
        .validity_prover
        .get_update_witness(
            query.pubkey,
            query.root_block_number,
            query.leaf_block_number,
            query.is_prev_account_tree,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetUpdateWitnessResponse { update_witness }))
}

#[get("/get-deposit-info")]
pub async fn get_deposit_info(
    state: Data<State>,
    query: QsQuery<GetDepositInfoQuery>,
) -> Result<Json<GetDepositInfoResponse>, Error> {
    let query = query.into_inner();
    let deposit_info = state
        .validity_prover
        .get_deposit_info(query.deposit_hash)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetDepositInfoResponse { deposit_info }))
}

#[get("/get-block-number-by-tx-tree-root")]
pub async fn get_block_number_by_tx_tree_root(
    state: Data<State>,
    query: QsQuery<GetBlockNumberByTxTreeRootQuery>,
) -> Result<Json<GetBlockNumberByTxTreeRootResponse>, Error> {
    let query = query.into_inner();
    let block_number = state
        .validity_prover
        .get_block_number_by_tx_tree_root(query.tx_tree_root)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetBlockNumberByTxTreeRootResponse { block_number }))
}

#[get("/get-validity-pis")]
pub async fn get_validity_pis(
    state: Data<State>,
    query: QsQuery<GetValidityPisQuery>,
) -> Result<Json<GetValidityPisResponse>, Error> {
    let query = query.into_inner();
    let validity_pis = state
        .validity_prover
        .get_validity_pis(query.block_number)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetValidityPisResponse { validity_pis }))
}

#[get("/get-sender-leaves")]
pub async fn get_sender_leaves(
    state: Data<State>,
    query: QsQuery<GetSenderLeavesQuery>,
) -> Result<Json<GetSenderLeavesResponse>, Error> {
    let query = query.into_inner();
    let sender_leaves = state
        .validity_prover
        .get_sender_leaves(query.block_number)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetSenderLeavesResponse { sender_leaves }))
}

#[get("/get-block-merkle-proof")]
pub async fn get_block_merkle_proof(
    state: Data<State>,
    query: QsQuery<GetBlockMerkleProofQuery>,
) -> Result<Json<GetBlockMerkleProofResponse>, Error> {
    let query = query.into_inner();
    let block_merkle_proof = state
        .validity_prover
        .get_block_merkle_proof(query.root_block_number, query.leaf_block_number)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetBlockMerkleProofResponse { block_merkle_proof }))
}

#[get("/get-deposit-merkle-proof")]
pub async fn get_deposit_merkle_proof(
    state: Data<State>,
    query: QsQuery<GetDepositMerkleProofQuery>,
) -> Result<Json<GetDepositMerkleProofResponse>, Error> {
    let query = query.into_inner();
    let deposit_merkle_proof = state
        .validity_prover
        .get_deposit_merkle_proof(query.block_number, query.deposit_index)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(GetDepositMerkleProofResponse {
        deposit_merkle_proof,
    }))
}

pub fn validity_prover_scope() -> actix_web::Scope {
    actix_web::web::scope("/validity-prover")
        .service(get_block_number)
        .service(get_next_deposit_index)
        .service(get_account_info)
        .service(get_update_witness)
        .service(get_deposit_info)
        .service(get_block_number_by_tx_tree_root)
        .service(get_validity_pis)
        .service(get_sender_leaves)
        .service(get_block_merkle_proof)
        .service(get_deposit_merkle_proof)
}
