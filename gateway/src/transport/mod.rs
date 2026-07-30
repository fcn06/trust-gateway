// ─────────────────────────────────────────────────────────────
// Transport Module — Pluggable MCP transport drivers.
//
// Provides a `TransportRegistry` that maps app IDs to transport
// driver instances, and re-exports the three driver implementations:
//
// - `NatsMcpDriver`  — NATS JetStream pub/sub (default)
// - `SseMcpDriver`   — Server-Sent Events for external MCPs
// - `StdioMcpDriver` — Local subprocess via stdin/stdout JSON-RPC
// ─────────────────────────────────────────────────────────────

pub mod nats_driver;
pub mod sse_driver;
pub mod stdio_driver;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use trust_core::transport::{McpTransport, TransportType};

pub use nats_driver::NatsMcpDriver;
pub use sse_driver::SseMcpDriver;
pub use stdio_driver::StdioMcpDriver;

/// Registry mapping application IDs to their transport driver instances.
///
/// The gateway router consults this registry when dispatching actions
/// to dynamically registered apps. Built-in tools continue to use
/// the hardcoded NATS dispatch path for backward compatibility.
pub struct TransportRegistry {
    drivers: RwLock<HashMap<String, Arc<dyn McpTransport>>>,
}

impl TransportRegistry {
    /// Create a new empty transport registry.
    pub fn new() -> Self {
        Self {
            drivers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a transport driver for a given app ID.
    ///
    /// If a driver was already registered for this app, it is replaced.
    pub async fn register(&self, app_id: &str, driver: Arc<dyn McpTransport>) {
        tracing::info!(
            "🔌 Transport registered: app_id='{}' type={}",
            app_id,
            driver.transport_type()
        );
        self.drivers
            .write()
            .await
            .insert(app_id.to_string(), driver);
    }

    /// Look up the transport driver for a given app ID.
    pub async fn get(&self, app_id: &str) -> Option<Arc<dyn McpTransport>> {
        self.drivers.read().await.get(app_id).cloned()
    }

    /// Remove a registered transport driver.
    ///
    /// Calls `shutdown()` on the driver before removing it.
    pub async fn remove(&self, app_id: &str) {
        if let Some(driver) = self.drivers.write().await.remove(app_id) {
            if let Err(e) = driver.shutdown().await {
                tracing::warn!(
                    "⚠️ Error shutting down transport for app '{}': {}",
                    app_id,
                    e
                );
            }
            tracing::info!("🔌 Transport removed: app_id='{}'", app_id);
        }
    }

    /// List all registered transports with their types.
    pub async fn list(&self) -> Vec<(String, TransportType)> {
        self.drivers
            .read()
            .await
            .iter()
            .map(|(id, driver)| (id.clone(), driver.transport_type()))
            .collect()
    }

    /// Check how many drivers are registered.
    pub async fn count(&self) -> usize {
        self.drivers.read().await.len()
    }

    /// Look up a transport driver by tool name.
    ///
    /// This is a slower path used when the router needs to find
    /// which app owns a specific tool. It scans all drivers and
    /// calls `list_tools()` — consider caching the mapping.
    pub async fn find_by_tool_name(
        &self,
        tool_name: &str,
    ) -> Option<(String, Arc<dyn McpTransport>)> {
        let drivers = self.drivers.read().await;
        for (app_id, driver) in drivers.iter() {
            if let Ok(tools) = driver.list_tools().await {
                if tools.iter().any(|t| t.name == tool_name) {
                    return Some((app_id.clone(), driver.clone()));
                }
            }
        }
        None
    }
}

impl Default for TransportRegistry {
    fn default() -> Self {
        Self::new()
    }
}
