use anyhow::{Context, Result};
use clap::ArgMatches;
use std::io::{self, Write};

use crate::credential_manager::{Credential, CredentialManager};
use crate::synology_api::{SynologyErrorStatus, SynologyFileStation};

use super::Subcommand;

fn get_input(prompt: &str) -> Result<String>{
    print!("{}",prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_goes_into_input_above) => {},
        Err(_no_updates_is_fine) => {},
    }
    Ok(input.trim().to_string())
}

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
        let device_id: Option<String>;
        if credential_manager.has_credential(url)? {
            let credential = credential_manager.get_credential(url)?.context("Credential should not be null")?;
            password = credential.password.clone();
            device_id = credential.device_id;
        }
        else {
            password = rpassword::prompt_password("Synology NAS Password: ")?;
            device_id = None;
        }

        let mut credential = Credential::new(
            user.clone(),
            password.clone());
        credential.device_id = device_id;

        let mut file_station = SynologyFileStation::new(url);
        let credential = match file_station.login(&credential, false, None).await {
            Ok(credential) => Ok(credential),
            Err(error) => match error {
                SynologyErrorStatus::NoTotp => {
                    let totp = get_input("TOTP: ")?;
                    
                    file_station.login(&credential, true, Some(totp)).await
                },
                _ => Err(error)
            }
        }?;
        
        credential_manager.set_credential(url, &credential)?;

        Ok(())
    }
}