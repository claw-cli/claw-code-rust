use std::io::{self, BufRead, Write};

use anyhow::Result;

use clawcr_core::ProviderKind;

use crate::config::{AppConfig, ProviderProfile};

/// Run the first-time interactive setup wizard.
///
/// Asks the user to pick a provider and enter credentials.
/// Saves the result to `~/.clawcr/config.toml`.
pub fn run_onboarding() -> Result<AppConfig> {
    println!("╔══════════════════════════════════════════╗");
    println!("║      Welcome to Claw RS!                 ║");
    println!("║   Let's set up your AI provider.         ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    println!("Choose a provider:");
    println!("  [1] Anthropic API  (Claude models)");
    println!("  [2] Ollama         (local models)");
    println!("  [3] OpenAI-compatible (any OpenAI-format API)");
    println!();

    let choice = prompt_choice("Provider [1/2/3]", 1, 3)?;

    let config = match choice {
        1 => setup_anthropic()?,
        2 => setup_ollama()?,
        3 => setup_openai_compat()?,
        _ => unreachable!(),
    };

    crate::config::save_config(&config)?;
    println!();
    println!("Config saved. You can change it later by editing ~/.clawcr/config.toml");
    println!("or by setting environment variables (ANTHROPIC_API_KEY, etc.).");
    println!();

    Ok(config)
}

fn setup_anthropic() -> Result<AppConfig> {
    println!();
    println!("Anthropic API setup");
    println!("-------------------");
    println!("You need an API key from https://console.anthropic.com/");
    println!("(or set ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN env var)");
    println!();

    let api_key = prompt_string("API key")?;
    let base_url = prompt_optional("Custom base URL (leave empty for default)")?;
    let model = choose_model("claude-sonnet-4-20250514")?;

    Ok(AppConfig {
        default_provider: Some(ProviderKind::Anthropic),
        anthropic: ProviderProfile {
            model: Some(model),
            base_url,
            api_key: Some(api_key),
        },
        openai: ProviderProfile::default(),
        ollama: ProviderProfile::default(),
    })
}

fn setup_ollama() -> Result<AppConfig> {
    println!();
    println!("Ollama setup");
    println!("------------");
    println!("Make sure Ollama is running locally.");
    println!();

    let base_url = prompt_with_default("Ollama URL", "http://localhost:11434")?;
    let model = choose_model("qwen3.5:9b")?;

    Ok(AppConfig {
        default_provider: Some(ProviderKind::Ollama),
        anthropic: ProviderProfile::default(),
        openai: ProviderProfile::default(),
        ollama: ProviderProfile {
            model: Some(model),
            base_url: Some(base_url),
            api_key: None,
        },
    })
}

fn setup_openai_compat() -> Result<AppConfig> {
    println!();
    println!("OpenAI-compatible API setup");
    println!("--------------------------");
    println!();

    let base_url = prompt_string("Base URL (e.g. https://api.openai.com)")?;
    let api_key = prompt_optional("API key (leave empty if not required)")?;
    let model = choose_model("gpt-4o")?;

    Ok(AppConfig {
        default_provider: Some(ProviderKind::Openai),
        anthropic: ProviderProfile::default(),
        openai: ProviderProfile {
            model: Some(model),
            base_url: Some(base_url),
            api_key,
        },
        ollama: ProviderProfile::default(),
    })
}

fn choose_model(default_model: &str) -> Result<String> {
    println!();
    println!("Choose a model mode:");
    println!("  [1] Built-in default");
    println!("  [2] Custom model");
    println!();

    match prompt_choice("Model mode [1/2]", 1, 2)? {
        1 => Ok(default_model.to_string()),
        2 => prompt_with_default("Model", default_model),
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Prompt helpers
// ---------------------------------------------------------------------------

fn prompt_choice(prompt: &str, min: u32, max: u32) -> Result<u32> {
    let stdin = io::stdin();
    loop {
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let trimmed = line.trim();

        if let Ok(n) = trimmed.parse::<u32>() {
            if n >= min && n <= max {
                return Ok(n);
            }
        }
        println!("Please enter a number between {} and {}.", min, max);
    }
}

fn prompt_string(prompt: &str) -> Result<String> {
    let stdin = io::stdin();
    loop {
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let trimmed = line.trim().to_string();

        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
        println!("This field is required.");
    }
}

fn prompt_optional(prompt: &str) -> Result<Option<String>> {
    print!("{}: ", prompt);
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();

    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();

    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}
