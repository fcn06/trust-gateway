use crate::error::ContractError;
use crate::model::DataPolicy;

/// Evaluates an egress response payload against the contract's DataPolicy constraints.
pub fn evaluate_response_data_policy(
    policy: &DataPolicy,
    response_payload: &serde_json::Value,
    payload_byte_len: usize,
) -> Result<(), ContractError> {
    // 1. Max response byte limit check
    if let Some(max_bytes) = policy.max_response_bytes {
        if payload_byte_len > max_bytes as usize {
            return Err(ContractError::EgressPolicyViolation(format!(
                "Response size ({payload_byte_len} bytes) exceeds contract limit ({max_bytes} bytes)"
            )));
        }
    }

    // 2. Prohibited data classifications scan
    if !policy.prohibited_classes.is_empty() {
        let serialized = serde_json::to_string(response_payload).unwrap_or_default();

        for class in &policy.prohibited_classes {
            match class.to_lowercase().as_str() {
                "pii" | "email" => {
                    if contains_email(&serialized) {
                        return Err(ContractError::EgressPolicyViolation(
                            "Response contains prohibited PII (email address) per contract DataPolicy"
                                .to_string(),
                        ));
                    }
                }
                "financial" | "credit_card" => {
                    if contains_credit_card(&serialized) {
                        return Err(ContractError::EgressPolicyViolation(
                            "Response contains prohibited Financial data (credit card) per contract DataPolicy"
                                .to_string(),
                        ));
                    }
                }
                "credentials" | "api_key" => {
                    if contains_secret_key(&serialized) {
                        return Err(ContractError::EgressPolicyViolation(
                            "Response contains prohibited Credentials/API Keys per contract DataPolicy"
                                .to_string(),
                        ));
                    }
                }
                custom_tag => {
                    // Check if custom prohibited tag appears in response keys or values
                    if serialized.contains(custom_tag) {
                        return Err(ContractError::EgressPolicyViolation(format!(
                            "Response contains prohibited classification '{custom_tag}' per contract DataPolicy"
                        )));
                    }
                }
            }
        }
    }

    Ok(())
}

fn contains_email(s: &str) -> bool {
    s.contains('@')
        && (s.contains(".com") || s.contains(".org") || s.contains(".net") || s.contains(".io"))
}

fn contains_credit_card(s: &str) -> bool {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.len() >= 16
        && (digits.starts_with('4')
            || digits.starts_with("51")
            || digits.starts_with("52")
            || digits.starts_with("53")
            || digits.starts_with("54")
            || digits.starts_with("55")
            || digits.starts_with("37"))
}

fn contains_secret_key(s: &str) -> bool {
    s.contains("sk-") || s.contains("AKIA") || s.contains("bearer ") || s.contains("token=")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_bytes_enforcement() {
        let policy = DataPolicy {
            allowed_classes: vec!["confidential".to_string()],
            max_response_bytes: Some(100),
            prohibited_classes: vec![],
            retention_days: None,
        };

        let val = serde_json::json!({ "status": "ok" });
        assert!(evaluate_response_data_policy(&policy, &val, 50).is_ok());
        assert!(evaluate_response_data_policy(&policy, &val, 150).is_err());
    }

    #[test]
    fn test_prohibited_pii_rejection() {
        let policy = DataPolicy {
            allowed_classes: vec!["confidential".to_string()],
            max_response_bytes: None,
            prohibited_classes: vec!["pii".to_string()],
            retention_days: None,
        };

        let clean_val = serde_json::json!({ "user_id": "usr_123" });
        assert!(evaluate_response_data_policy(&policy, &clean_val, 20).is_ok());

        let pii_val = serde_json::json!({ "email": "john.doe@example.com" });
        assert!(evaluate_response_data_policy(&policy, &pii_val, 35).is_err());
    }
}
