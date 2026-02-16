//! CWR CLI - Command line tool for querying CDEC water and snow data.

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "cwr-cli",
    version,
    about = "California Water Reservoir data toolkit"
)]
struct Cli {
    #[command(subcommand)]
    command: cwr_cmd::Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    cwr_cmd::run(cli.command).await
}
