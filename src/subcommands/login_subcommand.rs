use anyhow::{Context, Result};
use clap::ArgMatches;

use crate::credential_manager::{Credential, CredentialManager};
use crate::synology_api::SynologyFileStation;

use super::Subcommand;

#[derive(Debug)]
pub struct LoginSubcommand {
}

impl Subcommand for LoginSubcommand {
    #[tracing::instrument]
    async fn execute(&mut self, arg_matches: &ArgMatches) -> Result<()> {
        let url = arg_matches.get_one::<String>("URL").context("URL not provided.")?;
        let user = arg_matches.get_one::<String>("USER").context("USER not provided.")?;

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

        let credential = Credential::new(
            user.clone(),
            password.clone());

        let mut file_station = SynologyFileStation::new(url);
        file_station.login(&credential).await?;
        
        credential_manager.set_credential(url, &credential)?;

        Ok(())
    }
}