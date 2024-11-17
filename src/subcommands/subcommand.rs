use clap::ArgMatches;

pub trait Subcommand {
    fn execute(&self);
    fn parse_args(&mut self, arg_matches: &ArgMatches) -> Option<()>;
}