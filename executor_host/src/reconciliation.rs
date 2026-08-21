use serde::{Deserialize, Serialize};
use trust_core::action::{ActionResult, ActionStatus, TransactionLifecycleState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTracker {
    pub action_id: String,
    pub grant_id: String,
    #[serde(default = "default_tenant_id")]
    pub tenant_id: String,
    #[serde(default = "default_workspace_id")]
    pub workspace_id: String,
    pub idempotency_key: String,
    pub state: TransactionLifecycleState,
    pub created_at_epoch: i64,
}

fn default_tenant_id() -> String {
    "default".to_string()
}

fn default_workspace_id() -> String {
    "default".to_string()
}

impl TransactionTracker {
    pub fn new(
        action_id: impl Into<String>,
        grant_id: impl Into<String>,
        tenant_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let action_id = action_id.into();
        let grant_id = grant_id.into();
        let tenant_id = tenant_id.into();
        let workspace_id = workspace_id.into();
        let idempotency_key = format!("idemp_{}_{}_{}", tenant_id, workspace_id, grant_id);
        Self {
            action_id,
            grant_id,
            tenant_id,
            workspace_id,
            idempotency_key,
            state: TransactionLifecycleState::Dispatched,
            created_at_epoch: chrono::Utc::now().timestamp(),
        }
    }

    /// Returns the deterministic upstream SaaS provider idempotency key.
    pub fn provider_idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
}

pub struct ReconciliationHandler;

impl ReconciliationHandler {
    /// Reconciles an execution state when a network crash or timeout occurs.
    /// Uses provider_idempotency_key to prevent replaying side effects.
    pub fn handle_crash_reconciliation(
        tracker: &mut TransactionTracker,
        external_status: Option<&str>,
    ) -> ActionResult {
        match external_status {
            Some("confirmed") | Some("succeeded") => {
                tracker.state = TransactionLifecycleState::SideEffectConfirmed;
                ActionResult {
                    action_id: tracker.action_id.clone(),
                    status: ActionStatus::Succeeded,
                    connector: "reconciled_executor".to_string(),
                    external_reference: Some(tracker.idempotency_key.clone()),
                    output: serde_json::json!({
                        "reconciled": true,
                        "provider_idempotency_key": tracker.idempotency_key,
                        "state": "side_effect_confirmed"
                    }),
                }
            }
            _ => {
                tracker.state = TransactionLifecycleState::ReconciliationRequired;
                ActionResult {
                    action_id: tracker.action_id.clone(),
                    status: ActionStatus::Failed,
                    connector: "reconciled_executor".to_string(),
                    external_reference: None,
                    output: serde_json::json!({
                        "reconciled": false,
                        "provider_idempotency_key": tracker.idempotency_key,
                        "state": "reconciliation_required",
                        "error": "Execution state unknown post-crash: side-effect must not be blindly replayed"
                    }),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_provider_idempotency_key() {
        let tracker = TransactionTracker::new(
            "act_100",
            "grant_200",
            "tenant_alpha",
            "ws_prod",
        );
        assert_eq!(
            tracker.provider_idempotency_key(),
            "idemp_tenant_alpha_ws_prod_grant_200"
        );
        // Assert INV-009: composite key strictly uses underscore '_'
        assert!(!tracker.provider_idempotency_key().contains(':'));
    }

    #[test]
    fn test_crash_reconciliation_confirmed() {
        let mut tracker = TransactionTracker::new(
            "act_100",
            "grant_200",
            "tenant_alpha",
            "ws_prod",
        );
        tracker.state = TransactionLifecycleState::ExecutionStarted;

        let result = ReconciliationHandler::handle_crash_reconciliation(
            &mut tracker,
            Some("confirmed"),
        );

        assert_eq!(result.status, ActionStatus::Succeeded);
        assert_eq!(tracker.state, TransactionLifecycleState::SideEffectConfirmed);
        assert_eq!(
            result.external_reference,
            Some("idemp_tenant_alpha_ws_prod_grant_200".to_string())
        );
    }

    #[test]
    fn test_crash_reconciliation_unknown_does_not_replay() {
        let mut tracker = TransactionTracker::new(
            "act_100",
            "grant_200",
            "tenant_alpha",
            "ws_prod",
        );
        tracker.state = TransactionLifecycleState::ExecutionStarted;

        let result = ReconciliationHandler::handle_crash_reconciliation(
            &mut tracker,
            None,
        );

        assert_eq!(result.status, ActionStatus::Failed);
        assert_eq!(
            tracker.state,
            TransactionLifecycleState::ReconciliationRequired
        );
    }
}

