//! Usage: One-shot localhost HTTP listener for OAuth authorization code callbacks.

use crate::shared::security::constant_time_eq;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as TokioTcpListener;

#[derive(Debug, Clone)]
pub(crate) struct OAuthCallbackPayload {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub(crate) struct BoundOAuthCallbackListener {
    listener: TokioTcpListener,
    pub port: u16,
}

pub(crate) async fn bind_callback_listener(
    preferred_port: u16,
) -> Result<BoundOAuthCallbackListener, String> {
    // Try preferred port first
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, preferred_port));
    if let Ok(std_listener) = TcpListener::bind(addr) {
        std_listener
            .set_nonblocking(true)
            .map_err(|e| format!("failed to set nonblocking: {e}"))?;
        let listener = TokioTcpListener::from_std(std_listener)
            .map_err(|e| format!("failed to create tokio listener: {e}"))?;
        return Ok(BoundOAuthCallbackListener {
            listener,
            port: preferred_port,
        });
    }

    // Fall back to dynamic port
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0u16));
    let std_listener =
        TcpListener::bind(addr).map_err(|e| format!("failed to bind callback listener: {e}"))?;
    let port = std_listener
        .local_addr()
        .map_err(|e| format!("failed to get local addr: {e}"))?
        .port();
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("failed to set nonblocking: {e}"))?;
    let listener = TokioTcpListener::from_std(std_listener)
        .map_err(|e| format!("failed to create tokio listener: {e}"))?;

    tracing::info!(
        preferred_port,
        actual_port = port,
        "callback listener: preferred port unavailable, using dynamic port"
    );

    Ok(BoundOAuthCallbackListener { listener, port })
}

impl BoundOAuthCallbackListener {
    pub(crate) async fn wait_for_callback(
        self,
        expected_state: &str,
        timeout_secs: u64,
    ) -> Result<OAuthCallbackPayload, String> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            self.accept_one(),
        )
        .await
        .map_err(|_| "OAuth callback timeout: no response received")?;

        let payload = result?;

        // Validate state parameter
        if let Some(ref state) = payload.state {
            if !constant_time_eq(state.as_bytes(), expected_state.as_bytes()) {
                return Err("OAuth state mismatch: possible CSRF attack".to_string());
            }
        }

        if payload.error.is_some() {
            let err = payload.error.as_deref().unwrap_or("unknown");
            let desc = payload.error_description.as_deref().unwrap_or("");
            return Err(format!("OAuth callback error: {err}: {desc}"));
        }

        if payload.code.is_none() {
            return Err("OAuth callback missing authorization code".to_string());
        }

        Ok(payload)
    }

    async fn accept_one(self) -> Result<OAuthCallbackPayload, String> {
        let (mut stream, _addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| format!("failed to accept callback connection: {e}"))?;

        let mut buf = vec![0u8; 4096];
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| format!("failed to read callback request: {e}"))?;

        let request = String::from_utf8_lossy(&buf[..n]);
        let payload = parse_callback_request(&request);

        // Send response
        let (status, body) = if payload.error.is_some() || payload.code.is_none() {
            ("400 Bad Request", "<html><body><h1>OAuth Error</h1><p>Authentication failed. You can close this tab.</p></body></html>")
        } else {
            ("200 OK", "<html><body><h1>Success</h1><p>Authentication successful! You can close this tab.</p></body></html>")
        };

        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );

        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;

        Ok(payload)
    }
}

fn parse_callback_request(request: &str) -> OAuthCallbackPayload {
    let mut payload = OAuthCallbackPayload {
        code: None,
        state: None,
        error: None,
        error_description: None,
    };

    // Parse GET request line: "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    if let Some(query) = path.split_once('?').map(|(_, q)| q) {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded = url_decode(value);
                match key {
                    "code" => payload.code = Some(decoded),
                    "state" => payload.state = Some(decoded),
                    "error" => payload.error = Some(decoded),
                    "error_description" => payload.error_description = Some(decoded),
                    _ => {}
                }
            }
        }
    }

    payload
}

fn url_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_val(bytes[i + 1]);
                let lo = hex_val(bytes[i + 2]);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
