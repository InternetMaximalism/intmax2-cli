use serde::Deserialize;

pub mod api;
pub mod health_check;

#[derive(Deserialize)]
pub struct Env {
    pub port: u16,
}
