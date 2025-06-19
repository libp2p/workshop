use anyhow::Result;
use clap::Parser;
use workshop::{App, Log};

#[derive(Parser)]
#[command(name = "workshop")]
#[command(about = "A tool for presenting programming workshops")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(disable_version_flag = true)]
struct Args {
    #[arg(long, help = "Install a workshop from a URL")]
    install: Option<String>,

    #[arg(long, help = "Show version information")]
    version: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --version flag
    if args.version {
        println!("workshop v{}", env!("CARGO_PKG_VERSION"));
        println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        return Ok(());
    }

    // initialize the logger
    let from_logger = Log::init(Some("log.txt"))?;

    // Initialize the app
    let mut app = App::new(from_logger)?;

    // run the app
    let app_handle = tokio::spawn(async move { app.run(args.install).await });

    // Wait for the app to finish
    let app_result = app_handle.await?;

    // Check for errors
    if let Err(e) = app_result {
        eprintln!("App error: {e}");
    }

    Ok(())
}
