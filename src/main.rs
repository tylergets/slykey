mod cli;
mod config;
mod core;
mod io;
mod platform;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::config::AppConfig;
use crate::core::engine::Engine;
#[cfg(target_os = "linux")]
use crate::platform::app_indicator;
use crate::platform::x11_rdev::X11RdevBackend;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run(cli.config),
        Commands::ValidateConfig => validate_config(cli.config),
    }
}

fn run(config_path_override: Option<std::path::PathBuf>) -> Result<()> {
    let loaded = AppConfig::load(config_path_override)?;
    let config = loaded.config;
    config.validate()?;

    println!("Loaded config from {}", loaded.path.display());
    println!("Listening on X11 backend (rdev)...");

    #[cfg(target_os = "linux")]
    let _app_indicator = app_indicator::start();

    let backend = Arc::new(X11RdevBackend::new()?);
    let mut engine = Engine::new(config);
    engine.set_output(backend.clone());
    let engine = std::sync::Mutex::new(engine);

    backend.listen(move |event| {
        let mut guard = engine.lock().expect("engine mutex poisoned");
        if let Err(err) = guard.handle_event(event) {
            eprintln!("event handling error: {err}");
        }
    })?;

    Ok(())
}

fn validate_config(config_path_override: Option<std::path::PathBuf>) -> Result<()> {
    let loaded = AppConfig::load(config_path_override)?;
    loaded.config.validate()?;
    println!("Config is valid: {}", loaded.path.display());
    Ok(())
}
