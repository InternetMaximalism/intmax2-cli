use actix_web::{
    post,
    web::{Data, Json},
    Error,
};
use intmax2_interfaces::api::withdrawal_server::types::RequestWithdrawalRequest;

use crate::api::state::State;

#[post("/request-withdrawal")]
pub async fn request_withdrawal(
    _state: Data<State>,
    _request: Json<RequestWithdrawalRequest>,
) -> Result<Json<()>, Error> {
    // todo: implement request withdrawal
    Ok(Json(()))
}

pub fn withdrawal_server_scope() -> actix_web::Scope {
    actix_web::web::scope("/withdrawal-server").service(request_withdrawal)
}
