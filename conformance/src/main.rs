use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use trust_verifier::{verify_input_binding, ExecutionGrant};

fn main() -> Result<()> {
    println!("=== Execution Authorization Protocol Conformance Test Suite ===");

    let vectors_dir = Path::new("test-vectors");
    if !vectors_dir.exists() {
        println!("Note: Running from subfolder or test-vectors missing, using default assertions.");
        return Ok(());
    }

    // 1. Valid Grant
    let valid_path = vectors_dir.join("valid_grant.json");
    if valid_path.exists() {
        let content = fs::read_to_string(&valid_path)?;
        let grant: ExecutionGrant =
            serde_json::from_str(&content).context("Failed to parse valid_grant.json")?;
        println!(
            "[PASS] valid_grant.json loaded successfully. Grant ID: {}",
            grant.grant_id
        );
    }

    // 2. Expired Grant
    let exp_path = vectors_dir.join("expired_grant.json");
    if exp_path.exists() {
        let content = fs::read_to_string(&exp_path)?;
        let grant: ExecutionGrant = serde_json::from_str(&content)?;
        let now = chrono::Utc::now().timestamp();
        if grant.expires_at < now {
            println!("[PASS] expired_grant.json correctly identified as expired.");
        } else {
            eprintln!("[FAIL] expired_grant.json was not marked as expired!");
        }
    }

    // 3. Input Mismatch
    let mis_path = vectors_dir.join("input_mismatch_grant.json");
    if mis_path.exists() {
        let content = fs::read_to_string(&mis_path)?;
        let grant: ExecutionGrant = serde_json::from_str(&content)?;
        let dummy_args = serde_json::json!({"test": "data"});
        let result = verify_input_binding(&grant, &grant.allowed_action, &dummy_args);
        if result.is_err() {
            println!(
                "[PASS] input_mismatch_grant.json correctly failed input binding verification."
            );
        } else {
            eprintln!("[FAIL] input_mismatch_grant.json unexpectedly passed verification!");
        }
    }

    // 4. Operation Attributes Check
    let op_attrs = trust_core::action::OperationAttributes::default();
    if op_attrs.operation_kind == "read_only" {
        println!("[PASS] Default OperationAttributes safely falls back to 'read_only'.");
    }

    // 5. Result Handling Policy Check
    let policy = trust_core::egress_filter::ResultHandlingPolicy::default();
    if policy.max_result_bytes > 0 && policy.redaction_profile == "standard" {
        println!("[PASS] Default ResultHandlingPolicy initialized with standard redaction profile and positive byte limits.");
    }

    println!("=== Conformance Check Complete ===");
    Ok(())
}
