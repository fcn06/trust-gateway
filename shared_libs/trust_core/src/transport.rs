// ─────────────────────────────────────────────────────────────
// McpTransport — Pluggable transport driver abstraction.
//
// Defines the core trait that decouples tool execution from
// the underlying network transport. The Trust Gateway routes
// actions through transport drivers without knowing whether
// the target MCP server is reached via NATS, SSE, or Stdio.
//
// This is a pure contract (no transport dependencies) that
// lives in trust_core so both gateway and executor crates
// can depend on the shared type definitions.
// ─────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// Transport type classification.
///
/// Used by the `AppManifest` routing config and `TransportRegistry`
/// to select the appropriate driver implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportType {
    /// Internal NATS JetStream bus (default for all built-in tools).
    Nats,
    /// Server-Sent Events for external cloud-hosted MCP servers.
    Sse,
    /// Local subprocess communicating via stdin/stdout JSON-RPC.
    Stdio,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Nats => write!(f, "nats"),
            TransportType::Sse => write!(f, "sse"),
            TransportType::Stdio => write!(f, "stdio"),
        }
    }
}

/// A tool definition discovered from a transport driver.
///
/// Lighter than `ToolDescriptor` — represents only what the
/// transport layer reports about a remote tool. The gateway
/// enriches this into a full `ToolDescriptor` upon registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportToolDefinition {
    /// Tool name as reported by the MCP server.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for input parameters.
    pub input_schema: serde_json::Value,
}

/// Errors from transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("transport timed out after {0}s")]
    Timeout(u64),
    #[error("protocol error: {0}")]
    ProtocolError(String),
    #[error("transport unavailable: {0}")]
    Unavailable(String),
    #[error("transport not initialized")]
    NotInitialized,
    #[error("internal transport error: {0}")]
    Internal(String),
}

/// A dispatch request containing the minimal information needed
/// for a transport driver to execute a tool call.
///
/// This is transport-neutral — each driver maps it to its own
/// wire format (NATS envelope, JSON-RPC, SSE tools/call, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportDispatchRequest {
    /// The tool to invoke.
    pub tool_name: String,
    /// Canonical JSON arguments.
    pub arguments: serde_json::Value,
    /// The signed ExecutionGrant JWT (passed to NATS executors
    /// for verification; SSE/Stdio drivers may ignore this).
    pub grant_jwt: String,
    /// SHA-256 hash of the canonical arguments.
    pub input_hash: String,
    /// Tenant context for scoping.
    pub tenant_id: String,
    /// Unique action ID for correlation.
    pub action_id: String,
}

/// The result returned by a transport driver after tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportDispatchResult {
    /// Whether the execution succeeded.
    pub success: bool,
    /// The output payload (tool-specific).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Error message if execution failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
}

/// Pluggable transport driver for MCP server communication.
///
/// Implementations:
/// - `NatsMcpDriver` — translates to `exec.v1.<tenant>.<profile>.invoke` topology
/// - `SseMcpDriver` — maintains long-lived HTTP SSE streams to external MCPs
/// - `StdioMcpDriver` — spawns isolated local subprocesses for script-based tools
///
/// All implementations must be `Send + Sync` for use in Tokio/Axum environments.
#[async_trait::async_trait]
pub trait McpTransport: Send + Sync {
    /// The transport type this driver implements.
    fn transport_type(&self) -> TransportType;

    /// Human-readable identifier for this driver instance.
    fn driver_id(&self) -> &str;

    /// Initialize the transport connection.
    ///
    /// For NATS: no-op (connection is managed externally).
    /// For SSE: performs the MCP handshake.
    /// For Stdio: spawns the subprocess.
    async fn initialize(&mut self) -> Result<(), TransportError>;

    /// Discover tools available through this transport.
    ///
    /// Returns the tool definitions reported by the remote MCP server.
    /// For NATS-based executors, this typically returns an empty vec
    /// (tools are registered via the ToolRegistry, not discovered).
    async fn list_tools(&self) -> Result<Vec<TransportToolDefinition>, TransportError>;

    /// Dispatch a tool call through this transport.
    async fn dispatch(
        &self,
        request: &TransportDispatchRequest,
    ) -> Result<TransportDispatchResult, TransportError>;

    /// Health check — verifies the transport is still operational.
    async fn health_check(&self) -> Result<(), TransportError>;

    /// Gracefully shut down this transport driver.
    async fn shutdown(&self) -> Result<(), TransportError> {
        // Default: no-op. Drivers with persistent connections override this.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_type_display() {
        assert_eq!(TransportType::Nats.to_string(), "nats");
        assert_eq!(TransportType::Sse.to_string(), "sse");
        assert_eq!(TransportType::Stdio.to_string(), "stdio");
    }

    #[test]
    fn transport_type_serde_round_trip() {
        let nats = TransportType::Nats;
        let json = serde_json::to_string(&nats).unwrap();
        assert_eq!(json, "\"nats\"");
        let restored: TransportType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, TransportType::Nats);
    }

    #[test]
    fn dispatch_request_round_trip() {
        let req = TransportDispatchRequest {
            tool_name: "hubspot_get_contact".into(),
            arguments: serde_json::json!({"contact_id": "123"}),
            grant_jwt: "eyJ...".into(),
            input_hash: "abc123".into(),
            tenant_id: "t1".into(),
            action_id: "act-001".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: TransportDispatchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tool_name, "hubspot_get_contact");
        assert_eq!(restored.tenant_id, "t1");
    }

    #[test]
    fn dispatch_result_round_trip() {
        let result = TransportDispatchResult {
            success: true,
            output: Some(serde_json::json!({"name": "John"})),
            error: None,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: TransportDispatchResult = serde_json::from_str(&json).unwrap();
        assert!(restored.success);
        assert_eq!(restored.duration_ms, 42);
        assert!(restored.error.is_none());
    }

    #[test]
    fn tool_definition_round_trip() {
        let tool = TransportToolDefinition {
            name: "search".into(),
            description: "Search the web".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let restored: TransportToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "search");
    }
}
