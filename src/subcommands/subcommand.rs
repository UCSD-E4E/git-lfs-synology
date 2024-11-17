use anyhow::Result;
use clap::ArgMatches;

pub trait Subcommand {
    fn execute(&self, arg_matches: &ArgMatches) -> Result<()>;
}