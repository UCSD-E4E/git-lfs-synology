use clap::ArgMatches;

use crate::subcommands::Subcommand;
use crate::credential_manager::CredentialManager;

pub struct LoginSubcommand {
    
}

impl LoginSubcommand {
    fn login(user: &str, url: &str) {
        let credential_manager = CredentialManager { };

        if credential_manager.has_credential(url) {
            // get password and totp command
        }
        else {
            // get password and totop command
        }

        // try login
        // if success store

        // else throw error
    }
}

impl Subcommand for LoginSubcommand {
    fn execute(&self, arg_matches: &ArgMatches) {

    }
}