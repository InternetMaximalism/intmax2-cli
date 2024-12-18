use actix_web::{get, web::Json, Error};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub name: String,
    pub version: String,
}

/// Because the health check endpoint is used by all services, we need to set the environment variables
pub fn set_health_check_env_vars(name: &str, version: &str) {
    std::env::set_var("HEALTH_CHECK_NAME", name);
    std::env::set_var("HEALTH_CHECK_VERSION", version);
}

#[get("/health-check")]
pub async fn health_check() -> Result<Json<HealthCheckResponse>, Error> {
    let name = std::env::var("HEALTH_CHECK_NAME").map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("HEALTH_CHECK_NAME is not set: {}", e))
    })?;
    let version = std::env::var("HEALTH_CHECK_VERSION").map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "HEALTH_CHECK_VERSION is not set: {}",
            e
        ))
    })?;
    Ok(Json(HealthCheckResponse { name, version }))
}
