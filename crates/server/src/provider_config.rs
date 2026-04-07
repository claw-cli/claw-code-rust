use std::{fs, path::Path};

use anyhow::{Context, Result};
use clawcr_core::ProviderKind;
use serde::Deserialize;

use clawcr_provider::{anthropic::AnthropicProvider, openai::OpenAIProvider, ModelProvider};

/// Resolved provider bootstrap owned by the server runtime.
pub struct ResolvedServerProvider {
    /// Concrete provider used for model requests.
    pub provider: std::sync::Arc<dyn ModelProvider>,
    /// Default model slug used when a session or turn does not request one.
    pub default_model: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ProviderProfile {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct AppConfigFile {
    #[serde(default)]
    default_provider: Option<ProviderKind>,
    #[serde(default)]
    anthropic: ProviderProfile,
    #[serde(default)]
    openai: ProviderProfile,
    #[serde(default)]
    ollama: ProviderProfile,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

/// Loads the server-side provider from config and environment variables.
pub fn load_server_provider(
    config_file: &Path,
    default_model: Option<&str>,
) -> Result<ResolvedServerProvider> {
    let file_config = read_provider_config(config_file).unwrap_or_default();
    let env_provider = env_non_empty("CLAWCR_PROVIDER");
    let env_model = env_non_empty("CLAWCR_MODEL");
    let env_base_url = env_non_empty("CLAWCR_BASE_URL");
    let env_api_key = env_non_empty("CLAWCR_API_KEY");

    let provider_name = env_provider
        .as_deref()
        .and_then(parse_provider_kind)
        .or(file_config.default_provider)
        .or_else(|| {
            file_config
                .provider
                .as_deref()
                .and_then(parse_provider_kind)
        })
        .or_else(|| infer_default_provider(&file_config))
        .unwrap_or(ProviderKind::Openai);

    let profile = profile_for_provider(&file_config, provider_name);

    let model = env_model
        .or(profile.model.clone())
        .or(file_config.model.clone())
        .or_else(|| default_model.map(ToOwned::to_owned))
        .unwrap_or_else(|| default_model_for_provider(provider_name));

    let base_url = env_base_url
        .or(profile.base_url.clone())
        .or(file_config.base_url.clone())
        .or_else(|| env_non_empty("ANTHROPIC_BASE_URL"))
        .or_else(|| env_non_empty("OPENAI_BASE_URL"));
    let api_key = env_api_key
        .or(profile.api_key.clone())
        .or(file_config.api_key.clone())
        .or_else(|| env_non_empty("ANTHROPIC_API_KEY"))
        .or_else(|| env_non_empty("ANTHROPIC_AUTH_TOKEN"))
        .or_else(|| env_non_empty("OPENAI_API_KEY"));

    let provider = match provider_name {
        ProviderKind::Anthropic => {
            let api_key = api_key.context("anthropic provider requires an API key")?;
            if let Some(url) = base_url {
                std::sync::Arc::new(AnthropicProvider::new_with_url(api_key, url))
                    as std::sync::Arc<dyn ModelProvider>
            } else {
                std::sync::Arc::new(AnthropicProvider::new(api_key))
            }
        }
        ProviderKind::Ollama | ProviderKind::Openai => {
            let base_url = ensure_openai_v1(&base_url.unwrap_or_else(|| {
                if provider_name == ProviderKind::Ollama {
                    "http://localhost:11434".to_string()
                } else {
                    "https://api.openai.com".to_string()
                }
            }));
            let mut provider = OpenAIProvider::new(base_url);
            if let Some(api_key) = api_key {
                provider = provider.with_api_key(api_key);
            }
            std::sync::Arc::new(provider)
        }
    };

    Ok(ResolvedServerProvider {
        provider,
        default_model: model,
    })
}

fn read_provider_config(config_file: &Path) -> Result<AppConfigFile> {
    if !config_file.exists() {
        return Ok(AppConfigFile::default());
    }
    let contents = fs::read_to_string(config_file)
        .with_context(|| format!("failed to read {}", config_file.display()))?;
    let value: toml::Value =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", config_file.display()))?;
    let table = value.as_table().cloned().unwrap_or_default();
    if table.contains_key("default_provider")
        || table.contains_key("anthropic")
        || table.contains_key("openai")
        || table.contains_key("ollama")
    {
        return value
            .try_into()
            .with_context(|| format!("failed to parse {}", config_file.display()));
    }

    let legacy: AppConfigFile =
        value.try_into().with_context(|| format!("failed to parse {}", config_file.display()))?;
    Ok(legacy)
}

fn profile_for_provider(config: &AppConfigFile, provider: ProviderKind) -> &ProviderProfile {
    match provider {
        ProviderKind::Anthropic => &config.anthropic,
        ProviderKind::Openai => &config.openai,
        ProviderKind::Ollama => &config.ollama,
    }
}

fn infer_default_provider(config: &AppConfigFile) -> Option<ProviderKind> {
    if config.anthropic.model.is_some()
        || config.anthropic.base_url.is_some()
        || config.anthropic.api_key.is_some()
    {
        Some(ProviderKind::Anthropic)
    } else if config.openai.model.is_some()
        || config.openai.base_url.is_some()
        || config.openai.api_key.is_some()
    {
        Some(ProviderKind::Openai)
    } else if config.ollama.model.is_some()
        || config.ollama.base_url.is_some()
        || config.ollama.api_key.is_some()
    {
        Some(ProviderKind::Ollama)
    } else {
        None
    }
}

fn default_model_for_provider(provider: ProviderKind) -> String {
    match provider {
        ProviderKind::Anthropic => "claude-sonnet-4-20250514".to_string(),
        ProviderKind::Ollama => "qwen3.5:9b".to_string(),
        ProviderKind::Openai => "gpt-4o".to_string(),
    }
}

fn parse_provider_kind(value: &str) -> Option<ProviderKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "anthropic" => Some(ProviderKind::Anthropic),
        "openai" => Some(ProviderKind::Openai),
        "ollama" => Some(ProviderKind::Ollama),
        _ => None,
    }
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn ensure_openai_v1(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}
