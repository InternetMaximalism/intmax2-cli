use serde::Deserialize;

pub mod api;
pub mod health_check;

#[derive(Debug, Deserialize)]
pub struct Env {
    pub port: u16,
    pub database_url: String,
}
