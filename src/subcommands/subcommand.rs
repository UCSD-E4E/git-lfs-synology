use anyhow::Result;
use clap::ArgMatches;

pub trait Subcommand {
    async fn execute(&self, arg_matches: &ArgMatches) -> Result<()>;
}