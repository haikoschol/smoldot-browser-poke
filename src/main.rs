use anyhow::{Context, Result};
use notify::{Event, RecursiveMode, Watcher};
use std::env;
use std::path::Path;
use std::time::Duration;
use thirtyfour::prelude::*;
use tokio::fs;
use tokio::sync::mpsc;
use tokio::time::sleep;

/// Main asynchronous function to set up file watching and handle events.
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Get file path from command-line arguments.
    let path_to_watch = env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("Usage: please provide a file path as a command-line argument.")
    })?;
    println!("Watching file: {}", &path_to_watch);

    // 2. Create a channel to communicate file change events.
    let (tx, mut rx) = mpsc::channel(1);

    // 3. Create and configure the file watcher.
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        match res {
            Ok(event) if event.kind.is_modify() => {
                if tx.try_send(()).is_err() {}
            }
            Ok(_) => {}
            Err(e) => eprintln!("Watch error: {:?}", e),
        }
    })?;

    // Start watching the specified path.
    watcher
        .watch(Path::new(&path_to_watch), RecursiveMode::NonRecursive)
        .with_context(|| format!("Failed to start watching file: {}", path_to_watch))?;

    println!("Watcher started. Waiting for file changes... (Press Ctrl+C to exit)");

    // 4. Main event loop.
    while let Some(_) = rx.recv().await {
        println!("\nFile change detected!");
        sleep(Duration::from_millis(100)).await;
        match fs::read_to_string(&path_to_watch).await {
            Ok(contents) => {
                println!(
                    "Read {} bytes from file. Running browser automation...",
                    contents.len()
                );
                if let Err(e) = run_automation(&contents).await {
                    eprintln!("Automation failed: {:#}", e);
                } else {
                    println!("Automation completed successfully.");
                }
            }
            Err(e) => {
                eprintln!("Failed to read file '{}': {:#}", path_to_watch, e);
            }
        }
    }

    Ok(())
}

/// Performs the browser automation using thirtyfour.
async fn run_automation(content: &str) -> Result<()> {
    // PREREQUISITE:
    // This program requires TWO things to be running beforehand:
    //
    // 1. A running instance of Google Chrome started with the remote debugging port enabled.
    //    e.g., on macOS: /Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222
    //
    // 2. A running instance of chromedriver (downloaded from the Chrome for Testing dashboard).
    //    e.g., ./chromedriver --port=9515

    let mut caps = DesiredCapabilities::chrome();
    caps.set_debugger_address("localhost:9222")
        .context("Failed to set debugger address for Chrome options")?;

    let driver = WebDriver::new("http://localhost:9515", caps)
        .await
        .context("Failed to connect to chromedriver. Is it running on port 9515?")?;

    // driver.new_tab().await.context("Failed to open a new tab")?;

    let url = "http://localhost:8082";
    println!("Navigating to {}", url);
    driver.goto(url).await.with_context(|| format!("Failed to navigate to URL: {}", url))?;

    println!("Looking for input field with ID 'peerAddress'...");
    let peer_address_input = driver
        .find(By::Id("peerAddress"))
        .await
        .context("Could not find input field with ID 'peerAddress'")?;

    peer_address_input.clear().await.context("Failed to clear input field")?;
    peer_address_input
        .send_keys(content)
        .await
        .context("Failed to send keys to input field")?;
    println!("Entered content into the input field.");

    println!("Looking for button with ID 'runDemo'...");
    let run_demo_button = driver
        .find(By::Id("runDemo"))
        .await
        .context("Could not find button with ID 'runDemo'")?;

    run_demo_button.click().await.context("Failed to click the 'runDemo' button")?;
    println!("Clicked the 'runDemo' button.");

    Ok(())
}


