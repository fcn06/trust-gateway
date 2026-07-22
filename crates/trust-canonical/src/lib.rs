use sha2::{Digest, Sha256};

/// Recursively produces canonical JSON string with sorted object keys.
pub fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(&String, &serde_json::Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let pairs: Vec<String> = entries
                .into_iter()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), canonical_json(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", items.join(","))
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Compute SHA-256 hex digest of canonical JSON.
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
    fn test_canonical_json_sorting() {
        let json1 = serde_json::json!({"b": 2, "a": 1});
        let json2 = serde_json::json!({"a": 1, "b": 2});
        assert_eq!(canonical_json(&json1), canonical_json(&json2));
        assert_eq!(canonical_hash(&json1), canonical_hash(&json2));
    }
}
