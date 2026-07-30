// ─────────────────────────────────────────────────────────────
// App Registry — Dynamic app registration manager.
//
// Ingests AppManifests published over NATS, validates them,
// instantiates the corresponding transport drivers, and registers
// the tools dynamically into the ToolRegistry.
// ─────────────────────────────────────────────────────────────

use crate::router::{ToolRegistry, ToolRegistryEntry};
use crate::transport::{NatsMcpDriver, SseMcpDriver, StdioMcpDriver, TransportRegistry};
use async_nats::jetstream::kv::Store;
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use trust_core::app_manifest::{AppManifest, AppType};
use trust_core::transport::{McpTransport, TransportType};

/// Registry to supervise dynamic application onboarding.
pub struct AppRegistry {
    kv: Option<Store>,
    tool_registry: Arc<ToolRegistry>,
    transport_registry: Arc<TransportRegistry>,
    nats: async_nats::Client,
    policy_fingerprint: String,
    http_client: reqwest::Client,
}

impl AppRegistry {
    /// Create a new AppRegistry instance.
    pub fn new(
        kv: Option<Store>,
        tool_registry: Arc<ToolRegistry>,
        transport_registry: Arc<TransportRegistry>,
        nats: async_nats::Client,
        policy_fingerprint: String,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            kv,
            tool_registry,
            transport_registry,
            nats,
            policy_fingerprint,
            http_client,
        }
    }

    /// Register a new application manifest.
    pub async fn register_app(&self, mut manifest: AppManifest) -> anyhow::Result<()> {
        manifest.validate()?;
        manifest.registered_at = Some(chrono::Utc::now().timestamp());

        let app_id = manifest.app.id.clone();
        tracing::info!(
            "🔌 Registering application '{}' ({})",
            manifest.app.display_name,
            app_id
        );

        // Create driver by transport type and initialize before wrapping in Arc
        let driver: Arc<dyn McpTransport> = match manifest.routing.transport_type {
            TransportType::Nats => {
                let profile = manifest
                    .routing
                    .nats_profile
                    .clone()
                    .unwrap_or_else(|| "connector".to_string());
                let mut d =
                    NatsMcpDriver::new(self.nats.clone(), profile, self.policy_fingerprint.clone());
                d.initialize().await?;
                Arc::new(d)
            }
            TransportType::Sse => {
                let sse_url = manifest
                    .routing
                    .sse_url
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("Missing sse_url for SSE transport"))?;
                let mut d = SseMcpDriver::new(sse_url, self.http_client.clone());
                d.initialize().await?;
                Arc::new(d)
            }
            TransportType::Stdio => {
                let command =
                    manifest.routing.stdio_command.clone().ok_or_else(|| {
                        anyhow::anyhow!("Missing stdio_command for Stdio transport")
                    })?;
                let args = manifest.routing.stdio_args.clone();
                let mut d = StdioMcpDriver::new(command, args);
                d.initialize().await?;
                Arc::new(d)
            }
        };

        // Register in the transport registry
        self.transport_registry.register(&app_id, driver).await;

        // Register tools in the ToolRegistry
        let mut tool_entries = Vec::new();
        for tool in &manifest.tools {
            let entry = ToolRegistryEntry {
                executor_type: "mcp".to_string(),
                category: tool.bundle_membership.first().cloned(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                cron: None,
                app_id: Some(app_id.clone()),
            };
            tool_entries.push((tool.name.clone(), entry));
        }

        self.tool_registry
            .register_dynamic_tools(tool_entries)
            .await;

        // Persist to JetStream KV
        if let Some(ref kv) = self.kv {
            let key = manifest.kv_key();
            let val = serde_json::to_vec(&manifest)?;
            kv.put(&key, val.into()).await?;
        }

        Ok(())
    }

    /// Unregister an application and its tools.
    pub async fn unregister_app(&self, app_id: &str) -> anyhow::Result<()> {
        // 1. Remove from transport registry
        self.transport_registry.remove(app_id).await;

        // 2. Remove tools from ToolRegistry
        self.tool_registry.unregister_dynamic_tools(app_id).await;

        // 3. Remove from NATS KV
        if let Some(ref kv) = self.kv {
            let dummy_manifest = AppManifest {
                app: trust_core::app_manifest::AppInfo {
                    id: app_id.to_string(),
                    display_name: String::new(),
                    app_type: AppType::Mcp,
                    version: String::new(),
                },
                tools: vec![],
                routing: trust_core::app_manifest::RoutingConfig {
                    nats_profile: None,
                    transport_type: TransportType::Nats,
                    sse_url: None,
                    stdio_command: None,
                    stdio_args: vec![],
                },
                auth_contract: None,
                registered_at: None,
            };
            let key = dummy_manifest.kv_key();
            kv.delete(&key).await.ok();
        }

        tracing::info!("🔌 Unregistered application: '{}'", app_id);
        Ok(())
    }

    /// List all registered manifests.
    pub async fn list_apps(&self) -> anyhow::Result<Vec<AppManifest>> {
        let kv = match &self.kv {
            Some(k) => k,
            None => return Ok(Vec::new()),
        };

        let mut apps = Vec::new();
        let mut keys = kv.keys().await?;
        while let Some(key_res) = keys.next().await {
            if let Ok(key) = key_res {
                if key.starts_with("app_") {
                    if let Ok(Some(entry)) = kv.get(&key).await {
                        if let Ok(manifest) = serde_json::from_slice::<AppManifest>(&entry) {
                            apps.push(manifest);
                        }
                    }
                }
            }
        }
        Ok(apps)
    }

    /// Restore registered apps from JetStream KV (called at startup).
    pub async fn restore_from_kv(&self) -> anyhow::Result<()> {
        let kv = match &self.kv {
            Some(k) => k,
            None => return Ok(()),
        };

        tracing::info!("🔄 Restoring registered apps from NATS KV...");
        let mut keys = kv.keys().await?;
        let mut restore_count = 0;

        while let Some(key_res) = keys.next().await {
            if let Ok(key) = key_res {
                if key.starts_with("app_") {
                    if let Ok(Some(entry)) = kv.get(&key).await {
                        if let Ok(manifest) = serde_json::from_slice::<AppManifest>(&entry) {
                            if let Err(e) = self.register_app(manifest).await {
                                tracing::warn!(
                                    "⚠️ Failed to restore app from key '{}': {}",
                                    key,
                                    e
                                );
                            } else {
                                restore_count += 1;
                            }
                        }
                    }
                }
            }
        }

        if restore_count > 0 {
            tracing::info!("🔄 Restored {} applications from KV", restore_count);
        } else {
            tracing::info!("✅ No dynamic applications to restore");
        }

        Ok(())
    }

    /// Run the main NATS subscriber loop for app registration and unregistration.
    pub async fn listen_for_registrations(&self) -> anyhow::Result<()> {
        let register_sub = self
            .nats
            .subscribe("infra.v1.app.register".to_string())
            .await?;
        let unregister_sub = self
            .nats
            .subscribe("infra.v1.app.unregister".to_string())
            .await?;

        tracing::info!("📬 AppRegistry listening for registration events on NATS...");

        let mut stream = futures::stream::select(
            register_sub.map(|msg| (true, msg)),
            unregister_sub.map(|msg| (false, msg)),
        );

        while let Some((is_register, msg)) = stream.next().await {
            if is_register {
                match serde_json::from_slice::<AppManifest>(&msg.payload) {
                    Ok(manifest) => {
                        let app_id = manifest.app.id.clone();
                        match self.register_app(manifest).await {
                            Ok(_) => {
                                let confirmation = json!({
                                    "app_id": app_id,
                                    "status": "registered",
                                    "timestamp": chrono::Utc::now().timestamp(),
                                });
                                if let Ok(payload) = serde_json::to_vec(&confirmation) {
                                    let _ = self
                                        .nats
                                        .publish(
                                            "infra.v1.app.registered".to_string(),
                                            payload.into(),
                                        )
                                        .await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to register app: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ Invalid AppManifest payload received on NATS: {}", e);
                    }
                }
            } else {
                match serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                    Ok(payload) => {
                        if let Some(app_id) = payload["app_id"].as_str() {
                            match self.unregister_app(app_id).await {
                                Ok(_) => {
                                    let confirmation = json!({
                                        "app_id": app_id,
                                        "status": "unregistered",
                                        "timestamp": chrono::Utc::now().timestamp(),
                                    });
                                    if let Ok(payload) = serde_json::to_vec(&confirmation) {
                                        let _ = self
                                            .nats
                                            .publish(
                                                "infra.v1.app.registered".to_string(),
                                                payload.into(),
                                            )
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "❌ Failed to unregister app '{}': {}",
                                        app_id,
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ Invalid unregister payload received on NATS: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}
