use crate::subcommands::subcommand::Subcommand;
use crate::credential_manager::CredentialManager;

use anyhow::{Context, Result};
use clap::ArgMatches;

pub struct LogoutSubcommand {
}

impl Subcommand for LogoutSubcommand {
    fn execute(&self, arg_matches: &ArgMatches) -> Result<()> {
        let url = arg_matches.get_one::<String>("URL").context("URL not provided.")?;

        let mut credential_manager = CredentialManager::new()?;
        credential_manager.remove_credential(url)?;

        Ok(())
    }
}
