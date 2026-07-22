// ─────────────────────────────────────────────────────────────
// Canonical JSON — deterministic serialization (RFC 8785 subset)
//
// Produces a byte-stable JSON representation suitable for
// cryptographic hashing. Keys are sorted lexicographically at
// all nesting levels and no extraneous whitespace is emitted.
// ─────────────────────────────────────────────────────────────

use sha2::{Digest, Sha256};

/// Serialize a JSON value into canonical form (sorted keys, no whitespace).
pub fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s))
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", items.join(","))
        }
        serde_json::Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let pairs: Vec<String> = keys
                .iter()
                .map(|k| {
                    let key_str =
                        serde_json::to_string(*k).unwrap_or_else(|_| format!("\"{}\"", k));
                    let val_str = canonical_json(obj.get(*k).unwrap());
                    format!("{}:{}", key_str, val_str)
                })
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
    }
}

/// Compute the SHA-256 hash of canonical JSON for a given value.
pub fn canonical_hash(value: &serde_json::Value) -> String {
    let canonical = canonical_json(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

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

    #[test]
    fn test_canonical_hash_deterministic() {
        let a = serde_json::json!({"action": "create", "resource": "event"});
        let b = serde_json::json!({"resource": "event", "action": "create"});
        assert_eq!(canonical_hash(&a), canonical_hash(&b));
    }
}
