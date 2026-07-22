use trust_model::{PolicyDecision, ProposedAction};

pub struct CorePolicyEngine;

impl CorePolicyEngine {
    /// Pure policy decision logic based on explicit operation attributes
    pub fn evaluate(action: &ProposedAction, max_amount_cents: u64) -> PolicyDecision {
        let op_kind = &action.operation_attributes.operation_kind;

        if op_kind == "destructive" {
            return PolicyDecision {
                action_id: action.action_id.clone(),
                approved: false,
                clearance_level: "denied".to_string(),
                policy_fingerprint: "policy_v1_destructive".to_string(),
                reason: "Destructive operations are blocked by default policy".to_string(),
            };
        }

        if op_kind == "financial_mutation" {
            if let Some(ref money) = action.operation_attributes.amount {
                if money.amount_cents > max_amount_cents && max_amount_cents > 0 {
                    return PolicyDecision {
                        action_id: action.action_id.clone(),
                        approved: false,
                        clearance_level: "human_approved".to_string(),
                        policy_fingerprint: "policy_v1_financial".to_string(),
                        reason: format!(
                            "Financial mutation of {} {} exceeds threshold of {} cents",
                            money.amount_cents, money.currency, max_amount_cents
                        ),
                    };
                }
            }
        }

        PolicyDecision {
            action_id: action.action_id.clone(),
            approved: true,
            clearance_level: "auto_approved".to_string(),
            policy_fingerprint: "policy_v1_allow".to_string(),
            reason: "Action permitted under default policy".to_string(),
        }
    }
}
