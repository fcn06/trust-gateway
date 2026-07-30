// ─────────────────────────────────────────────────────────────
// OAuth Provider Config Loader — Parses providers from TOML.
//
// Reads the active OAuth providers configuration file (usually
// oauth_providers.toml) and parses it into the shared domain models
// defined in oauth_token.rs.
// ─────────────────────────────────────────────────────────────

use crate::oauth_token::OAuthProviderConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Wrapper for the list of providers in `oauth_providers.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvidersFile {
    pub providers: Vec<OAuthProviderConfig>,
}

impl OAuthProvidersFile {
    /// Load provider configurations from a TOML file.
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let file: Self = toml::from_str(&content)?;
        Ok(file)
    }

    /// Load provider configurations from a TOML string.
    pub fn load_from_str(content: &str) -> anyhow::Result<Self> {
        let file: Self = toml::from_str(content)?;
        Ok(file)
    }

    /// Convert the list of providers into a lookup map by provider ID.
    pub fn to_map(self) -> HashMap<String, OAuthProviderConfig> {
        self.providers
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect()
    }
}
