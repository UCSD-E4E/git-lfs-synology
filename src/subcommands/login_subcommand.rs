use anyhow::{Context, Result};
use clap::ArgMatches;

use crate::subcommands::Subcommand;
use crate::credential_manager::{Credential, CredentialManager};
use crate::synology_file_station::SynologyFileStation;

#[derive(Debug)]
pub struct LoginSubcommand {
}

impl Subcommand for LoginSubcommand {
    #[tracing::instrument]
    fn execute(&self, arg_matches: &ArgMatches) -> Result<()> {
        let url = arg_matches.get_one::<String>("URL").context("URL not provided.")?;
        let user = arg_matches.get_one::<String>("USER").context("USER not provided.")?;
        let totp_command = arg_matches.get_one::<String>("TOTP_COMMAND");

        let mut credential_manager = CredentialManager::new()?;

        let password: String;
        let credential_ref: Option<Credential>;
        if credential_manager.has_credential(url)? {
            let credential = credential_manager.get_credential(url)?.context("Credential should not be null")?;
            password = credential.password.clone();
            credential_ref = Some(credential);
        }
        else {
            password = rpassword::prompt_password("Synology NAS Password: ")?;
            credential_ref = None;
        }

        let totp_command = match totp_command {
            Some(totp_command) => Some(totp_command.clone()),
            None => match credential_ref {
                Some(credential) => match credential.totp_command {
                    Some(totp_command) => Some(totp_command),
                    None => None
                },
                None => None
            }
        };

        let credential = Credential::new(
            user.clone(),
            password.clone(),
            totp_command);

        let file_station = SynologyFileStation::new(url);
        let result = file_station.login(&credential);

        match result {
            Ok(_) => credential_manager.set_credential(url, &credential),
            Err(error) => match error {
                _ => Err(error).map_err(anyhow::Error::msg)
            }
        }?;

        Ok(())
    }
}