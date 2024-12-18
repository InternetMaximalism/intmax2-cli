use serde::Deserialize;

pub mod api;

#[derive(Deserialize)]
pub struct Env {
    pub port: u16,
}
