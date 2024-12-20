use std::env;

use actix_web::{get, web::Json, Error};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub name: String,
    pub version: String,
}

#[get("/health-check")]
pub async fn health_check() -> Result<Json<HealthCheckResponse>, Error> {
    let (name, version) = load_name_and_version();
    Ok(Json(HealthCheckResponse { name, version }))
}

pub fn set_name_and_version(name: &str, version: &str) {
    env::set_var("RUNTIME_CARGO_PKG_NAME", name);
    env::set_var("RUNTIME_CARGO_PKG_VERSION", version);
}

pub fn load_name_and_version() -> (String, String) {
    let name = env::var("RUNTIME_CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string());
    let version = env::var("RUNTIME_CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    (name, version)
}
