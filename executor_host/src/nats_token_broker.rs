// ─────────────────────────────────────────────────────────────
// NatsTokenBroker — NATS KV-backed TokenBroker implementation.
//
// Implements the canonical `TokenBroker` trait for the Community
// Edition executor_host. Manages token storage in the `oauth_tokens`
// KV bucket and handles transparent auto-refresh when access tokens
// expire.
// ─────────────────────────────────────────────────────────────

use async_nats::jetstream::kv::Store;
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use trust_core::oauth_token::{OAuthProviderConfig, OAuthToken, TokenBroker, TokenError};

/// NATS KV-backed token broker with inline auto-refresh capabilities.
pub struct NatsTokenBroker {
    kv: Store,
    nats: async_nats::Client,
    http_client: reqwest::Client,
    providers: HashMap<String, OAuthProviderConfig>,
}

impl NatsTokenBroker {
    /// Create a new NatsTokenBroker instance.
    pub fn new(kv: Store, nats: async_nats::Client, http_client: reqwest::Client) -> Self {
        let config_path = std::env::var("OAUTH_PROVIDERS_CONFIG_PATH")
            .unwrap_or_else(|_| "connector_mcp_server/config/oauth_providers.toml".to_string());

        let providers = match trust_core::oauth_provider_config::OAuthProvidersFile::load_from_file(
            &config_path,
        ) {
            Ok(file) => {
                let map = file.to_map();
                tracing::info!(
                    "🔑 Loaded {} OAuth providers from {}",
                    map.len(),
                    config_path
                );
                map
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ Failed to load OAuth providers from {}: {}. Auto-refresh will be disabled.",
                    config_path,
                    e
                );
                HashMap::new()
            }
        };

        Self {
            kv,
            nats,
            http_client,
            providers,
        }
    }

    /// Perform the standard OAuth2 refresh token flow.
    async fn refresh_token_oauth(
        &self,
        provider_config: &OAuthProviderConfig,
        refresh_token: &str,
    ) -> Result<(String, Option<String>, i64), anyhow::Error> {
        let client_id = std::env::var(&provider_config.client_id_env).map_err(|_| {
            anyhow::anyhow!(
                "Client ID environment variable '{}' is not set",
                provider_config.client_id_env
            )
        })?;
        let client_secret = std::env::var(&provider_config.client_secret_env).map_err(|_| {
            anyhow::anyhow!(
                "Client Secret environment variable '{}' is not set",
                provider_config.client_secret_env
            )
        })?;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ];

        let resp = self
            .http_client
            .post(&provider_config.token_url)
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OAuth token URL returned status {status}: {body}");
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<i64>,
        }

        let token_resp: TokenResponse = resp.json().await?;
        let expires_in = token_resp.expires_in.unwrap_or(3600);
        let expires_at = chrono::Utc::now().timestamp() + expires_in;

        Ok((
            token_resp.access_token,
            token_resp.refresh_token,
            expires_at,
        ))
    }
}

