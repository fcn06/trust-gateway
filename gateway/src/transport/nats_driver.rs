// ─────────────────────────────────────────────────────────────
// NatsMcpDriver — NATS JetStream transport for built-in executors.
//
// Translates `TransportDispatchRequest` into the existing
// `exec.v1.<tenant>.<profile>.invoke` topology using the
// canonical `TrustEnvelope<GrantedAction>` format.
//
// This is the DEFAULT transport for all built-in tools and
// mirrors the logic previously inline in router.rs dispatch.
// ─────────────────────────────────────────────────────────────

use async_trait::async_trait;
use futures::StreamExt;
use trust_core::envelope::{GrantedAction, TrustEnvelope};
use trust_core::transport::{
    McpTransport, TransportDispatchRequest, TransportDispatchResult, TransportError,
    TransportToolDefinition, TransportType,
};

/// NATS JetStream transport driver.
///
/// Routes tool invocations through the existing NATS `exec.v1.>` topology.
/// The executor_host on the other end of the NATS bus verifies the
/// ExecutionGrant JWT and dispatches to the appropriate Executor trait
/// implementation.
pub struct NatsMcpDriver {
    nats: async_nats::Client,
    /// The executor profile name (e.g., "connector", "vp", "native-tool").
    profile: String,
    /// Human-readable ID for this driver instance.
    id: String,
    /// Policy fingerprint to embed in outgoing envelopes.
    policy_fingerprint: String,
}

impl NatsMcpDriver {
    /// Create a new NATS driver for a specific executor profile.
    pub fn new(
        nats: async_nats::Client,
        profile: impl Into<String>,
        policy_fingerprint: impl Into<String>,
    ) -> Self {
        let profile = profile.into();
        let id = format!("nats_{profile}");
        Self {
            nats,
            profile,
            id,
            policy_fingerprint: policy_fingerprint.into(),
        }
    }
}

#[async_trait]
impl McpTransport for NatsMcpDriver {
    fn transport_type(&self) -> TransportType {
        TransportType::Nats
    }

    fn driver_id(&self) -> &str {
        &self.id
    }

    async fn initialize(&mut self) -> Result<(), TransportError> {
        // NATS connection is managed externally — nothing to initialize.
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<TransportToolDefinition>, TransportError> {
        // NATS executors don't self-advertise tools — tools come from
        // the ToolRegistry (builtin_descriptors or dynamic registration).
        Ok(Vec::new())
    }

    async fn dispatch(
        &self,
        request: &TransportDispatchRequest,
    ) -> Result<TransportDispatchResult, TransportError> {
        let start = std::time::Instant::now();

        // 1. Build the reply subject for this invocation.
        let reply_subject = format!("exec.v1.reply.{}", uuid::Uuid::new_v4());
        let mut subscription = self
            .nats
            .subscribe(reply_subject.clone())
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to subscribe: {e}")))?;

        // 2. Construct the canonical TrustEnvelope<GrantedAction>.
        let granted_action = GrantedAction {
            grant_jwt: request.grant_jwt.clone(),
            tool_id: request.tool_name.clone(),
            tool_version: "1.0.0".to_string(),
            canonical_args: request.arguments.clone(),
            input_hash: request.input_hash.clone(),
            reply_subject: reply_subject.clone(),
        };

        let envelope = TrustEnvelope::new(&request.tenant_id, &request.action_id, granted_action)
            .with_policy_fingerprint(&self.policy_fingerprint);

        // 3. Publish to the executor's invoke subject.
        let publish_subject = format!("exec.v1.{}.{}.invoke", request.tenant_id, self.profile);
        let payload = serde_json::to_vec(&envelope)
            .map_err(|e| TransportError::ProtocolError(format!("Serialization failed: {e}")))?;

        self.nats
            .publish(publish_subject, payload.into())
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Publish failed: {e}")))?;

        // 4. Wait for response with timeout.
        let result =
            match tokio::time::timeout(std::time::Duration::from_secs(45), subscription.next())
                .await
            {
                Ok(Some(msg)) => {
                    let response: serde_json::Value = serde_json::from_slice(&msg.payload)
                        .map_err(|e| {
                            TransportError::ProtocolError(format!(
                                "Failed to parse response: {e}"
                            ))
                        })?;

                    let payload = response.get("payload").ok_or_else(|| {
                        TransportError::ProtocolError("Missing 'payload' in response".into())
                    })?;

                    let output = payload.get("output").cloned();
                    let error = payload
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let success = error.is_none()
                        && payload
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);

                    Ok(TransportDispatchResult {
                        success,
                        output,
                        error,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                Ok(None) => Err(TransportError::Unavailable(
                    "Executor disconnected before replying".into(),
                )),
                Err(_) => Err(TransportError::Timeout(45)),
            };

        // 5. Explicitly drop subscription to avoid leaks.
        drop(subscription);

        result
    }

    async fn health_check(&self) -> Result<(), TransportError> {
        // Verify NATS connection is alive.
        if self.nats.connection_state() == async_nats::connection::State::Disconnected {
            return Err(TransportError::Unavailable(
                "NATS connection is disconnected".into(),
            ));
        }
        Ok(())
    }
}
