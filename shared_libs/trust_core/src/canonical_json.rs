// ─────────────────────────────────────────────────────────────
// Canonical JSON — deterministic serialization
// Re-exported from trust_verifier
// ─────────────────────────────────────────────────────────────

pub use trust_verifier::{canonical_hash, canonical_json};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_json_sorted_keys() {
        let json = serde_json::json!({
            "zebra": 1,
            "alpha": 2,
            "middle": 3,
        });
        let result = canonical_json(&json);
        assert_eq!(result, r#"{"alpha":2,"middle":3,"zebra":1}"#);
    }
}
