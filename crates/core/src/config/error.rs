use std::path::PathBuf;

/// Enumerates failures that can occur while loading or validating app config.
#[derive(Debug, thiserror::Error)]
pub enum AppConfigError {
    /// Reading a config file from disk failed.
    #[error("config IO failed at {path}: {source}")]
    Io {
        /// The config path that failed to read.
        path: PathBuf,
        /// The underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// Parsing TOML into the config schema failed.
    #[error("config parse failed at {path}: {message}")]
    Parse { path: PathBuf, message: String },
    /// Cross-field validation rejected the normalized config.
    #[error("invalid app config: {message}")]
    Validation { message: String },
}
