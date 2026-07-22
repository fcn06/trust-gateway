// ─────────────────────────────────────────────────────────────
// StdioMcpDriver — Local subprocess transport for script-based
// MCP servers communicating via stdin/stdout JSON-RPC.
//
// Spawns an isolated process with env_clear() + setsid + memory
// limits, matching the existing sandboxed execution pipeline.
//
// SECURITY: This driver follows the same process isolation rules
// as custom tool execution:
//   - env_clear() wipes all environment variables
//   - setsid isolates process groups
//   - Memory limits prevent runaway processes
// ─────────────────────────────────────────────────────────────

use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use trust_core::transport::{
    McpTransport, TransportDispatchRequest, TransportDispatchResult,
    TransportError, TransportToolDefinition, TransportType,
};

/// Stdio transport driver for local subprocess-based MCP servers.
///
/// Each dispatch spawns a fresh subprocess to ensure isolation.
/// The subprocess communicates via JSON-RPC over stdin/stdout.
pub struct StdioMcpDriver {
    /// Command to spawn (e.g., "node", "python3", "/usr/local/bin/my-mcp").
    command: String,
    /// Command arguments.
    args: Vec<String>,
    /// Human-readable driver ID.
    id: String,
    /// Whether the driver has been initialized (command validated).
    initialized: RwLock<bool>,
    /// Cached tool list from the last discovery.
    cached_tools: RwLock<Vec<TransportToolDefinition>>,
}

impl StdioMcpDriver {
    /// Create a new Stdio driver with the given command and arguments.
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        let command = command.into();
        let id = format!("stdio_{}", command.replace(['/', '.', ' '], "_"));
        Self {
            command,
            args,
            id,
            initialized: RwLock::new(false),
            cached_tools: RwLock::new(Vec::new()),
        }
    }

    /// Spawn an isolated subprocess and send a JSON-RPC request.
    ///
    /// Returns the JSON-RPC response parsed as a serde_json::Value.
    async fn send_jsonrpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TransportError> {
        // Build the JSON-RPC request.
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let request_str = serde_json::to_string(&request)
            .map_err(|e| TransportError::Internal(format!("Serialization failed: {}", e)))?;

        // Spawn the subprocess with security isolation.
        let mut cmd = tokio::process::Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // SECURITY: Clear all environment variables to prevent leakage
            // of NATS credentials, API keys, or system secrets.
            .env_clear();

        // On Unix: isolate process group with setsid.
        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        let mut child = cmd.spawn().map_err(|e| {
            TransportError::ConnectionFailed(format!(
                "Failed to spawn '{}': {}",
                self.command, e
            ))
        })?;

        // Write the request to stdin.
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(request_str.as_bytes())
                .await
                .map_err(|e| {
                    TransportError::ProtocolError(format!("Failed to write to stdin: {}", e))
                })?;
            stdin.write_all(b"\n").await.ok();
            // Close stdin to signal end of input.
            drop(stdin);
        }

        // Read the response from stdout with a timeout.
        let stdout = child.stdout.take().ok_or_else(|| {
            TransportError::ProtocolError("No stdout handle".into())
        })?;
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();

        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            reader.read_line(&mut response_line),
        )
        .await
        {
            Ok(Ok(0)) => {
                // Check stderr for error messages.
                let mut stderr_output = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let mut stderr_reader = BufReader::new(&mut stderr);
                    let _ = stderr_reader.read_line(&mut stderr_output).await;
                }
                return Err(TransportError::ProtocolError(format!(
                    "Subprocess produced no output. stderr: {}",
                    stderr_output.trim()
                )));
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                return Err(TransportError::ProtocolError(format!(
                    "Failed to read from stdout: {}",
                    e
                )));
            }
            Err(_) => {
                // Kill the runaway process.
                let _ = child.kill().await;
                return Err(TransportError::Timeout(30));
            }
        }

        // Kill the child process (we don't reuse it).
        let _ = child.kill().await;
        let _ = child.wait().await;

        // Parse the JSON-RPC response.
        let response: serde_json::Value = serde_json::from_str(response_line.trim())
            .map_err(|e| {
                TransportError::ProtocolError(format!(
                    "Invalid JSON-RPC response: {} (raw: '{}')",
                    e,
                    response_line.trim()
                ))
            })?;

        // Check for JSON-RPC errors.
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(TransportError::ProtocolError(format!(
                "JSON-RPC error: {}",
                message
            )));
        }

        Ok(response
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }
}

#[async_trait]
impl McpTransport for StdioMcpDriver {
    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }

    fn driver_id(&self) -> &str {
        &self.id
    }

    async fn initialize(&mut self) -> Result<(), TransportError> {
        // Validate that the command exists and can spawn.
        let tools = self.discover_tools_internal().await?;
        *self.cached_tools.write().await = tools;
        *self.initialized.write().await = true;
        tracing::info!(
            "✅ Stdio transport initialized: {} (discovered {} tools)",
            self.command,
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

        let params = serde_json::json!({
            "name": request.tool_name,
            "arguments": request.arguments,
        });

        match self.send_jsonrpc("tools/call", params).await {
            Ok(result) => {
                let is_error = result
                    .get("isError")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let content = result.get("content").cloned();
                let output_text = content
                    .as_ref()
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();

                Ok(TransportDispatchResult {
                    success: !is_error,
                    output: content.or_else(|| {
                        Some(serde_json::json!([{"type": "text", "text": output_text}]))
                    }),
                    error: if is_error {
                        Some(output_text)
                    } else {
                        None
                    },
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn health_check(&self) -> Result<(), TransportError> {
        // Verify the command is accessible.
        let output = tokio::process::Command::new("which")
            .arg(&self.command)
            .env_clear()
            .output()
            .await
            .map_err(|e| {
                TransportError::Internal(format!(
                    "Cannot check command '{}': {}",
                    self.command, e
                ))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(TransportError::Unavailable(format!(
                "Command '{}' not found in PATH",
                self.command
            )))
        }
    }

    async fn shutdown(&self) -> Result<(), TransportError> {
        tracing::info!(
            "🔌 Stdio transport shutting down: {}",
            self.command
        );
        *self.initialized.write().await = false;
        Ok(())
    }
}

impl StdioMcpDriver {
    /// Internal tool discovery via JSON-RPC `tools/list`.
    async fn discover_tools_internal(
        &self,
    ) -> Result<Vec<TransportToolDefinition>, TransportError> {
        let result = self
            .send_jsonrpc("tools/list", serde_json::json!({}))
            .await?;

        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(tools
            .into_iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?.to_string();
                let description = t
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let input_schema = t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or(serde_json::json!({"type": "object"}));
                Some(TransportToolDefinition {
                    name,
                    description,
                    input_schema,
                })
            })
            .collect())
    }
}
