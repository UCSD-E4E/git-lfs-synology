use std::{fs::read_to_string, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub nas_url: String,
    pub path: String
}

impl Configuration {
    pub fn load() -> Result<Configuration> {
        let mut path = PathBuf::new();
        path.push("./");
        path.push(".git-lfs-synology.yaml");

        let yaml_string = read_to_string(path)?;

        Ok(serde_yml::from_str::<Configuration>(yaml_string.as_str())?)
    }
}