// ─────────────────────────────────────────────────────────────
// App Manifest — Schema types for hot-plug MCP registration.
//
// Defines the `AppManifest` structure that external MCP servers
// or OAuth connectors publish over `infra.v1.app.register` to
// dynamically register themselves with the Trust Gateway.
//
// The manifest replaces the need for manual code changes to
// `builtin_descriptors()`, `handles()`, and `policy.toml`
// when onboarding new integrations.
// ─────────────────────────────────────────────────────────────

use crate::tool_registry::RiskTier;
use crate::transport::TransportType;
use serde::{Deserialize, Serialize};

/// The type of application being registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    /// An MCP tool server (provides tool invocation capabilities).
    Mcp,
    /// An OAuth2 connector (provides credential-gated API access).
    OAuth2Connector,
    /// A native tool (self-contained executable or script).
    NativeTool,
}

impl std::fmt::Display for AppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppType::Mcp => write!(f, "mcp"),
            AppType::OAuth2Connector => write!(f, "oauth2_connector"),
            AppType::NativeTool => write!(f, "native_tool"),
        }
    }
}

/// Core identity of the registered application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// Reverse-DNS identifier (e.g., "io.lianxi.mcp.hubspot").
    pub id: String,
    /// Human-readable display name (e.g., "HubSpot CRM Connector").
    pub display_name: String,
    /// Application type classification.
    pub app_type: AppType,
    /// Semantic version string (e.g., "1.0.0").
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Tool descriptor within an app manifest.
///
/// Each tool declared here will be registered in the gateway's
/// `ToolRegistry` and become available for agent invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestToolDescriptor {
    /// MCP-compatible tool name (e.g., "hubspot_get_contact").
    pub name: String,
    /// Human-readable description for LLMs.
    pub description: String,
    /// JSON Schema for input parameters.
    #[serde(default = "default_schema")]
    pub input_schema: serde_json::Value,
    /// Risk classification for policy evaluation.
    #[serde(default)]
    pub risk_tier: Option<RiskTier>,
    /// Bundle membership tags (e.g., ["crm", "sales"]).
    #[serde(default)]
    pub bundle_membership: Vec<String>,
}

fn default_schema() -> serde_json::Value {
    serde_json::json!({"type": "object"})
}

/// Routing configuration — how the gateway reaches this app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// NATS executor profile name (e.g., "connector", "custom").
    #[serde(default)]
    pub nats_profile: Option<String>,
    /// Transport type for reaching this app.
    pub transport_type: TransportType,
    /// For SSE transports: the base URL of the MCP server.
    #[serde(default)]
    pub sse_url: Option<String>,
    /// For Stdio transports: the command to spawn.
    #[serde(default)]
    pub stdio_command: Option<String>,
    /// For Stdio transports: command arguments.
    #[serde(default)]
    pub stdio_args: Vec<String>,
}

/// OAuth2 contract — describes authentication requirements
/// for tools that need third-party API access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContract {
    /// Whether this app requires OAuth2 tokens.
    pub requires_oauth: bool,
    /// The OAuth provider ID to use from `oauth_providers.toml`.
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Custom authorization URL (if not using a pre-configured provider).
    #[serde(default)]
    pub provider_auth_url: Option<String>,
    /// Required OAuth scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Complete application manifest for hot-plug registration.
///
/// Published over `infra.v1.app.register` NATS subject.
/// Persisted in the `app_registry` KV bucket for restart recovery.
///
/// # Example (TOML representation)
///
/// ```toml
/// [app]
/// id = "io.lianxi.mcp.hubspot"
/// display_name = "HubSpot CRM Connector"
/// app_type = "mcp"
///
/// [[tools]]
/// name = "hubspot_get_contact"
/// description = "Get a contact from HubSpot CRM"
/// risk_tier = "read_only"
/// bundle_membership = ["crm"]
///
/// [[tools]]
/// name = "hubspot_update_deal"
/// description = "Update a deal in HubSpot CRM"
/// risk_tier = "write"
/// bundle_membership = ["crm", "sales"]
///
/// [routing]
/// transport_type = "sse"
/// sse_url = "http://hubspot-mcp:8080"
///
/// [auth_contract]
/// requires_oauth = true
/// provider_id = "hubspot"
/// scopes = ["crm.objects.contacts.read", "crm.objects.deals.write"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    /// Application identity and classification.
    pub app: AppInfo,
    /// Tools provided by this application.
    #[serde(default)]
    pub tools: Vec<ManifestToolDescriptor>,
    /// Routing configuration.
    pub routing: RoutingConfig,
    /// Optional OAuth2 authentication contract.
    #[serde(default)]
    pub auth_contract: Option<AuthContract>,
    /// Unix timestamp when this manifest was registered.
    /// Set by the gateway upon registration, not by the publisher.
    #[serde(default)]
    pub registered_at: Option<i64>,
}

impl AppManifest {
    /// Validate that the manifest is well-formed.
    ///
    /// Checks:
    /// - App ID is non-empty and uses reverse-DNS format
    /// - At least one tool is declared
    /// - Tool names are non-empty and unique within the manifest
    /// - Transport-specific fields are present (e.g., sse_url for SSE)
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        // App ID validation
        if self.app.id.is_empty() {
            return Err(ManifestValidationError::MissingField("app.id".into()));
        }
        if !self.app.id.contains('.') {
            return Err(ManifestValidationError::InvalidFormat(
                "app.id must use reverse-DNS format (e.g., io.lianxi.mcp.hubspot)".into(),
            ));
        }

