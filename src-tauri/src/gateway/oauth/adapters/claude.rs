//! Usage: Claude (Anthropic) OAuth adapter.

use crate::gateway::oauth::provider_trait::*;
use axum::http::{header, HeaderMap, HeaderValue};
use std::future::Future;
use std::pin::Pin;

pub(crate) struct ClaudeOAuthProvider {
    endpoints: OAuthEndpoints,
}

impl ClaudeOAuthProvider {
    pub(crate) fn new() -> Self {
        Self {
            endpoints: OAuthEndpoints {
                auth_url: "https://claude.ai/oauth/authorize",
                token_url: "https://api.anthropic.com/v1/oauth/token",
                client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string(),
                client_secret: None,
                scopes: vec!["org:create_api_key", "user:profile", "user:inference"],
                redirect_host: "localhost",
                callback_path: "/callback",
                default_callback_port: 54545,
            },
        }
    }
}

impl OAuthProvider for ClaudeOAuthProvider {
    fn cli_key(&self) -> &'static str {
        "claude"
    }

    fn provider_type(&self) -> &'static str {
        "claude_oauth"
    }

    fn endpoints(&self) -> &OAuthEndpoints {
        &self.endpoints
    }

    fn default_base_url(&self) -> &'static str {
        "https://api.anthropic.com/v1"
    }

    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        vec![("code", "true")]
    }

    fn inject_upstream_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
    ) -> Result<(), String> {
        let bearer = format!("Bearer {access_token}");
        let bearer_val = HeaderValue::from_str(&bearer).map_err(|e| {
            format!("claude oauth: invalid access_token for Authorization header: {e}")
        })?;
        headers.insert(header::AUTHORIZATION, bearer_val);

        let key_val = HeaderValue::from_str(access_token)
            .map_err(|e| format!("claude oauth: invalid access_token for x-api-key header: {e}"))?;
        headers.insert("x-api-key", key_val);

        if !headers.contains_key("anthropic-version") {
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        }
        if !headers.contains_key("anthropic-beta") {
            headers.insert(
                "anthropic-beta",
                HeaderValue::from_static(
                    "claude-code-20250219,oauth-2025-04-20,interleaved-thinking-2025-05-14,context-management-2025-06-27,prompt-caching-scope-2026-01-05",
                ),
            );
        }
        Ok(())
    }

    fn fetch_limits(
        &self,
        client: &reqwest::Client,
        access_token: &str,
    ) -> Pin<Box<dyn Future<Output = Result<OAuthLimitsResult, String>> + Send + '_>> {
        let token = access_token.to_string();
        let client = client.clone();
        Box::pin(async move {
            let resp = client
                .get("https://api.anthropic.com/api/oauth/usage")
                .header("Authorization", format!("Bearer {}", token))
                .header("anthropic-beta", "oauth-2025-04-20")
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(|e| format!("claude limits fetch failed: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("claude limits fetch status: {}", resp.status()));
            }

            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("claude limits parse failed: {e}"))?;

            Ok(OAuthLimitsResult {
                raw_json: Some(json),
                ..Default::default()
            })
        })
    }
}
