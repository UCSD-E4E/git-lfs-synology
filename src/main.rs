use anyhow::Result;
use clap::{Command, Arg};
use config_dir::get_config_dir;
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::MakeWriterExt;

mod config_dir;
mod subcommands;
mod credential_manager;
mod synology_file_station;

use subcommands::{LoginSubcommand, LogoutSubcommand, Subcommand};

fn setup_logging() -> Result<()> {
    let config_path = get_config_dir()?;
    let log_file = rolling::daily(config_path, "log").with_max_level(tracing::Level::INFO);

    tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_writer(log_file)
        .init();

    Ok(())
}

#[tracing::instrument]
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
                .arg(
                    Arg::new("TOTP_COMMAND")
                        .short('c')
                        .long("totp-command")
                        .help("A command which generates a TOTP code required to log into the Synology NAS.")
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

fn main() -> Result<()> {
    setup_logging()?;

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("login", sub_matches)) => {
            let login_command = LoginSubcommand { };
            login_command.execute(sub_matches)?;

            Ok(())
        },
        Some(("logout", sub_matches)) => {
            let logout_command = LogoutSubcommand { };
            logout_command.execute(sub_matches)?;

            Ok(())
        }
        _ => Ok(())
    }
}
