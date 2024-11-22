use actix_web::{
    get,
    web::{self, Json},
    Error,
};

use super::state::State;

/// Resets the state of the server.
#[get("/reset")]
pub async fn reset(data: web::Data<State>) -> Result<Json<()>, Error> {
    data.reset();
    Ok(Json(()))
}
