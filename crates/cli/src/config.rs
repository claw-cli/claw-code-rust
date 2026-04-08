use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clawcr_core::ProviderKind;
use clawcr_utils::{current_user_config_file, FileSystemConfigPathResolver};
use serde::{Deserialize, Serialize};

/// One model entry stored under a provider section in `config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfiguredModel {
    /// The model slug or custom model name.
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// One provider-specific configuration block that can store many model entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ConfiguredModel>,
}

impl ProviderProfile {
    pub(crate) fn is_empty(&self) -> bool {
        self.default_model.is_none()
            && self.base_url.is_none()
            && self.api_key.is_none()
            && self.models.is_empty()
    }
}

/// Persisted provider configuration grouped by provider family.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<ProviderKind>,
    #[serde(default, skip_serializing_if = "ProviderProfile::is_empty")]
    pub anthropic: ProviderProfile,
    #[serde(default, skip_serializing_if = "ProviderProfile::is_empty")]
    pub openai: ProviderProfile,
    #[serde(default, skip_serializing_if = "ProviderProfile::is_empty")]
    pub ollama: ProviderProfile,
}

/// The fully-resolved provider settings that can be forwarded to a server process.
pub struct ResolvedProviderSettings {
    /// Normalized provider name.
    pub provider: ProviderKind,
    /// Final model identifier.
    pub model: String,
    /// Optional provider base URL override.
    pub base_url: Option<String>,
    /// Optional provider API key override.
    pub api_key: Option<String>,
}

// ---------------------------------------------------------------------------
// Config file I/O
// ---------------------------------------------------------------------------

/// `~/.clawcr/config.toml`
pub fn config_path() -> Result<PathBuf> {
    current_user_config_file().context("could not determine user config path")
}

/// The previous JSON location under the current `.clawcr` directory.
fn legacy_json_config_path() -> Result<PathBuf> {
    let resolver = FileSystemConfigPathResolver::from_env()
        .context("could not determine home directory for legacy config path")?;
    Ok(resolver.user_config_dir().join("config.json"))
}

/// The older pre-spec JSON location used by early CLI builds.
fn legacy_cli_config_path() -> Result<PathBuf> {
    let resolver = FileSystemConfigPathResolver::from_env()
        .context("could not determine home directory for legacy config path")?;
    Ok(resolver
        .user_config_dir()
        .parent()
        .expect("config dir should have a parent home directory")
        .join(".claw-code-rust")
        .join("config.json"))
}

/// Load a JSON config file from one of the legacy locations.
fn load_legacy_json_config(path: &Path) -> Result<AppConfig> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let legacy: LegacyFlatAppConfig = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(legacy.into_app_config())
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    if path.exists() {
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let value: toml::Value =
            toml::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))?;
        let table = value.as_table().cloned().unwrap_or_default();
        if table.contains_key("default_provider")
            || table.contains_key("anthropic")
            || table.contains_key("openai")
            || table.contains_key("ollama")
            || table.contains_key("models")
            || table.contains_key("default_model")
        {
            let parsed_new: Result<AppConfig, toml::de::Error> = value.clone().try_into();
            if let Ok(config) = parsed_new {
                return Ok(config);
            }
            let legacy_provider_cfg: LegacySectionAppConfig = value
                .try_into()
                .with_context(|| format!("failed to parse {}", path.display()))?;
            return Ok(legacy_provider_cfg.into_app_config());
        }

        let legacy_cfg: LegacyFlatAppConfig = value
            .try_into()
            .with_context(|| format!("failed to parse {}", path.display()))?;
        return Ok(legacy_cfg.into_app_config());
    }

    let legacy_json_path = legacy_json_config_path()?;
    if legacy_json_path.exists() {
        let cfg = load_legacy_json_config(&legacy_json_path)?;
        save_config(&cfg)?;
        return Ok(cfg);
    }

    let legacy_cli_path = legacy_cli_config_path()?;
    if legacy_cli_path.exists() {
        let cfg = load_legacy_json_config(&legacy_cli_path)?;
        save_config(&cfg)?;
        return Ok(cfg);
    }

    Ok(AppConfig::default())
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let toml = toml::to_string_pretty(config)?;
    std::fs::write(&path, toml).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Env-var detection
// ---------------------------------------------------------------------------

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Build a partial config from environment variables.
fn env_config() -> AppConfig {
    let anthropic_api_key =
        env_non_empty("ANTHROPIC_API_KEY").or_else(|| env_non_empty("ANTHROPIC_AUTH_TOKEN"));
    let anthropic_base_url = env_non_empty("ANTHROPIC_BASE_URL");
    let openai_api_key = env_non_empty("OPENAI_API_KEY");
    let openai_base_url = env_non_empty("OPENAI_BASE_URL");

    let default_provider = if anthropic_api_key.is_some() || anthropic_base_url.is_some() {
        Some(ProviderKind::Anthropic)
    } else if openai_api_key.is_some() || openai_base_url.is_some() {
        Some(ProviderKind::Openai)
    } else {
        None
    };

    AppConfig {
        default_provider,
        anthropic: ProviderProfile {
            base_url: anthropic_base_url,
            api_key: anthropic_api_key,
            default_model: None,
            models: Vec::new(),
        },
        openai: ProviderProfile {
            base_url: openai_base_url,
            api_key: openai_api_key,
            default_model: None,
            models: Vec::new(),
        },
        ollama: ProviderProfile::default(),
    }
}

// ---------------------------------------------------------------------------
// Provider resolution: env vars > config file > onboarding
// ---------------------------------------------------------------------------

