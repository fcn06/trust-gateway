use anyhow::{Context, Result};
use trust_verifier::{verify_input_binding, ExecutionGrant};

fn main() -> Result<()> {
    println!("=== Standalone Rust Executor Example ===");

    let grant_json = serde_json::json!({
        "grant_id": "grant_rust_001",
        "action_id": "act_999",
        "tenant_id": "tenant_prod",
        "owner_did": "did:example:user",
        "requester_did": "did:example:agent",
        "allowed_action": "google.calendar.event.create",
        "clearance": "auto_approved",
        "expires_at": chrono::Utc::now().timestamp() + 300,
        "input_hash": trust_verifier::canonical_hash(&serde_json::json!({"duration": 30, "title": "Team Sync"}))
    });

    let grant: ExecutionGrant = serde_json::from_value(grant_json)?;
    let arguments = serde_json::json!({"title": "Team Sync", "duration": 30});

    verify_input_binding(&grant, "google.calendar.event.create", &arguments)
        .context("ExecutionGrant verification failed!")?;

    println!("✅ Grant successfully verified! Executing tool action...");
    println!("Action: {}", grant.allowed_action);
    println!("Arguments: {}", serde_json::to_string_pretty(&arguments)?);

    Ok(())
}
