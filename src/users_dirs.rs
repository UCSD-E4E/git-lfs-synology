use std::{fs::create_dir_all, path::PathBuf};

use anyhow::Result;
use app_dirs2::{AppDataType, AppInfo, app_root};
use tracing::info;

#[tracing::instrument]
fn get_app_info() -> AppInfo {
    AppInfo{
        name: "git-lfs-synology",
        author: "Engineers for Exploration"
    }
}

#[tracing::instrument]
pub fn get_cache_dir() -> Result<PathBuf> {
    let path = app_root(AppDataType::UserCache, &get_app_info())?;

    if !path.exists() {
        info!("Cache directory does not exist, creating it.");
        create_dir_all(&path)?;
    }

    Ok(path)
}

#[tracing::instrument]
pub fn get_config_dir() -> Result<PathBuf> {
    let path = app_root(AppDataType::UserConfig, &get_app_info())?;

    if !path.exists() {
        info!("Config directory does not exist, creating it.");
        create_dir_all(&path)?;
    }
    
    Ok(path)
}