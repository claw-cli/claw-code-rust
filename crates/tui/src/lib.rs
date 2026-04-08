//! Interactive terminal UI for ClawCR.
//!
//! This module is the public entry point for launching the full-screen TUI.

mod app;
mod events;
mod input;
mod onboarding_config;
mod paste_burst;
mod render;
mod slash;
mod terminal;
mod worker;

pub use app::run_interactive_tui;
pub use app::AppExit;
pub use app::InteractiveTuiConfig;
pub use events::SavedModelEntry;

#[cfg(test)]
mod tests;
