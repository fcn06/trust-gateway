use serde::{Deserialize, Serialize};
use trust_core::action::{ActionResult, ActionStatus, TransactionLifecycleState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTracker {
    pub action_id: String,
    pub grant_id: String,
    pub idempotency_key: String,
    pub state: TransactionLifecycleState,
    pub created_at_epoch: i64,
}

pub struct ReconciliationHandler;

impl ReconciliationHandler {
    /// Reconciles an execution state when a network crash or timeout occurs.
    pub fn handle_crash_reconciliation(
        tracker: &mut TransactionTracker,
        external_status: Option<&str>,
    ) -> ActionResult {
        match external_status {
            Some("confirmed") => {
                tracker.state = TransactionLifecycleState::SideEffectConfirmed;
                ActionResult {
                    action_id: tracker.action_id.clone(),
                    status: ActionStatus::Succeeded,
                    connector: "reconciled_executor".to_string(),
                    external_reference: Some(tracker.idempotency_key.clone()),
                    output: serde_json::json!({
                        "reconciled": true,
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
                        "state": "reconciliation_required",
                        "error": "Execution state unknown post-crash"
                    }),
                }
            }
        }
    }
}
