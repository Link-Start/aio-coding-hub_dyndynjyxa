//! Usage: Gemini (Google) OAuth adapter.

use crate::gateway::oauth::provider_trait::*;
use axum::http::{header, HeaderMap, HeaderValue};

pub(crate) struct GeminiOAuthProvider {
    endpoints: OAuthEndpoints,
}

const GEMINI_DEFAULT_CLIENT_ID: &str = concat!(
    "681255809395",
    "-oo8ft2oprdrnp9e3aqf6av3hmdib135j",
    ".apps.googleusercontent.com"
);
const GEMINI_DEFAULT_CLIENT_SECRET: &str = concat!("GOCSPX-", "4uHgMPm-1o7Sk-geV6Cu5clXFsxl");

impl GeminiOAuthProvider {
    pub(crate) fn new() -> Self {
        let client_id = std::env::var("AIO_GEMINI_OAUTH_CLIENT_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| GEMINI_DEFAULT_CLIENT_ID.to_string());
        let client_secret = Some(
            std::env::var("AIO_GEMINI_OAUTH_CLIENT_SECRET")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| GEMINI_DEFAULT_CLIENT_SECRET.to_string()),
        );

        Self {
            endpoints: OAuthEndpoints {
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
                token_url: "https://oauth2.googleapis.com/token",
                client_id,
                client_secret,
                scopes: vec![
                    "https://www.googleapis.com/auth/cloud-platform",
                    "https://www.googleapis.com/auth/userinfo.email",
                    "https://www.googleapis.com/auth/userinfo.profile",
                ],
                redirect_host: "127.0.0.1",
                callback_path: "/oauth2callback",
                default_callback_port: 8085,
            },
        }
    }
}

impl OAuthProvider for GeminiOAuthProvider {
    fn cli_key(&self) -> &'static str {
        "gemini"
    }

    fn provider_type(&self) -> &'static str {
        "gemini_oauth"
    }

    fn endpoints(&self) -> &OAuthEndpoints {
        &self.endpoints
    }

    fn default_base_url(&self) -> &'static str {
        "https://generativelanguage.googleapis.com/v1beta"
    }

    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        vec![("access_type", "offline"), ("prompt", "consent")]
    }

    fn resolve_effective_token(
        &self,
        token_set: &OAuthTokenSet,
        _stored_id_token: Option<&str>,
    ) -> (String, Option<String>) {
        let token = &token_set.access_token;
        if !token.starts_with("ya29.") {
            tracing::warn!(
                "gemini oauth: access_token does not start with 'ya29.' prefix, may be invalid"
            );
        }
        (token.clone(), token_set.id_token.clone())
    }

    fn inject_upstream_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
    ) -> Result<(), String> {
        let bearer = format!("Bearer {access_token}");
        let bearer_val = HeaderValue::from_str(&bearer).map_err(|e| {
            format!("gemini oauth: invalid access_token for Authorization header: {e}")
        })?;
        headers.insert(header::AUTHORIZATION, bearer_val);
        if !headers.contains_key("x-goog-api-client") {
            headers.insert(
                "x-goog-api-client",
                HeaderValue::from_static("GeminiCLI/1.0"),
            );
        }
        Ok(())
    }
}
