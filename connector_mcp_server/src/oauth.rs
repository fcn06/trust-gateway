//! OAuth flow handlers — authorization redirect and callback.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Redirect,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{token_store::OAuthToken, AppState};

/// Query params for OAuth callback.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: String, // Contains tenant_id
}

/// GET /oauth/{provider_id}/authorize/{tenant_id} — Redirect to OAuth provider consent screen.
pub async fn provider_authorize(
    State(state): State<Arc<AppState>>,
    Path((provider_id, tenant_id)): Path<(String, String)>,
) -> Result<Redirect, (StatusCode, String)> {
    let provider = state.providers.get(&provider_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("OAuth provider '{provider_id}' not found"),
        )
    })?;

    let client_id = std::env::var(&provider.client_id_env).unwrap_or_default();
    if client_id.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("OAuth provider '{provider_id}' is not configured (missing client ID)"),
        ));
    }

    let redirect_uri = std::env::var(format!("{}_REDIRECT_URI", provider_id.to_uppercase()))
        .unwrap_or_else(|_| {
            if let Ok(google_uri) = std::env::var("GOOGLE_REDIRECT_URI") {
                if let Some(pos) = google_uri.find("/oauth/google/callback") {
                    return format!("{}/oauth/{}/callback", &google_uri[..pos], provider_id);
                }
            }
            format!(
                "{}/oauth/{}/callback",
                identity_context::load_secret("CONNECTOR_MCP_URL")
                    .map(|s| s.expose_secret().to_string())
                    .unwrap_or_else(|| "http://localhost:3050".to_string()),
                provider_id
            )
        });

    let scopes = provider.scopes.join(" ");

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
        provider.auth_url,
        client_id,
        urlencoding(&redirect_uri),
        urlencoding(&scopes),
        urlencoding(&tenant_id),
    );

    tracing::info!(
        "🔐 Redirecting tenant {} to {} OAuth consent",
        tenant_id,
        provider.display_name
    );
    Ok(Redirect::temporary(&auth_url))
}

/// GET /oauth/{provider_id}/callback — Handle OAuth callback and store tokens.
pub async fn provider_callback(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let tenant_id = params.state;
    tracing::info!("🔑 {} OAuth callback for tenant {}", provider_id, tenant_id);

    let provider = state.providers.get(&provider_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("OAuth provider '{provider_id}' not found"),
        )
    })?;

    let client_id = std::env::var(&provider.client_id_env).unwrap_or_default();
    let client_secret = std::env::var(&provider.client_secret_env).unwrap_or_default();

    let redirect_uri = std::env::var(format!("{}_REDIRECT_URI", provider_id.to_uppercase()))
        .unwrap_or_else(|_| {
            if let Ok(google_uri) = std::env::var("GOOGLE_REDIRECT_URI") {
                if let Some(pos) = google_uri.find("/oauth/google/callback") {
                    return format!("{}/oauth/{}/callback", &google_uri[..pos], provider_id);
                }
            }
            format!(
                "{}/oauth/{}/callback",
                identity_context::load_secret("CONNECTOR_MCP_URL")
                    .map(|s| s.expose_secret().to_string())
                    .unwrap_or_else(|| "http://localhost:3050".to_string()),
                provider_id
            )
        });

    // Exchange authorization code for tokens
    let client = state.http_client.clone();
    let token_response = client
        .post(&provider.token_url)
        .form(&[
            ("code", params.code.as_str()),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Token exchange failed: {e}"),
            )
        })?;

    if !token_response.status().is_success() {
        let err_body = token_response.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "{} token exchange failed: {}",
                provider.display_name, err_body
            ),
        ));
    }

    let token_data: serde_json::Value = token_response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse token response: {e}"),
        )
    })?;

    let now = chrono::Utc::now().timestamp();
    let expires_in = token_data["expires_in"].as_i64().unwrap_or(3600);

    let oauth_token = OAuthToken {
        tenant_id: tenant_id.clone(),
        provider: provider_id.clone(),
        access_token: token_data["access_token"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        refresh_token: token_data["refresh_token"].as_str().map(|s| s.to_string()),
        expires_at: now + expires_in,
        scopes: provider.scopes.clone(),
        created_at: now,
    };

    state
        .token_store
        .store_token(&oauth_token)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to store token: {e}"),
            )
        })?;

    Ok(Json(serde_json::json!({
        "status": "connected",
        "tenant_id": tenant_id,
        "provider": provider_id,
        "scopes": provider.scopes,
    })))
}

/// GET /oauth/status/{tenant_id} — Return connection status for all providers.
pub async fn integration_status(
    State(state): State<Arc<AppState>>,
    Path(tenant_id): Path<String>,
) -> Json<serde_json::Value> {
    let mut statuses = Vec::new();

    for provider_id in state.providers.keys() {
        let (connected, scopes, connected_at) =
            match state.token_store.get_token(&tenant_id, provider_id).await {
                Ok(Some(token)) => {
                    let valid = crate::token_store::TokenStore::is_token_valid(&token);
                    (valid, token.scopes.clone(), Some(token.created_at))
                }
                _ => (false, vec![], None),
            };

        statuses.push(serde_json::json!({
            "provider": provider_id,
            "connected": connected,
            "scopes": scopes,
            "connected_at": connected_at,
        }));
    }

    Json(serde_json::json!({ "integrations": statuses }))
}

/// Simple URL encoding helper.
fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('@', "%40")
}
