// ─────────────────────────────────────────────────────────────
// SseMcpDriver — SSE transport for external cloud-hosted MCPs.
//
// Maintains a long-lived HTTP SSE connection to an external MCP
// server, using the rmcp client library for the MCP protocol
// handshake and tool discovery/invocation.
//
// This driver wraps the existing `discover_mcp_tools()` pattern
// from router.rs into a proper McpTransport implementation.
// ─────────────────────────────────────────────────────────────

use async_trait::async_trait;
use tokio::sync::RwLock;
use trust_core::transport::{
    McpTransport, TransportDispatchRequest, TransportDispatchResult, TransportError,
    TransportToolDefinition, TransportType,
};

/// SSE transport driver for external MCP servers.
///
/// Connects to a standard MCP SSE endpoint (e.g., `http://host:8080/sse`)
/// and performs tool discovery and invocation via the MCP protocol.
pub struct SseMcpDriver {
    /// Base URL of the MCP server (e.g., "http://hubspot-mcp:8080").
    base_url: String,
    /// Human-readable driver ID.
    id: String,
    /// HTTP client for the SSE connection.
    http_client: reqwest::Client,
    /// Cached tool list from the last discovery.
    cached_tools: RwLock<Vec<TransportToolDefinition>>,
    /// Whether the driver has been initialized.
    initialized: RwLock<bool>,
}

impl SseMcpDriver {
    /// Create a new SSE driver targeting the given MCP server URL.
    pub fn new(base_url: impl Into<String>, http_client: reqwest::Client) -> Self {
        let base_url = base_url.into();
        let id = format!("sse_{}", base_url.replace([':', '/', '.'], "_"));
        Self {
            base_url,
            id,
            http_client,
            cached_tools: RwLock::new(Vec::new()),
            initialized: RwLock::new(false),
        }
    }
}

#[async_trait]
impl McpTransport for SseMcpDriver {
    fn transport_type(&self) -> TransportType {
        TransportType::Sse
    }

    fn driver_id(&self) -> &str {
        &self.id
    }

    async fn initialize(&mut self) -> Result<(), TransportError> {
        // Perform initial tool discovery to validate the connection.
        let tools = self.discover_tools_internal().await?;
        *self.cached_tools.write().await = tools;
        *self.initialized.write().await = true;
        tracing::info!(
            "✅ SSE transport initialized: {} (discovered {} tools)",
            self.base_url,
            self.cached_tools.read().await.len()
        );
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<TransportToolDefinition>, TransportError> {
        if !*self.initialized.read().await {
            return Err(TransportError::NotInitialized);
        }
        Ok(self.cached_tools.read().await.clone())
    }

    async fn dispatch(
        &self,
        request: &TransportDispatchRequest,
    ) -> Result<TransportDispatchResult, TransportError> {
        if !*self.initialized.read().await {
            return Err(TransportError::NotInitialized);
        }

        let start = std::time::Instant::now();

        // Build the SSE endpoint URL.
        let mut sse_uri = self.base_url.trim_end_matches('/').to_string();
        if !sse_uri.ends_with("/sse") {
            sse_uri = format!("{sse_uri}/sse");
        }

        // Create a new MCP client session for this invocation.
        let config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                sse_uri,
            );
        let transport = rmcp::transport::StreamableHttpClientTransport::with_client(
            self.http_client.clone(),
            config,
        );

        let client_info = rmcp::model::InitializeRequestParams::new(
            rmcp::model::ClientCapabilities::default(),
            rmcp::model::Implementation::new("trust-gateway-transport", "0.1.0"),
        );

        let running = rmcp::serve_client(client_info, transport)
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("MCP handshake failed: {e}"))
            })?;

        let mcp_client = running.clone();

        // Drive the transport in a background task.
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let _svc = running;
            let _ = rx.await;
        });

        // Build the tool call request.
        let call_params = rmcp::model::CallToolRequestParams::new(request.tool_name.clone())
            .with_arguments(request.arguments.as_object().cloned().unwrap_or_default());

        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            mcp_client.call_tool(call_params),
        )
        .await
        {
            Ok(Ok(call_result)) => {
                let output_text = call_result
                    .content
                    .iter()
                    .filter_map(|c| {
                        if let rmcp::model::RawContent::Text(text) = &c.raw {
                            Some(text.text.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let is_error = call_result.is_error.unwrap_or(false);
                let output_json = serde_json::from_str::<serde_json::Value>(&output_text)
                    .unwrap_or_else(|_| serde_json::json!([{"type": "text", "text": output_text}]));

                Ok(TransportDispatchResult {
                    success: !is_error,
                    output: Some(output_json),
                    error: if is_error { Some(output_text) } else { None },
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
            Ok(Err(e)) => Err(TransportError::ProtocolError(format!(
                "Tool call failed: {e}"
            ))),
            Err(_) => Err(TransportError::Timeout(30)),
        };

        // Shut down the MCP client session.
        let _ = tx.send(());
        handle.abort();

        result
    }

    async fn health_check(&self) -> Result<(), TransportError> {
        // Simple HTTP connectivity check.
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        match self
            .http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                // 404 is acceptable — not all MCP servers expose /health.
                Ok(())
            }
            Ok(resp) => Err(TransportError::Unavailable(format!(
                "Health check returned {}",
                resp.status()
            ))),
            Err(e) => Err(TransportError::ConnectionFailed(format!(
                "Health check failed: {e}"
            ))),
        }
    }

    async fn shutdown(&self) -> Result<(), TransportError> {
        tracing::info!("🔌 SSE transport shutting down: {}", self.base_url);
        *self.initialized.write().await = false;
        Ok(())
    }
}

impl SseMcpDriver {
    /// Internal tool discovery using the MCP protocol.
    async fn discover_tools_internal(
        &self,
    ) -> Result<Vec<TransportToolDefinition>, TransportError> {
        let mut sse_uri = self.base_url.trim_end_matches('/').to_string();
        if !sse_uri.ends_with("/sse") {
            sse_uri = format!("{sse_uri}/sse");
        }

        let config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                sse_uri,
            );
        let transport = rmcp::transport::StreamableHttpClientTransport::with_client(
            self.http_client.clone(),
            config,
        );

        let client_info = rmcp::model::InitializeRequestParams::new(
            rmcp::model::ClientCapabilities::default(),
            rmcp::model::Implementation::new("trust-gateway-discovery", "0.1.0"),
        );

        let running = rmcp::serve_client(client_info, transport)
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Discovery handshake failed: {e}"))
            })?;

        let mcp_client = running.clone();

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let _svc = running;
            let _ = rx.await;
        });

        let result = mcp_client.list_tools(Default::default()).await;

        let _ = tx.send(());
        handle.abort();

        let list_result = result
            .map_err(|e| TransportError::ProtocolError(format!("Tool listing failed: {e}")))?;

        Ok(list_result
            .tools
            .into_iter()
            .map(|t| TransportToolDefinition {
                name: t.name.to_string(),
                description: t.description.unwrap_or_default().to_string(),
                input_schema: serde_json::Value::Object((*t.input_schema).clone()),
            })
            .collect())
    }
}
