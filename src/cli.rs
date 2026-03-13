use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "colporteur",
    about = "Convert email newsletters into Atom feeds",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[arg(long, global = true, help = "Output in JSON format")]
    pub json: bool,

    #[arg(
        long,
        short = 'q',
        global = true,
        conflicts_with = "verbose",
        help = "Suppress all output except errors"
    )]
    pub quiet: bool,

    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count, help = "Increase verbosity (-v info, -vv debug)")]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "Fetch new emails and update Atom feed files")]
    Fetch(FetchArgs),
    #[command(about = "Test IMAP connection(s)")]
    Test(TestArgs),
    #[command(about = "List configured feeds and their sync state")]
    List,
    #[command(about = "Create a sample config file to get started")]
    Init,
    #[command(about = "Scan IMAP account(s) and list unique sender addresses")]
    Scan(ScanArgs),
    #[command(about = "Export configured feeds as an OPML file")]
    ExportOpml(ExportOpmlArgs),
}

#[derive(Args, Debug)]
pub struct ExportOpmlArgs {
    #[arg(long, value_name = "URL", help = "Base URL for feed links")]
    pub base_url: String,
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Output file (default: stdout)"
    )]
    pub output: Option<String>,
}

#[derive(Args, Debug)]
pub struct FetchArgs {
    #[arg(long, value_name = "FEED", help = "Process only this feed")]
    pub feed: Option<String>,
    #[arg(long, help = "Preview without writing files or updating state")]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct TestArgs {
    #[arg(long, value_name = "ACCOUNT", help = "Test only this account")]
    pub account: Option<String>,
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    #[arg(long, value_name = "ACCOUNT", help = "Scan only this account")]
    pub account: Option<String>,
}
