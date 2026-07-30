use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use trust_core::errors::TrustError;
use trust_core::executor::{Executor, VerifiedGrant};

#[derive(Clone)]
pub struct VpExecutor {
    http_client: reqwest::Client,
    nats: async_nats::Client,
}

impl VpExecutor {
    pub fn new(nats: async_nats::Client) -> Result<Self, TrustError> {
        Ok(Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| TrustError::Internal(format!("Failed to build http client: {e}")))?,
            nats,
        })
    }
}

#[async_trait]
impl Executor for VpExecutor {
    fn name(&self) -> &str {
        "vp"
    }

    fn handles(&self, tool_id: &str) -> bool {
        matches!(
            tool_id,
            "vp_search"
                | "discover_agent_services"
                | "call_b2b_agent"
                | "register_b2b_agent"
                | "list_registered_b2b_agents"
                | "discover_b2b_agents"
        )
    }

    async fn execute(
        &self,
        grant: VerifiedGrant,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        match grant.allowed_action() {
            "vp_search" => self.execute_search(args).await,
            "discover_agent_services" => self.execute_discover(grant, args).await,
            "call_b2b_agent" => self.execute_call_b2b(grant, args).await,
            "register_b2b_agent" => self.execute_register_b2b(grant, args).await,
            "list_registered_b2b_agents" => self.execute_list_b2b(grant, args).await,
            "discover_b2b_agents" => self.execute_discover_b2b(grant, args).await,
            _ => Err(TrustError::Internal(format!(
                "Unsupported VP tool: {}",
                grant.allowed_action()
            ))),
        }
    }
}

impl VpExecutor {
    async fn execute_search(
        &self,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        let query = args
            .get("search_query")
            .or_else(|| args.get("query"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        tracing::info!("🔍 [VP Search] Query: '{}'", query);

        if query.is_empty() {
            return Ok(serde_json::json!({ "error": "Search query is empty" }));
        }

        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json",
            urlencoding::encode(query)
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| TrustError::Internal(format!("Search request failed: {e}")))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TrustError::Internal(format!("Failed to parse search response: {e}")))?;

        // Extract multiple fields for a richer result
        let abstract_text = body
            .get("AbstractText")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let abstract_source = body
            .get("AbstractSource")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let heading = body.get("Heading").and_then(|v| v.as_str()).unwrap_or("");

        let mut result_text = String::new();

        if !heading.is_empty() {
            result_text.push_str(&format!("## {heading}\n\n"));
        }

        if !abstract_text.is_empty() {
            result_text.push_str(&format!(
                "Summary (from {abstract_source}): {abstract_text}\n\n"
            ));
        }

        if let Some(related) = body.get("RelatedTopics").and_then(|v| v.as_array()) {
            if !related.is_empty() {
                result_text.push_str("### Related Information:\n");
                for (i, topic) in related.iter().enumerate() {
                    if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                        result_text.push_str(&format!("{}. {}\n", i + 1, text));
                    }
                    if i >= 5 {
                        break;
                    } // Limit to top 6 related topics
                }
            }
        }

        if result_text.trim().is_empty() {
            result_text = format!("No specific information found for '{query}' on DuckDuckGo.");
        }

