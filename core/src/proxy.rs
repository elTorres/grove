//! Shared HTTP proxy resolution helpers for registry fetches and explorer clients.

fn env_proxy(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| std::env::var(name.to_ascii_lowercase()).ok())
        .filter(|value| !value.trim().is_empty())
}

fn host_from_url(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split('/').next().unwrap_or(rest);
    let host = host.split('@').next().unwrap_or(host);
    let host = host.trim().trim_matches('[').trim_matches(']');
    let host = host.split(':').next().unwrap_or(host);
    Some(host.to_ascii_lowercase())
}

/// Resolve a proxy URL for a request target, honoring common proxy env vars and
/// `NO_PROXY`/`no_proxy` exclusions.
pub fn proxy_from_env(url: &str) -> Option<String> {
    let host = host_from_url(url)?;
    let no_proxy = std::env::var("NO_PROXY")
        .ok()
        .or_else(|| std::env::var("no_proxy").ok())
        .unwrap_or_default();

    for entry in no_proxy.split(',') {
        let entry = entry.trim().to_ascii_lowercase();
        if entry.is_empty() || entry == "*" {
            continue;
        }
        if entry == host || host.ends_with(&format!(".{entry}")) {
            return None;
        }
    }

    if url.starts_with("https://") {
        env_proxy("HTTPS_PROXY")
            .or_else(|| env_proxy("ALL_PROXY"))
            .or_else(|| env_proxy("HTTP_PROXY"))
    } else if url.starts_with("http://") {
        env_proxy("HTTP_PROXY")
            .or_else(|| env_proxy("ALL_PROXY"))
            .or_else(|| env_proxy("HTTPS_PROXY"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::proxy_from_env;

    #[test]
    fn respects_no_proxy_for_matching_host() {
        std::env::set_var("NO_PROXY", "example.test");
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("ALL_PROXY");

        assert!(proxy_from_env("https://example.test/data").is_none());

        std::env::remove_var("NO_PROXY");
    }
}
