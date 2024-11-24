use crate::subcommands::subcommand::Subcommand;
use crate::credential_manager::CredentialManager;

use anyhow::{Context, Result};
use clap::ArgMatches;

#[derive(Debug)]
pub struct LogoutSubcommand {
}

impl Subcommand for LogoutSubcommand {
    #[tracing::instrument]
    async fn execute(&mut self, arg_matches: &ArgMatches) -> Result<()> {
        let url = arg_matches.get_one::<String>("URL").context("URL not provided.")?;

        let mut credential_manager = CredentialManager::new()?;
        credential_manager.remove_credential(url)?;

        Ok(())
    }
}
