use actix_web::{
    post,
    web::{scope, Data, Json},
    Error, Scope,
};

use crate::api::{
    balance_prover::types::{
        ProveReceiveDepositRequest, ProveReceiveTransferRequest, ProveResponse, ProveSendRequest,
        ProveSingleWithdrawalRequest, ProveSpentRequest, ProveUpdateRequest,
    },
    state::State,
};

#[post("/prove-spent")]
pub async fn prove_spent(
    state: Data<State>,
    request: Json<ProveSpentRequest>,
) -> Result<Json<ProveResponse>, Error> {
    let proof = state
        .balance_prover
        .prove_spent(&request.spent_witness)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(ProveResponse { proof }))
}

#[post("/prove-send")]
pub async fn prove_send(
    state: Data<State>,
    request: Json<ProveSendRequest>,
) -> Result<Json<ProveResponse>, Error> {
    let proof = state
        .balance_prover
        .prove_send(
            request.pubkey,
            &request.tx_witnes,
            &request.update_witness,
            &request.spent_proof,
            &request.prev_proof,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(ProveResponse { proof }))
}

#[post("/prove-update")]
pub async fn prove_update(
    state: Data<State>,
    request: Json<ProveUpdateRequest>,
) -> Result<Json<ProveResponse>, Error> {
    let proof = state
        .balance_prover
        .prove_update(request.pubkey, &request.update_witness, &request.prev_proof)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(ProveResponse { proof }))
}

#[post("/prove-receive-transfer")]
pub async fn prove_receive_transfer(
    state: Data<State>,
    request: Json<ProveReceiveTransferRequest>,
) -> Result<Json<ProveResponse>, Error> {
    let request = request.into_inner();
    let proof = state
        .balance_prover
        .prove_receive_transfer(
            request.pubkey,
            &request.receive_transfer_witness,
            &request.prev_proof,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(ProveResponse { proof }))
}

#[post("/prove-receive-deposit")]
pub async fn prove_receive_deposit(
    state: Data<State>,
    request: Json<ProveReceiveDepositRequest>,
) -> Result<Json<ProveResponse>, Error> {
    let request = request.into_inner();
    let proof = state
        .balance_prover
        .prove_receive_deposit(
            request.pubkey,
            &request.receive_deposit_witness,
            &request.prev_proof,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(ProveResponse { proof }))
}

#[post("/prove-single-withdrawal")]
pub async fn prove_single_withdrawal(
    state: Data<State>,
    request: Json<ProveSingleWithdrawalRequest>,
) -> Result<Json<ProveResponse>, Error> {
    let request = request.into_inner();
    let proof = state
        .balance_prover
        .prove_single_withdrawal(&request.withdrawal_witness)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(ProveResponse { proof }))
}

pub fn balance_prover_scope() -> Scope {
    scope("/balance-prover")
        .service(prove_spent)
        .service(prove_send)
        .service(prove_update)
        .service(prove_receive_transfer)
        .service(prove_receive_deposit)
        .service(prove_single_withdrawal)
}
