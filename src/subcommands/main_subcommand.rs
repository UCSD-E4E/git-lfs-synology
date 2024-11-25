use anyhow::{Context, Result};
use clap::ArgMatches;

use crate::{configuration::Configuration, credential_manager::CredentialManager, git_lfs::{CustomTransferAgent, Event, GitLfsParser}, synology_api::SynologyFileStation};

use super::Subcommand;

#[derive(Debug)]
pub struct MainSubcommand {
    file_station: Option<SynologyFileStation>
}

impl CustomTransferAgent for MainSubcommand {
    async fn init(&mut self, _: &Event) -> Result<()> {
        let configuration = Configuration::load()?;
        let mut credential_manager = CredentialManager::new()?;

        let nas_url = configuration.nas_url.as_str();
        let mut file_station = SynologyFileStation::new(nas_url);

        let credential = credential_manager.get_credential(nas_url)?.context("Credential should not be null")?;
        file_station.login(&credential).await?;

        self.file_station = Some(file_station);

        Ok(())
    }
}

impl Subcommand for MainSubcommand {
    #[tracing::instrument]
    async fn execute(&mut self, arg_matches: &ArgMatches) -> Result<()> {
        let mut parser = GitLfsParser::<MainSubcommand>::new(self);
        parser.listen().await?;

        Ok(())
    }
}

impl MainSubcommand {
    pub fn new() -> MainSubcommand {
        MainSubcommand {
            file_station: None
        }
    }
}