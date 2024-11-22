use actix_web::{get, web::Json, Error};

#[get("/health-check")]
pub async fn health_check() -> Result<Json<()>, Error> {
    Ok(Json(()))
}
