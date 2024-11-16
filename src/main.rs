use clap::{Command, Arg};

mod commands;
mod credential_manager;

use commands::{Subcommand, LoginCommand};
use credential_manager::CredentialManager;

fn cli() -> Command {
    Command::new("git-lfs-synology")
        .about("This is an implementation of a git lfs custom transfer agent. See https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md for more information.")
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("login")
                .about("Allows logging into the Synology NAS.")
                .args_conflicts_with_subcommands(true)
                .flatten_help(true)
                .arg(
                    Arg::new("USER")
                        .short('u')
                        .long("user")
                        .required(true)
                        .help("The username for the Synology NAS")
                )
                .arg(
                    Arg::new("URL")
                        .short('l')
                        .long("url")
                        .required(true)
                        .help("The URL for the Synology NAS")
                )
        )
        .subcommand(
            Command::new("logout")
            .about("Deletes a login for the Synology NAS.")
            .args_conflicts_with_subcommands(true)
            .flatten_help(true)
            .arg(
                Arg::new("URL")
                    .short('l')
                    .long("url")
                    .required(true)
                    .help("The URL for the Synology NAS")
            )
        )
}

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

fn logout(url: &str) {
    let credential_manager = CredentialManager { };

    if credential_manager.has_credential(url) {
        credential_manager.remove_credential(url);
    }
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("login", sub_matches)) => {
            let login_command = LoginCommand { };
            login_command.execute(sub_matches);
        },
        Some(("logout", _)) => {
            // logout();
        }
        _ => println!("No subcommand")
    }
}