        // Tools validation
        if self.tools.is_empty() {
            return Err(ManifestValidationError::MissingField(
                "tools (at least one tool must be declared)".into(),
            ));
        }
        let mut seen_names = std::collections::HashSet::new();
        for tool in &self.tools {
            if tool.name.is_empty() {
                return Err(ManifestValidationError::MissingField("tool.name".into()));
            }
            if !seen_names.insert(&tool.name) {
                return Err(ManifestValidationError::DuplicateTool(tool.name.clone()));
            }
        }

        // Transport-specific validation
        match self.routing.transport_type {
            TransportType::Sse => {
                if self
                    .routing
                    .sse_url
                    .as_ref()
                    .map(|u| u.is_empty())
                    .unwrap_or(true)
                {
                    return Err(ManifestValidationError::MissingField(
                        "routing.sse_url (required for SSE transport)".into(),
                    ));
                }
            }
            TransportType::Stdio => {
                if self
                    .routing
                    .stdio_command
                    .as_ref()
                    .map(|c| c.is_empty())
                    .unwrap_or(true)
                {
                    return Err(ManifestValidationError::MissingField(
                        "routing.stdio_command (required for Stdio transport)".into(),
                    ));
                }
            }
            TransportType::Nats => {
                // NATS transport uses the existing exec.v1 topology — no extra fields needed.
            }
        }

        Ok(())
    }

    /// Generate the NATS KV key for persisting this manifest.
    /// RULE 020: Uses `_` as separator, never `:`.
    pub fn kv_key(&self) -> String {
        format!("app_{}", self.app.id.replace([':', '.', '@'], "_"))
    }
}

/// Errors from manifest validation.
#[derive(Debug, thiserror::Error)]
pub enum ManifestValidationError {
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    #[error("duplicate tool name: {0}")]
    DuplicateTool(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> AppManifest {
        AppManifest {
            app: AppInfo {
                id: "io.lianxi.mcp.hubspot".into(),
                display_name: "HubSpot CRM".into(),
                app_type: AppType::Mcp,
                version: "1.0.0".into(),
            },
            tools: vec![ManifestToolDescriptor {
                name: "hubspot_get_contact".into(),
                description: "Get a contact".into(),
                input_schema: serde_json::json!({"type": "object"}),
                risk_tier: Some(RiskTier::ReadOnly),
                bundle_membership: vec!["crm".into()],
            }],
            routing: RoutingConfig {
                nats_profile: None,
                transport_type: TransportType::Sse,
                sse_url: Some("http://hubspot-mcp:8080".into()),
                stdio_command: None,
                stdio_args: vec![],
            },
            auth_contract: Some(AuthContract {
                requires_oauth: true,
                provider_id: Some("hubspot".into()),
                provider_auth_url: None,
                scopes: vec!["crm.objects.contacts.read".into()],
            }),
            registered_at: None,
        }
    }

    #[test]
    fn valid_manifest_passes_validation() {
        assert!(valid_manifest().validate().is_ok());
    }

    #[test]
    fn empty_app_id_fails() {
        let mut m = valid_manifest();
        m.app.id = String::new();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::MissingField(_))
        ));
    }

    #[test]
    fn non_reverse_dns_id_fails() {
        let mut m = valid_manifest();
        m.app.id = "hubspot".into();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::InvalidFormat(_))
        ));
    }

    #[test]
    fn no_tools_fails() {
        let mut m = valid_manifest();
        m.tools.clear();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::MissingField(_))
        ));
    }

    #[test]
    fn duplicate_tool_names_fails() {
        let mut m = valid_manifest();
        m.tools.push(m.tools[0].clone());
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::DuplicateTool(_))
        ));
    }

    #[test]
    fn sse_without_url_fails() {
        let mut m = valid_manifest();
        m.routing.sse_url = None;
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::MissingField(_))
        ));
    }

    #[test]
    fn nats_transport_needs_no_extra_fields() {
        let mut m = valid_manifest();
        m.routing.transport_type = TransportType::Nats;
        m.routing.sse_url = None;
        assert!(m.validate().is_ok());
    }

    #[test]
    fn manifest_kv_key_uses_underscore() {
        // RULE 020: JetStream composite keys use _ separator
        let m = valid_manifest();
        let key = m.kv_key();
        assert!(!key.contains(':'));
        assert!(!key.contains('.'));
        assert_eq!(key, "app_io_lianxi_mcp_hubspot");
    }

    #[test]
    fn manifest_round_trip_json() {
        let m = valid_manifest();
        let json = serde_json::to_string(&m).unwrap();
        let restored: AppManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.app.id, "io.lianxi.mcp.hubspot");
        assert_eq!(restored.tools.len(), 1);
        assert_eq!(restored.routing.transport_type, TransportType::Sse);
    }

    #[test]
    fn app_type_display() {
        assert_eq!(AppType::Mcp.to_string(), "mcp");
        assert_eq!(AppType::OAuth2Connector.to_string(), "oauth2_connector");
        assert_eq!(AppType::NativeTool.to_string(), "native_tool");
    }
}
