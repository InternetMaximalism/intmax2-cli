use std::{path::PathBuf, sync::OnceLock};

use actix_web::{error::ErrorInternalServerError, get, web::Json, Error};
use cargo_metadata::MetadataCommand;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub name: String,
    pub version: String,
}

#[get("/health-check")]
pub async fn health_check() -> Result<Json<HealthCheckResponse>, Error> {
    let info = get_package_info().map_err(ErrorInternalServerError)?;
    Ok(Json(HealthCheckResponse {
        name: info.name.clone(),
        version: info.version.clone(),
    }))
}

/// Cached package information
static PACKAGE_INFO: OnceLock<PackageInfo> = OnceLock::new();

#[derive(Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

#[derive(Error, Debug)]
pub enum PackageInfoError {
    #[error("Failed to execute cargo metadata: {0}")]
    MetadataError(#[from] cargo_metadata::Error),

    #[error("Failed to get current directory: {0}")]
    CurrentDirError(#[from] std::io::Error),

    #[error("Package not found in metadata")]
    PackageNotFound,
}

// Get package information from Cargo.toml
pub fn get_package_info() -> Result<&'static PackageInfo, PackageInfoError> {
    PACKAGE_INFO.get_or_try_init(|| {
        let metadata = MetadataCommand::new().no_deps().exec()?;
        let current_dir = std::env::current_dir()?;
        let manifest_path = current_dir.join("Cargo.toml");
        let canonical_manifest_path = manifest_path.canonicalize()?;

        let package = metadata
            .packages
            .iter()
            .find(|p| {
                PathBuf::from(&p.manifest_path).canonicalize().ok()
                    == Some(canonical_manifest_path.clone())
            })
            .ok_or(PackageInfoError::PackageNotFound)?;

        Ok(PackageInfo {
            name: package.name.clone(),
            version: package.version.to_string(),
        })
    })
}
