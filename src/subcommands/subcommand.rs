use anyhow::Result;
use clap::ArgMatches;

pub trait Subcommand {
    async fn execute(&mut self, arg_matches: &ArgMatches) -> Result<()>;
}