use crate::subcommands::subcommand::Subcommand;
use crate::credential_manager::CredentialManager;

pub struct LogoutSubcommand {
    url: String
}

impl LogoutSubcommand {
    pub fn new() -> LogoutSubcommand {
        // Intentionally pass an empty string instead of using Some.
        // Url is required and will be handled by parsing.
        // We will still check during execution to see if the url is set just in case.
        LogoutSubcommand {
            url: "".to_string()
        }
    }
}

impl Subcommand for LogoutSubcommand {
    fn execute(&self) {
        let url = self.url.as_str();
        let credential_manager = CredentialManager { };
    
        // if credential_manager.has_credential(url) {
        //     credential_manager.remove_credential(url);
        // }
    }

    fn parse_args(&mut self, arg_matches: &clap::ArgMatches) -> Option<()> {
        let url = arg_matches.get_one::<String>("URL")?;
        self.url = url.clone();

        Some(())
    }
}
