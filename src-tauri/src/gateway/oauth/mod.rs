//! Usage: OAuth adapter pattern for multi-CLI OAuth login support.

pub(crate) mod adapters;
pub(crate) mod callback_server;
pub(crate) mod pkce;
pub(crate) mod provider_trait;
pub(crate) mod refresh;
pub(crate) mod refresh_loop;
pub(crate) mod registry;
pub(crate) mod token_exchange;

use std::sync::Mutex;
use tokio::sync::watch;

/// Global abort handle for in-progress OAuth flows.
/// When a new flow starts, it cancels any prior pending flow so the old callback
/// listener is dropped immediately (frees the port).
static ACTIVE_FLOW_ABORT: Mutex<Option<watch::Sender<()>>> = Mutex::new(None);

/// Cancel any in-progress OAuth flow and return a receiver that the new flow
/// should select on so it can itself be cancelled by a future invocation.
pub(crate) fn cancel_previous_flow() -> watch::Receiver<()> {
    let mut guard = ACTIVE_FLOW_ABORT.lock().unwrap_or_else(|e| e.into_inner());
    // Dropping the old sender causes the old receiver to see a channel-closed signal,
    // which aborts the old `wait_for_callback` via the tokio::select! in the caller.
    let (tx, rx) = watch::channel(());
    *guard = Some(tx);
    rx
}

/// Build an HTTP client suitable for OAuth token exchange and refresh requests.
///
/// Respects standard proxy environment variables (`HTTPS_PROXY`, `HTTP_PROXY`,
/// `ALL_PROXY`) automatically via reqwest defaults.  Additionally, if the user
/// has set `AIO_OAUTH_PROXY_URL`, that URL will be configured as an explicit
/// "all traffic" proxy, which is useful in corporate environments where system
/// proxy detection is insufficient.
pub(crate) fn build_oauth_http_client(
    user_agent: &str,
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs));

    // Explicit proxy override from dedicated env var.
    if let Ok(proxy_url) = std::env::var("AIO_OAUTH_PROXY_URL") {
        let trimmed = proxy_url.trim();
        if !trimmed.is_empty() {
            tracing::info!(
                proxy_url = %trimmed,
                "oauth: using explicit proxy from AIO_OAUTH_PROXY_URL"
            );
            let proxy = reqwest::Proxy::all(trimmed)
                .map_err(|e| format!("invalid AIO_OAUTH_PROXY_URL={trimmed}: {e}"))?;
            builder = builder.proxy(proxy);
        }
    } else {
        // Log which standard proxy env vars are active for diagnostics.
        for var in [
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "ALL_PROXY",
            "https_proxy",
            "http_proxy",
            "all_proxy",
        ] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    tracing::debug!(env_var = var, value = %val, "oauth: detected proxy env var");
                }
            }
        }
    }

    builder
        .build()
        .map_err(|e| format!("oauth HTTP client init failed: {e}"))
}
