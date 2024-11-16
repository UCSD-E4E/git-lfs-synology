use clap::{Command, Arg};

fn cli() -> Command {
    Command::new("git-lfs-synology")
        .about("This is an implementation of a git lfs custom transfer agent. See https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md for mor information.")
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
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("login", sub_matches)) => {
            println!("{:?}", sub_matches);

            println!("login")
        },
        _ => println!("No subcommand")
    }
}
