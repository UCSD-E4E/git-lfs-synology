use std::{fs::create_dir_all, path::PathBuf};

use anyhow::Result;
use app_dirs2::{AppDataType, AppInfo, app_root};

pub fn get_config_dir() -> Result<PathBuf> {
    let path = app_root(AppDataType::UserConfig, &AppInfo{
        name: "git-lfs-synology",
        author: "Engineers for Exploration"
    })?;

    if !path.exists() {
        create_dir_all(&path)?;
    }
    
    Ok(path)
}