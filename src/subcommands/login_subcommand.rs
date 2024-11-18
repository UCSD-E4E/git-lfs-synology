use anyhow::{Context, Result};
use clap::ArgMatches;

use crate::subcommands::Subcommand;
use crate::credential_manager::CredentialManager;

pub struct LoginSubcommand {
}

impl Subcommand for LoginSubcommand {
    fn execute(&self, arg_matches: &ArgMatches) -> Result<()> {
        let url = arg_matches.get_one::<String>("URL").context("URL not provided.")?;
        let user = arg_matches.get_one::<String>("USER").context("USER not provided.")?;

        let credential_manager = CredentialManager::new()?;

        if !credential_manager.has_credential(url)? {
            // TODO need to ask for password from user
        }

        Ok(())

        // if credential_manager.has_credential(url) {
        //     // get password and totp command
        // }
        // else {
        //     // get password and totop command
        // }

        // try login
        // if success store

        // else throw error
    }
}