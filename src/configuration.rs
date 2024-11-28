use std::path::PathBuf;

use anyhow::{anyhow, Context, Ok, Result};
use gix_config::File;
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub nas_url: String,
    pub path: String
}

impl Configuration {
    #[tracing::instrument]
    pub fn load() -> Result<Configuration> {
        let mut path = PathBuf::new();
        path.push("./");
        path.push(".lfsconfig");

        let config = File::from_path_no_includes(path, gix_config::Source::Local)?;
        let section = config.section("lfs", None)?;

        let url = section.value("url").context("Url should be set.")?.to_string();
        debug!("Url found: {}", url);

        let url = if url.starts_with("filestation-secure://") {
            Ok(url.replace("filestation-secure", "https"))
        }
        else if url.starts_with("filestation://") {
            Ok(url.replace("filestation", "http"))
        }
        else {
            Err(anyhow!("Url is not set incorrectly."))
        }?;
        debug!("Url updated: {}", url);

        let url_parsed = Url::parse(url.as_str())?;

        let path = url_parsed.path();
        let nas_url = url.replace(path, "");

        Ok(
            Configuration {
                nas_url: nas_url.to_string(),
                path: path.to_string()
            }
        )
    }
}