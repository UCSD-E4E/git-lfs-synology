use clap::ArgMatches;

use crate::commands::Subcommand;

pub struct LoginCommand {

}

impl Subcommand for LoginCommand {
    fn execute(&self, arg_matches: &ArgMatches) {

    }
}