use anyhow::Result;
use workshop::{App, Config, Log};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // initialize the logger
    let from_logger = Log::init(Some("log.txt"))?;

    // Load the configuration
    let config = Config::load()?;

    // Initialize the app
    let mut app = App::new(from_logger, config);

    // run the app
    let app_handle = tokio::spawn(async move { app.run().await });

    // Wait for the app to finish
    let app_result = app_handle.await?;

    // Check for errors
    if let Err(e) = app_result {
        eprintln!("App error: {}", e);
    }

    Ok(())
}
