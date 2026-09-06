//! Application configuration loaded from a TOML file at startup.

/// Chapter artwork upload configuration.
pub mod artwork;
/// HTTP server configuration.
pub mod http;
/// Image upload configuration.
pub mod image;

#[cfg(test)]
mod tests;

use anyhow::Context as _;
use serde::Deserialize;

use crate::config::artwork::ArtworkConfig;
use crate::config::http::HttpConfig;
use crate::config::image::ImageConfig;

// Runtime configuration file loaded from the current working directory.
const DEFAULT_FILE_NAME: &str = "app_config.toml";

/// Runtime application configuration.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    //
    /// HTTP server configuration.
    pub http: HttpConfig,

    /// Image upload configuration.
    pub image: ImageConfig,

    /// Chapter artwork upload limits.
    #[serde(default)]
    pub artwork: ArtworkConfig,
}

impl AppConfig {
    /// Reads the application configuration from `app_config.toml` in the
    /// current working directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or its TOML is invalid.
    pub async fn from_default_file() -> anyhow::Result<Self> {
        //
        let content = tokio::fs::read_to_string(DEFAULT_FILE_NAME)
            .await
            .inspect_err(|error| {
                //
                tracing::error!(
                    operation = "read_application_config",
                    sdk_err = ?error,
                    "Tokio SDK file read error",
                );
            })
            .with_context(
                || "[ApplicationConfig::from_default_file] Failed to read app_config.toml",
            )?;

        Self::parse(&content)
            .inspect_err(|error| {
                //
                tracing::error!(
                    operation = "parse_application_config",
                    config_err = ?error,
                    "application configuration parsing or validation failed",
                );
            })
            .with_context(
                || "[ApplicationConfig::from_default_file] Failed to parse app_config.toml",
            )
    }

    // Parses and validates runtime application configuration.
    fn parse(content: &str) -> anyhow::Result<Self> {
        //
        let config = toml::from_str::<Self>(content)?;

        config.image.validate()?;

        config.artwork.validate()?;

        Ok(config)
    }
}