/// Resolves provider settings without constructing a local provider instance.
pub fn resolve_provider_settings() -> Result<ResolvedProviderSettings> {
    let env = env_config();
    let file = load_config().unwrap_or_default();

    let provider_name = env
        .default_provider
        .or(file.default_provider)
        .or_else(|| infer_default_provider(&file));

    if let Some(provider_name) = provider_name {
        let selected_profile = profile_for_provider(&file, provider_name);
        let env_profile = profile_for_provider(&env, provider_name);
        let model = selected_profile
            .default_model
            .clone()
            .or_else(|| {
                selected_profile
                    .models
                    .first()
                    .map(|model| model.model.clone())
            })
            .or_else(|| env_profile.default_model.clone())
            .or_else(|| env_profile.models.first().map(|model| model.model.clone()))
            .unwrap_or_else(|| default_model_for_provider(provider_name));
        let base_url = selected_profile
            .base_url
            .clone()
            .or_else(|| env_profile.base_url.clone());
        let api_key = selected_profile
            .api_key
            .clone()
            .or_else(|| env_profile.api_key.clone());

        return Ok(ResolvedProviderSettings {
            model,
            provider: provider_name,
            base_url,
            api_key,
        });
    }

    anyhow::bail!(
        "No provider configured. Set ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN, \
         or run `clawcr onboard` to complete setup."
    )
}

fn default_model_for_provider(provider: ProviderKind) -> String {
    match provider {
        ProviderKind::Anthropic => "claude-sonnet-4-20250514".to_string(),
        ProviderKind::Ollama => "qwen3.5:9b".to_string(),
        ProviderKind::Openai => "gpt-4o".to_string(),
    }
}

fn parse_provider_kind(value: &str) -> Result<ProviderKind> {
    match value.to_ascii_lowercase().as_str() {
        "anthropic" => Ok(ProviderKind::Anthropic),
        "openai" => Ok(ProviderKind::Openai),
        "ollama" => Ok(ProviderKind::Ollama),
        other => anyhow::bail!("unknown provider `{other}`"),
    }
}

fn infer_default_provider(config: &AppConfig) -> Option<ProviderKind> {
    if !config.anthropic.is_empty() {
        Some(ProviderKind::Anthropic)
    } else if !config.openai.is_empty() {
        Some(ProviderKind::Openai)
    } else if !config.ollama.is_empty() {
        Some(ProviderKind::Ollama)
    } else {
        None
    }
}

pub fn profile_for_provider(config: &AppConfig, provider: ProviderKind) -> &ProviderProfile {
    match provider {
        ProviderKind::Anthropic => &config.anthropic,
        ProviderKind::Openai => &config.openai,
        ProviderKind::Ollama => &config.ollama,
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyFlatAppConfig {
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

impl LegacyFlatAppConfig {
    fn into_app_config(self) -> AppConfig {
        let provider = self
            .provider
            .and_then(|value| parse_provider_kind(&value).ok());
        let model = self.model.unwrap_or_else(|| {
            provider
                .map(default_model_for_provider)
                .unwrap_or_else(|| default_model_for_provider(ProviderKind::Anthropic))
        });
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let profile = ProviderProfile {
            default_model: Some(model.clone()),
            base_url: base_url.clone(),
            api_key: api_key.clone(),
            models: vec![ConfiguredModel {
                model,
                base_url,
                api_key,
            }],
        };
        let default_provider = provider.or_else(|| {
            if profile.api_key.is_some()
                || profile.base_url.is_some()
                || profile.default_model.is_some()
                || !profile.models.is_empty()
            {
                Some(ProviderKind::Anthropic)
            } else {
                None
            }
        });

        match default_provider {
            Some(ProviderKind::Anthropic) => AppConfig {
                default_provider,
                anthropic: profile,
                openai: ProviderProfile::default(),
                ollama: ProviderProfile::default(),
            },
            Some(ProviderKind::Openai) => AppConfig {
                default_provider,
                anthropic: ProviderProfile::default(),
                openai: profile,
                ollama: ProviderProfile::default(),
            },
            Some(ProviderKind::Ollama) => AppConfig {
                default_provider,
                anthropic: ProviderProfile::default(),
                openai: ProviderProfile::default(),
                ollama: profile,
            },
            None => AppConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacySectionAppConfig {
    #[serde(default)]
    default_provider: Option<ProviderKind>,
    #[serde(default)]
    anthropic: LegacySectionProviderProfile,
    #[serde(default)]
    openai: LegacySectionProviderProfile,
    #[serde(default)]
    ollama: LegacySectionProviderProfile,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LegacySectionProviderProfile {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

impl LegacySectionAppConfig {
    fn into_app_config(self) -> AppConfig {
        AppConfig {
            default_provider: self.default_provider,
            anthropic: legacy_section_profile_into_provider_profile(self.anthropic),
            openai: legacy_section_profile_into_provider_profile(self.openai),
            ollama: legacy_section_profile_into_provider_profile(self.ollama),
        }
    }
}

fn legacy_section_profile_into_provider_profile(
    legacy: LegacySectionProviderProfile,
) -> ProviderProfile {
    let model = legacy.model.clone();
    ProviderProfile {
        default_model: model.clone(),
        base_url: legacy.base_url.clone(),
        api_key: legacy.api_key.clone(),
        models: model
            .map(|model| ConfiguredModel {
                model,
                base_url: legacy.base_url,
                api_key: legacy.api_key,
            })
            .into_iter()
            .collect(),
    }
}
