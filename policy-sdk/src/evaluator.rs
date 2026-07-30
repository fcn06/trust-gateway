use crate::layers::{HierarchicalPolicy, PolicyOutcome};

/// 4-Layer Hierarchical Policy Evaluator
///
/// Implements monotonic intersection:
/// Platform ∩ Organization ∩ Agent ∩ Transaction
pub struct PolicyEvaluator {
    policy: HierarchicalPolicy,
}

impl PolicyEvaluator {
    pub fn new(policy: HierarchicalPolicy) -> Self {
        Self { policy }
    }

    /// Evaluates an execution grant proposal against all 4 policy layers in order.
    pub fn evaluate(
        &self,
        tenant_id: &str,
        agent_id: &str,
        tool_name: &str,
        amount_usd: Option<u64>,
    ) -> PolicyOutcome {
        // Layer 1: Platform Policy Invariants
        if self.policy.platform.enforce_tenant_isolation && tenant_id.is_empty() {
            return PolicyOutcome::Deny {
                reason: "Platform Policy: Missing tenant isolation context".to_string(),
            };
        }

        // Layer 2: Organization Policy
        if self
            .policy
            .organization
            .blacklisted_tools
            .contains(&tool_name.to_string())
        {
            return PolicyOutcome::Deny {
                reason: format!("Organization Policy: Tool '{tool_name}' is blacklisted"),
            };
        }
        if let Some(amt) = amount_usd {
            if amt > self.policy.organization.max_financial_limit_usd
                && self.policy.organization.max_financial_limit_usd > 0
            {
                return PolicyOutcome::Deny {
                    reason: format!(
                        "Organization Policy: Transaction amount (${}) exceeds organizational cap (${})",
                        amt, self.policy.organization.max_financial_limit_usd
                    ),
                };
            }
        }

        // Layer 3: Agent Policy
        if !self.policy.agent.agent_id.is_empty() && self.policy.agent.agent_id != agent_id {
            return PolicyOutcome::Deny {
                reason: format!(
                    "Agent Policy: Agent ID mismatch (expected '{}', got '{}')",
                    self.policy.agent.agent_id, agent_id
                ),
            };
        }
        if !self.policy.agent.allowed_tools.is_empty()
            && !self
                .policy
                .agent
                .allowed_tools
                .contains(&tool_name.to_string())
        {
            return PolicyOutcome::Deny {
                reason: format!("Agent Policy: Tool '{tool_name}' not permitted for agent profile"),
            };
        }

        // Layer 4: Transaction Policy
        if let Some(threshold) = self.policy.transaction.human_approval_threshold_usd {
            if let Some(amt) = amount_usd {
                if amt >= threshold {
                    return PolicyOutcome::RequiresHumanApproval {
                        clearance_required: "human_approved".to_string(),
                    };
                }
            }
        }

        PolicyOutcome::Allow
    }

    /// Evaluates an action using explicit OperationAttributes instead of heuristics.
    pub fn evaluate_operation(
        &self,
        tenant_id: &str,
        agent_id: &str,
        tool_name: &str,
        op_attrs: &trust_model::OperationAttributes,
    ) -> PolicyOutcome {
        let amount_usd = op_attrs.amount.as_ref().map(|m| m.amount_cents / 100);

        if op_attrs.operation_kind == "financial_mutation"
            || op_attrs.operation_kind == "destructive"
        {
            if let Some(threshold) = self.policy.transaction.human_approval_threshold_usd {
                let amt = amount_usd.unwrap_or(0);
                if amt >= threshold {
                    return PolicyOutcome::RequiresHumanApproval {
                        clearance_required: "human_approved".to_string(),
                    };
                }
            }
        }

        self.evaluate(tenant_id, agent_id, tool_name, amount_usd)
    }
}
