use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use trust_egress::EgressFilter;
use trust_executor_sdk::{verify_input_hash, Executor};
use trust_model::{ExecutionResult, GrantedAction, TransactionOutcomeState};

pub type ToolHandler =
    Arc<dyn Fn(&serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>;

pub struct ReferenceExecutor {
    handlers: Arc<Mutex<HashMap<String, ToolHandler>>>,
}

impl Default for ReferenceExecutor {
    fn default() -> Self {
        let mut exec = Self::new();
        exec.register_handler("mock_echo", Arc::new(|args| Ok(args.clone())));
        exec.register_handler(
            "mock_refund",
            Arc::new(|args| {
                let amount = args.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
                Ok(serde_json::json!({
                    "status": "refund_processed",
                    "amount": amount,
                    "account_email": "user@example.com"
                }))
            }),
        );
        exec
    }
}

impl ReferenceExecutor {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_handler(&mut self, tool_name: &str, handler: ToolHandler) {
        self.handlers
            .lock()
            .unwrap()
            .insert(tool_name.to_string(), handler);
    }
}

#[async_trait]
impl Executor for ReferenceExecutor {
    async fn execute(&self, action: GrantedAction) -> Result<ExecutionResult, anyhow::Error> {
        let start = std::time::Instant::now();

        // 1. Verify input_hash binding
        verify_input_hash(&action.grant.input_hash, &action.action_arguments)?;

        // 2. Lookup registered handler
        let handler = {
            let guard = self.handlers.lock().unwrap();
            guard.get(&action.grant.tool_name).cloned()
        };

        let (raw_output, is_success) = match handler {
            Some(h) => match h(&action.action_arguments) {
                Ok(val) => (val, true),
                Err(err) => (serde_json::json!({ "error": err }), false),
            },
            None => (
                serde_json::json!({ "error": format!("Tool '{}' not found in ReferenceExecutor", action.grant.tool_name) }),
                false,
            ),
        };

        // 3. Apply PII egress filter sanitization
        let serialized = serde_json::to_string(&raw_output)?;
        let sanitized = EgressFilter::sanitize_text(&serialized);
        let sanitized_output: serde_json::Value = serde_json::from_str(&sanitized)?;

        let status = if is_success {
            TransactionOutcomeState::Succeeded
        } else {
            TransactionOutcomeState::Failed
        };

        Ok(ExecutionResult {
            action_id: action.grant.action_id.clone(),
            status,
            connector: "reference_executor".to_string(),
            external_reference: Some(format!("ref_{}", action.grant.grant_id)),
            output: sanitized_output,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
