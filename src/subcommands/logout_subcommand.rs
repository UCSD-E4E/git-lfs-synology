use crate::subcommands::subcommand::Subcommand;
use crate::credential_manager::CredentialManager;

pub struct LogoutSubcommand {

}

impl LogoutSubcommand {
    fn logout(url: &str) {
        let credential_manager = CredentialManager { };
    
        if credential_manager.has_credential(url) {
            credential_manager.remove_credential(url);
        }
    }
}

impl Subcommand for LogoutSubcommand {
    fn execute(&self, arg_matches: &clap::ArgMatches) {
        
    }
}
