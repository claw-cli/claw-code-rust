//! Interactive terminal UI for ClawCR.

mod app;
mod events;
mod input;
mod paste_burst;
mod render;
mod slash;
mod terminal;
mod worker;

use std::path::PathBuf;

use anyhow::Result;

pub use app::AppExit;

/// Immutable configuration used to launch the interactive terminal UI.
pub struct InteractiveTuiConfig {
    /// Model identifier used for requests and shown in the header.
    pub model: String,
    /// Working directory shown in the header and passed to the session.
    pub cwd: PathBuf,
    /// Environment overrides applied to the spawned stdio server process.
    pub server_env: Vec<(String, String)>,
    /// Optional prompt submitted immediately after the UI opens.
    pub startup_prompt: Option<String>,
}

/// Runs the interactive alternate-screen terminal UI until the user exits.
pub async fn run_interactive_tui(config: InteractiveTuiConfig) -> Result<AppExit> {
    app::TuiApp::run(config).await
}
