use anyhow::Result;
use workshop::{ui::tui::Ui, Config, Log};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // initialize the logger
    let from_logger = Log::init(Some("log.txt"))?;

    // Load the configuration
    let config = Config::load()?;

    // Initialize the ui
    let mut ui = Ui::new(from_logger, config);

    // run the ui
    let ui_handle = tokio::spawn(async move { ui.run().await });

    // Wait for the engine and ui to finish
    let ui_result = ui_handle.await?;

    // Check for errors
    if let Err(e) = ui_result {
        eprintln!("UI error: {}", e);
    }

    Ok(())
}