        tracing::info!("✅ [VP Search] Returning {} chars", result_text.len());
        Ok(serde_json::Value::String(result_text))
    }

    async fn execute_discover(
        &self,
        grant: VerifiedGrant,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        let target_did = args
            .get("target_did")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if target_did.is_empty() {
            return Err(TrustError::Internal(
                "Missing target_did argument".to_string(),
            ));
        }

        let query_thid = uuid::Uuid::new_v4().to_string();
        let reply_subject = format!("mcp.v1.discovery.reply.{query_thid}");

        // 1. Subscribe to reply subject
        let mut subscriber = self
            .nats
            .subscribe(reply_subject.clone())
            .await
            .map_err(|e| TrustError::Internal(format!("Failed to subscribe: {e}")))?;

        // 2. Publish request
        let payload = serde_json::json!({
            "target_did": target_did,
            "requester_did": grant.owner_did(),
            "query_thid": query_thid,
        })
        .to_string();

        tracing::info!(
            "📡 Publishing discover request for {} (reply expected on {})",
            target_did,
            reply_subject
        );
        self.nats
            .publish("host.v1.discovery.request".to_string(), payload.into())
            .await
            .map_err(|e| {
                TrustError::Internal(format!("Failed to publish discovery request: {e}"))
            })?;

        // 3. Await reply
        match tokio::time::timeout(std::time::Duration::from_secs(15), subscriber.next()).await {
            Ok(Some(msg)) => {
                let payload_str = String::from_utf8_lossy(&msg.payload).to_string();
                let parsed: serde_json::Value = serde_json::from_str(&payload_str)
                    .unwrap_or_else(|_| serde_json::json!({ "raw_response": payload_str }));
                Ok(parsed)
            }
            Ok(None) => Err(TrustError::Internal(
                "NATS subscription closed prematurely".to_string(),
            )),
            Err(_) => Err(TrustError::Internal(
                "Discovery request timed out waiting for reply".to_string(),
            )),
        }
    }

    async fn get_b2b_kv(&self) -> Result<async_nats::jetstream::kv::Store, TrustError> {
        let js = async_nats::jetstream::new(self.nats.clone());
        match js.get_key_value("b2b_agents").await {
            Ok(kv) => Ok(kv),
            Err(_) => js
                .create_key_value(async_nats::jetstream::kv::Config {
                    bucket: "b2b_agents".to_string(),
                    history: 1,
                    max_age: std::time::Duration::from_secs(365 * 24 * 3600),
                    ..Default::default()
                })
                .await
                .map_err(|e| TrustError::Internal(format!("Failed to create b2b_agents KV: {e}"))),
        }
    }

    async fn execute_register_b2b(
        &self,
        grant: VerifiedGrant,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        let alias = args.get("alias").and_then(|v| v.as_str()).unwrap_or("");
        let b2b_agent_did = args
            .get("b2b_agent_did")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let endpoint_url = args
            .get("endpoint_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if alias.is_empty() || b2b_agent_did.is_empty() || endpoint_url.is_empty() {
            return Err(TrustError::Internal(
                "Missing alias, b2b_agent_did, or endpoint_url".to_string(),
            ));
        }

        let kv = self.get_b2b_kv().await?;
        let tenant = grant.tenant_id().replace(":", "_");
        let key = format!("{}_{}", tenant, alias.replace(":", "_"));

        let record = serde_json::json!({
            "alias": alias,
            "b2b_agent_did": b2b_agent_did,
            "endpoint_url": endpoint_url
        });

        kv.put(
            key,
            serde_json::to_vec(&record)
                .map_err(|e| TrustError::Internal(e.to_string()))?
                .into(),
        )
        .await
        .map_err(|e| TrustError::Internal(format!("Failed to save B2B agent: {e}")))?;

        tracing::info!(
            "📇 Registered B2B agent: {} (DID: {})",
            alias,
            b2b_agent_did
        );
        Ok(serde_json::json!({
            "status": "registered",
            "alias": alias,
            "b2b_agent_did": b2b_agent_did,
            "endpoint_url": endpoint_url
        }))
    }

    async fn execute_list_b2b(
        &self,
        grant: VerifiedGrant,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        let kv = self.get_b2b_kv().await?;
        let tenant = grant.tenant_id().replace(":", "_");
        let prefix = format!("{tenant}_");

        let mut keys_stream = kv
            .keys()
            .await
            .map_err(|e| TrustError::Internal(format!("Failed to list keys: {e}")))?;

        let mut results = Vec::new();
        while let Some(Ok(key)) = keys_stream.next().await {
            if key.starts_with(&prefix) {
                if let Ok(Some(bytes)) = kv.get(&key).await {
                    if let Ok(record) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        results.push(record);
                    }
                }
            }
        }

        Ok(serde_json::Value::Array(results))
    }

    async fn execute_discover_b2b(
        &self,
        _grant: VerifiedGrant,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        let query = args
            .get("search_query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let mut mock_list = self.load_b2b_directory();

        if !query.is_empty() {
            mock_list.retain(|item| {
                let alias = item["alias"].as_str().unwrap_or("").to_lowercase();
                let did = item["b2b_agent_did"].as_str().unwrap_or("").to_lowercase();
                alias.contains(&query) || did.contains(&query)
            });
        }

        Ok(serde_json::Value::Array(mock_list))
    }

    async fn execute_call_b2b(
        &self,
        grant: VerifiedGrant,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, TrustError> {
        let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("");
        let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
        let session_jwt = args
            .get("session_jwt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let passport_hash = args
            .get("passport_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if target.is_empty() || prompt.is_empty() {
            return Err(TrustError::Internal(
                "Missing target or prompt argument".to_string(),
            ));
        }
        if session_jwt.is_empty() {
            return Err(TrustError::Internal(
                "Missing session_jwt in call_b2b_agent arguments".to_string(),
            ));
        }

        // 1. Resolve B2B agent endpoint and DID
        let kv = self.get_b2b_kv().await?;
        let tenant = grant.tenant_id().replace(":", "_");
        let prefix = format!("{tenant}_");

        let mut resolved_url = None;
        let mut resolved_did = None;

        if target.starts_with("did:") {
            let mut keys_stream = kv
                .keys()
                .await
                .map_err(|e| TrustError::Internal(format!("Failed to list keys: {e}")))?;
            while let Some(Ok(key)) = keys_stream.next().await {
                if key.starts_with(&prefix) {
                    if let Ok(Some(bytes)) = kv.get(&key).await {
                        if let Ok(record) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if record["b2b_agent_did"].as_str() == Some(target) {
                                resolved_url =
                                    record["endpoint_url"].as_str().map(|s| s.to_string());
                                resolved_did = Some(target.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        } else {
            let key = format!("{}_{}", tenant, target.replace(":", "_"));
            if let Ok(Some(bytes)) = kv.get(&key).await {
                if let Ok(record) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    resolved_url = record["endpoint_url"].as_str().map(|s| s.to_string());
                    resolved_did = record["b2b_agent_did"].as_str().map(|s| s.to_string());
                }
            }
        }

        // Fallback for strategic partners from config
        if resolved_url.is_none() {
            let directory = self.load_b2b_directory();
            for item in directory {
                if let (Some(alias), Some(did), Some(url)) = (
                    item["alias"].as_str(),
                    item["b2b_agent_did"].as_str(),
                    item["endpoint_url"].as_str(),
                ) {
                    if target == alias || target == did {
                        resolved_url = Some(url.to_string());
                        resolved_did = Some(did.to_string());
                        break;
                    }
                }
            }
        }

        let url = resolved_url.ok_or_else(|| {
            TrustError::Internal(format!(
                "Failed to resolve B2B agent endpoint for target: {target}"
            ))
        })?;
        let b2b_did = resolved_did.unwrap_or_else(|| target.to_string());

        tracing::info!(
            "📞 Calling B2B agent at {} (DID: {}) with user session token...",
            url,
            b2b_did
        );

        // 2. Build A2A JSON-RPC payload
        let req_id = uuid::Uuid::new_v4().to_string();
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let msg_id = format!("msg-{}", uuid::Uuid::new_v4());

        let mut metadata = serde_json::json!({
            "agent_jwt": session_jwt,
            "tenant_id": grant.tenant_id()
        });
        if !passport_hash.is_empty() {
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert(
                    "passport_hash".to_string(),
                    serde_json::Value::String(passport_hash.to_string()),
                );
            }
        }

        let json_rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tasks/send",
            "params": {
                "id": task_id,
                "historyLength": 50,
                "message": {
                    "kind": "message",
                    "messageId": msg_id,
                    "metadata": metadata,
                    "parts": [
                        {
                            "kind": "text",
                            "text": prompt
                        }
                    ],
                    "role": "user"
                }
            },
            "id": req_id
        });

        // 3. Make HTTP POST call to target endpoint with Bearer auth
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {session_jwt}"))
            .json(&json_rpc_payload)
            .send()
            .await
            .map_err(|e| TrustError::Internal(format!("Failed to contact B2B agent: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TrustError::Internal(format!(
                "B2B agent returned error status {status}: {body}"
            )));
        }

        let res_val: serde_json::Value = response.json().await.map_err(|e| {
            TrustError::Internal(format!("Failed to parse B2B agent response: {e}"))
        })?;

        // Extract result from JSON-RPC envelope
        if let Some(error) = res_val.get("error").filter(|e| !e.is_null()) {
            return Err(TrustError::Internal(format!(
                "B2B agent execution error: {error}"
            )));
        }

        let output = res_val.get("result").cloned().unwrap_or(res_val);

        Ok(output)
    }

    fn load_b2b_directory(&self) -> Vec<serde_json::Value> {
        let path = std::env::var("B2B_DIRECTORY_PATH")
            .unwrap_or_else(|_| "../../agent_in_a_box/host/config/b2b_directory.json".to_string());

        match std::fs::File::open(&path) {
            Ok(file) => match serde_json::from_reader::<_, Vec<serde_json::Value>>(file) {
                Ok(list) => {
                    tracing::info!(
                        "✅ Loaded {} B2B agents from directory config: {}",
                        list.len(),
                        path
                    );
                    list
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠️ Failed to parse B2B directory JSON at {}: {}. Using fallback mock.",
                        path,
                        e
                    );
                    self.get_fallback_mock_list()
                }
            },
            Err(_) => {
                if std::env::var("B2B_DIRECTORY_PATH").is_ok() {
                    tracing::warn!("⚠️ B2B directory file not found at path: {}", path);
                }
                self.get_fallback_mock_list()
            }
        }
    }

    fn get_fallback_mock_list(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "alias": "company-alpha",
                "b2b_agent_did": "did:web:company-alpha.com",
                "endpoint_url": "http://127.0.0.1:4010"
            }),
            serde_json::json!({
                "alias": "company-beta",
                "b2b_agent_did": "did:web:company-beta.com",
                "endpoint_url": "http://127.0.0.1:4010"
            }),
            serde_json::json!({
                "alias": "company-gamma",
                "b2b_agent_did": "did:web:company-gamma.com",
                "endpoint_url": "http://127.0.0.1:4010"
            }),
        ]
    }
}
