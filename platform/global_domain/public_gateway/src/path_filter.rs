use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

pub async fn security_filter(request: Request, next: Next) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    if is_malicious_path(path) {
        tracing::warn!("🛡️ Security filter blocked suspicious request: {}", path);
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

fn is_malicious_path(path: &str) -> bool {
    if path.contains("..") {
        return true;
    }

    // Block hidden files/directories except .well-known
    let segments: Vec<&str> = path.split('/').collect();
    for segment in segments {
        if segment.starts_with('.') && !segment.is_empty()
            && segment != ".well-known" {
                return true;
            }
    }

    let path_lower = path.to_lowercase();
    let blacklisted_extensions = [
        ".php", ".aspx", ".asp", ".jsp", ".cgi", ".env", ".yaml", ".yml", ".ini", ".conf", ".sql",
        ".bak",
    ];
    for ext in &blacklisted_extensions {
        if path_lower.ends_with(ext) {
            return true;
        }
    }

    let blacklisted_substrings = [
        "wp-admin",
        "wp-login",
        "wp-content",
        "etc/passwd",
        "cgi-bin",
        "autodiscover",
        "xmlrpc",
    ];
    for sub in &blacklisted_substrings {
        if path_lower.contains(sub) {
            return true;
        }
    }

    false
}
