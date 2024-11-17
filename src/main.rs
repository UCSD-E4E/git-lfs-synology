use clap::{Command, Arg};

mod subcommands;
mod credential_manager;

use subcommands::{LoginSubcommand, LogoutSubcommand, Subcommand};

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

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("login", sub_matches)) => {
            let mut login_command = LoginSubcommand::new();
            if login_command.parse_args(sub_matches).is_some() {
                login_command.execute();
            }
            else {
                // TODO: We failed processing
            }
        },
        Some(("logout", sub_matches)) => {
            let mut logout_command = LogoutSubcommand::new();
            if logout_command.parse_args(sub_matches).is_some() {
                logout_command.execute();
            }
            else {
                // TODO: We failed processing
            }
        }
        _ => println!("No subcommand")
    }
}