#[async_trait::async_trait]
impl TokenBroker for NatsTokenBroker {
    async fn get_valid_token(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<OAuthToken, TokenError> {
        let key = OAuthToken::make_kv_key(tenant_id, provider);

        let entry = self
            .kv
            .get(&key)
            .await
            .map_err(|e| TokenError::StoreError(e.to_string()))?;

        let token_entry = match entry {
            Some(bytes) => {
                let tok: OAuthToken = serde_json::from_slice(&bytes)
                    .map_err(|e| TokenError::StoreError(e.to_string()))?;
                tok
            }
            None => {
                return Err(TokenError::NotConnected {
                    tenant_id: tenant_id.to_string(),
                    provider: provider.to_string(),
                });
            }
        };

        if token_entry.is_valid() {
            return Ok(token_entry);
        }

        if token_entry.can_refresh() {
            tracing::info!(
                "🔑 Token for tenant {} provider {} is expired. Refreshing...",
                tenant_id,
                provider
            );
            self.refresh_token(tenant_id, provider).await
        } else {
            Err(TokenError::ExpiredNoRefresh)
        }
    }

    async fn store_token(&self, token: &OAuthToken) -> Result<(), TokenError> {
        let key = token.kv_key();
        let val = serde_json::to_vec(token).map_err(|e| TokenError::StoreError(e.to_string()))?;

        self.kv
            .put(&key, val.into())
            .await
            .map_err(|e| TokenError::StoreError(e.to_string()))?;

        Ok(())
    }

    async fn refresh_token(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<OAuthToken, TokenError> {
        let key = OAuthToken::make_kv_key(tenant_id, provider);
        let provider_config = self.providers.get(provider).ok_or_else(|| {
            TokenError::RefreshFailed(format!("No provider configuration found for '{provider}'"))
        })?;

        // Attempt refresh loop (CAS updates)
        for attempt in 0..3 {
            let entry = self
                .kv
                .entry(&key)
                .await
                .map_err(|e| TokenError::StoreError(e.to_string()))?
                .ok_or_else(|| TokenError::NotConnected {
                    tenant_id: tenant_id.to_string(),
                    provider: provider.to_string(),
                })?;

            let mut stored_token: OAuthToken = serde_json::from_slice(&entry.value)
                .map_err(|e| TokenError::StoreError(e.to_string()))?;

            // If another instance updated the token while we were waiting, and it is now valid, reuse it!
            if stored_token.is_valid() {
                tracing::info!(
                    "🔑 Concurrently refreshed token for tenant {} provider {} is now valid. Reusing.",
                    tenant_id,
                    provider
                );
                return Ok(stored_token);
            }

            let refresh_token_str = stored_token.refresh_token.as_deref().ok_or_else(|| {
                TokenError::RefreshFailed("No refresh token available".to_string())
            })?;

            match self
                .refresh_token_oauth(provider_config, refresh_token_str)
                .await
            {
                Ok((new_access, new_refresh, expires_at)) => {
                    stored_token.access_token = new_access;
                    if let Some(ref rt) = new_refresh {
                        stored_token.refresh_token = Some(rt.clone());
                    }
                    stored_token.expires_at = expires_at;

                    let val = serde_json::to_vec(&stored_token)
                        .map_err(|e| TokenError::StoreError(e.to_string()))?;

                    match self.kv.update(&key, val.into(), entry.revision).await {
                        Ok(_) => {
                            tracing::info!(
                                "🔑 Successfully refreshed OAuth token for tenant {} provider {}",
                                tenant_id,
                                provider
                            );
                            return Ok(stored_token);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "⚠️ CAS conflict refreshing token (attempt {}): {}. Retrying...",
                                attempt + 1,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    // Fall back to NATS event to signal UI re-authorization
                    let error_msg = format!("Refresh exchange failed: {e}");
                    let event = json!({
                        "tenant_id": tenant_id,
                        "provider_id": provider,
                        "error": error_msg,
                        "timestamp": chrono::Utc::now().timestamp(),
                    });
                    if let Ok(payload) = serde_json::to_vec(&event) {
                        let _ = self
                            .nats
                            .publish("infra.v1.oauth.refresh_needed".to_string(), payload.into())
                            .await;
                    }
                    return Err(TokenError::RefreshFailed(error_msg));
                }
            }
        }

        Err(TokenError::RefreshFailed(
            "CAS revision updates failed repeatedly due to concurrent refresh operations"
                .to_string(),
        ))
    }

    async fn revoke_token(&self, tenant_id: &str, provider: &str) -> Result<(), TokenError> {
        let key = OAuthToken::make_kv_key(tenant_id, provider);
        self.kv
            .delete(&key)
            .await
            .map_err(|e| TokenError::StoreError(e.to_string()))?;
        Ok(())
    }

    async fn list_connected_providers(&self, tenant_id: &str) -> Result<Vec<String>, TokenError> {
        let mut keys = self
            .kv
            .keys()
            .await
            .map_err(|e| TokenError::StoreError(e.to_string()))?;

        let tenant_prefix = format!("{}_", tenant_id.replace('.', "_"));
        let mut providers = Vec::new();

        while let Some(key_res) = keys.next().await {
            if let Ok(key) = key_res {
                if key.starts_with(&tenant_prefix) {
                    if let Some(provider) = key.strip_prefix(&tenant_prefix) {
                        providers.push(provider.to_string());
                    }
                }
            }
        }

        Ok(providers)
    }
}
