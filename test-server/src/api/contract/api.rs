use actix_web::{
    post,
    web::{Data, Json},
    Error,
};
use ethers::types::H256;
use intmax2_core_sdk::external_api::contract::interface::ContractInterface as _;

use crate::api::state::State;

use super::types::DepositRequest;

#[post("/deposit")]
pub async fn deposit(data: Data<State>, request: Json<DepositRequest>) -> Result<Json<()>, Error> {
    let request = request.into_inner();
    data.contract
        .deposit(
            H256::zero(),
            request.pubkey_salt_hash,
            request.token_index,
            request.amount,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(Json(()))
}

pub fn contract_scope() -> actix_web::Scope {
    actix_web::web::scope("/contract").service(deposit)
}
