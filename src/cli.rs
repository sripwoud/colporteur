use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "colporteur",
    about = "Convert email newsletters into Atom feeds",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, short = 'q', global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Fetch(FetchArgs),
    Test(TestArgs),
    List,
}

#[derive(Args, Debug)]
pub struct FetchArgs {
    #[arg(long, value_name = "FEED")]
    pub feed: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct TestArgs {
    #[arg(long, value_name = "ACCOUNT")]
    pub account: Option<String>,
}
