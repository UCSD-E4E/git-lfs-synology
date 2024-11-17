use clap::ArgMatches;

use crate::subcommands::Subcommand;
use crate::credential_manager::CredentialManager;

pub struct LoginSubcommand {
    url: String,
    user: String
}

impl LoginSubcommand {
    pub fn new() -> LoginSubcommand {
        LoginSubcommand {
            url: "".to_string(),
            user: "".to_string()
        }
    }
}

impl Subcommand for LoginSubcommand {
    fn execute(&self) {
        let url = self.url.as_str();
        let user = self.user.as_str();

        let credential_manager = CredentialManager { };

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

    fn parse_args(&mut self, arg_matches: &ArgMatches) -> Option<()> {
        let url = arg_matches.get_one::<String>("URL")?;
        let user = arg_matches.get_one::<String>("USER")?;

        self.url = url.clone();
        self.user = user.clone();

        Some(())
    }
}