use crate::error::ContractError;
use crate::model::CapabilityBinding;
use serde_json::Value;

/// Translates a contract-level capability operation and argument payload into the target tool's native invocation format.
pub fn translate_contract_action(
    binding: &CapabilityBinding,
    operation: &str,
    arguments: &Value,
) -> Result<(String, Value), ContractError> {
    // 1. Resolve operation mapping if present
    let mapped_op = binding
        .operation_mappings
        .get(operation)
        .cloned()
        .unwrap_or_else(|| operation.to_string());

    // 2. Perform field transformations on arguments if argument is a JSON object
    let translated_args = if let Value::Object(map) = arguments {
        let mut new_map = serde_json::Map::new();
        for (k, v) in map {
            let target_key = binding
                .field_mappings
                .get(k)
                .cloned()
                .unwrap_or_else(|| k.clone());
            new_map.insert(target_key, v.clone());
        }
        Value::Object(new_map)
    } else {
        arguments.clone()
    };

    Ok((mapped_op, translated_args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_dynamic_interface_translation() {
        let mut op_mappings = HashMap::new();
        op_mappings.insert(
            "dispatch_freight".to_string(),
            "local_carrier_dispatch".to_string(),
        );

        let mut field_mappings = HashMap::new();
        field_mappings.insert("destination_address".to_string(), "target_addr".to_string());
        field_mappings.insert("cargo_weight_kg".to_string(), "weight".to_string());

        let binding = CapabilityBinding {
            semantic_operation: "io.logistics.freight.dispatch@v1".to_string(),
            tool_id: "internal_freight_system".to_string(),
            schema_version: "1.0".to_string(),
            operation_mappings: op_mappings,
            field_mappings,
        };

        let incoming_args = serde_json::json!({
            "destination_address": "123 Port Way",
            "cargo_weight_kg": 450,
            "priority": "standard"
        });

        let (target_op, transformed_args) =
            translate_contract_action(&binding, "dispatch_freight", &incoming_args).unwrap();

        assert_eq!(target_op, "local_carrier_dispatch");
        assert_eq!(transformed_args["target_addr"], "123 Port Way");
        assert_eq!(transformed_args["weight"], 450);
        assert_eq!(transformed_args["priority"], "standard");
        assert!(transformed_args.get("destination_address").is_none());
    }
}
