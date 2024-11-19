use std::{fs::create_dir_all, path::PathBuf};

use anyhow::Result;
use app_dirs2::{AppDataType, AppInfo, app_root};
use tracing::debug;

#[tracing::instrument]
pub fn get_config_dir() -> Result<PathBuf> {
    let path = app_root(AppDataType::UserConfig, &AppInfo{
        name: "git-lfs-synology",
        author: "Engineers for Exploration"
    })?;

    if !path.exists() {
        debug!("Config directory does not exist, creating it.");
        create_dir_all(&path)?;
    }
    
    Ok(path)
}