//! The agent as a command-line program: `streamphone-agent` runs it in a
//! console, `streamphone-agent hook` / `report` talk to one that is running.

use anyhow::Result;
use streamphone_agent::{run_cli, serve, Options, CLI_HELP};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if run_cli(&args) {
        return Ok(());
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("streamphone-agent [--config <path>] [--port <number>]

{CLI_HELP}");
        return Ok(());
    }

    tracing_subscriber::fmt().with_target(false).init();
    serve(Options::from_args(&args)?, None).await
}
