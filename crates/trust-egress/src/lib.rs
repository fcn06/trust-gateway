use regex::Regex;
use std::sync::LazyLock;

static REDACT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
        Regex::new(r"\bsk-[A-Za-z0-9_-]{20,}\b").unwrap(),
        Regex::new(r"\b(?:sk|pk|rk)_(?:live|test)_[A-Za-z0-9]{20,}\b").unwrap(),
    ]
});

pub struct EgressFilter;

impl EgressFilter {
    pub fn sanitize_text(text: &str) -> String {
        let mut result = text.to_string();
        for pattern in REDACT_PATTERNS.iter() {
            result = pattern.replace_all(&result, "[REDACTED]").to_string();
        }
        result
    }
}
