use anyhow::Result;
use clap::ArgMatches;

use crate::{credential_manager::CredentialManager, git_lfs::{CustomTransferAgent, GitLfsParser}};

use super::Subcommand;

#[derive(Debug)]
pub struct MainSubcommand {
    credential_manager: Option<CredentialManager>
}

impl CustomTransferAgent for MainSubcommand {
    async fn init(&mut self) -> Result<()> {
        self.credential_manager = Some(CredentialManager::new()?);
        // Init synology api

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
            credential_manager: None
        }
    }
}