use anyhow::Result;
use engine::{Engine, Log, Message};
use workshop::{ui::tui::Ui, Config};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // initialize the logger
    let from_logger = Log::init()?;

    // Load the configuration
    let config = Config::load()?;

    // Get the present working directory
    let pwd = std::env::current_dir()?;

    // Create the message channels
    let (to_engine, from_ui) = tokio::sync::mpsc::channel::<Message>(100);
    let (to_ui, from_engine) = tokio::sync::mpsc::channel::<Message>(100);

    // Initialize the engine
    let mut engine = Engine::new(to_ui, from_ui)?;

    // Initialize the ui
    let mut ui = Ui::new(to_engine, from_engine, from_logger, config, &pwd)?;

    // run the engine and ui in parallel
    let engine_handle = tokio::spawn(async move { engine.run().await });
    let ui_handle = tokio::spawn(async move { ui.run().await });

    // Wait for the engine and ui to finish
    let (engine_result, ui_result) = tokio::try_join!(engine_handle, ui_handle)?;

    // Check for errors
    if let Err(e) = engine_result {
        eprintln!("Engine error: {}", e);
    }
    if let Err(e) = ui_result {
        eprintln!("UI error: {}", e);
    }

    Ok(())
}
