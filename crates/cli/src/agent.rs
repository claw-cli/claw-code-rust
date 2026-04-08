use anyhow::Result;
use clawcr_core::{BuiltinModelCatalog, ProviderKind};
use clawcr_tui::{run_interactive_tui, InteractiveTuiConfig, SavedModelEntry};

use crate::config;

/// Runs the interactive coding-agent entrypoint.
pub async fn run_agent(force_onboarding: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let model_catalog = BuiltinModelCatalog::load()?;
    let stored_config = config::load_config().unwrap_or_default();
    let onboarding_mode = force_onboarding
        || (stored_config.default_provider.is_none()
            && stored_config.anthropic.is_empty()
            && stored_config.openai.is_empty()
            && stored_config.ollama.is_empty());

    let resolved = resolve_initial_provider_settings();
    let saved_models = config::profile_for_provider(&stored_config, resolved.provider)
        .models
        .iter()
        .map(|model| SavedModelEntry {
            model: model.model.clone(),
            base_url: model.base_url.clone(),
            api_key: model.api_key.clone(),
        })
        .collect();

    let server_env = server_env_overrides(&resolved);
    let config::ResolvedProviderSettings {
        provider,
        model,
        base_url: _,
        api_key: _,
    } = resolved;

    run_interactive_tui(InteractiveTuiConfig {
        model,
        provider,
        cwd,
        server_env,
        startup_prompt: None,
        model_catalog,
        saved_models,
        show_model_onboarding: onboarding_mode,
    })
    .await
    .map(|_| ())
}

fn resolve_initial_provider_settings() -> config::ResolvedProviderSettings {
    config::resolve_provider_settings().unwrap_or_else(|err| {
        eprintln!("warning: failed to resolve provider settings: {err}");
        default_provider_settings()
    })
}

fn default_provider_settings() -> config::ResolvedProviderSettings {
    config::ResolvedProviderSettings {
        provider: ProviderKind::Openai,
        model: "gpt-4o".to_string(),
        base_url: Some("https://api.openai.com/v1".to_string()),
        api_key: None,
    }
}

fn server_env_overrides(resolved: &config::ResolvedProviderSettings) -> Vec<(String, String)> {
    let mut env = vec![
        (
            "CLAWCR_PROVIDER".to_string(),
            resolved.provider.as_str().to_string(),
        ),
        ("CLAWCR_MODEL".to_string(), resolved.model.clone()),
    ];
    if let Some(base_url) = &resolved.base_url {
        env.push(("CLAWCR_BASE_URL".to_string(), base_url.clone()));
    }
    if let Some(api_key) = &resolved.api_key {
        env.push(("CLAWCR_API_KEY".to_string(), api_key.clone()));
    }
    env
}
