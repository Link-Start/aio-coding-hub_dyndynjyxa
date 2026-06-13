# Gateway Raw/Decoded Body Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve compressed gateway request passthrough unless request bytes are actually changed, while still letting Privacy Filter and other plugins inspect and modify decoded JSON safely.

**Architecture:** Add a focused request-body model that tracks original wire bytes, decoded semantic bytes, original wire content coding, semantic hook headers, and mutation state separately. Middleware and plugins always operate on decoded semantic bytes plus semantic headers; immediately before `send_upstream`, the attempt path finalizes semantic headers/body back into a valid wire pair, using raw passthrough when unchanged and gzip re-encoding when changed.

**Tech Stack:** Rust, Axum `Bytes` and `HeaderMap`, reqwest upstream forwarding, `flate2` gzip encode/decode, existing gateway plugin pipeline, existing cargo test and clippy workflows.

---

## Scope And Non-Goals

This plan is limited to request body handling in the gateway proxy path.

In scope:
- `Content-Encoding: gzip` request bodies for Codex, Claude, and provider-transformed gateway requests.
- Keeping raw request bytes available until the final upstream send.
- Giving plugin request hooks decoded body bytes and semantic headers.
- Preserving original gzip bytes when no request-body mutation occurs.
- Re-encoding mutated decoded bodies as gzip when the original body was decoded from gzip successfully.
- Dropping or recalculating stale transfer metadata through reqwest instead of forwarding old `Content-Length`.
- Regression tests for both no-mutation gzip passthrough and Privacy Filter mutation.

Out of scope:
- Response body encoding behavior.
- Support for `br`, `deflate`, or stacked encodings such as `gzip, br`.
- Plugin API schema changes.
- UI changes.
- A general streaming request-body refactor.

## Existing Code Map

- `src-tauri/src/gateway/proxy/http_util.rs`  
  Owns gzip detection and current request/response gunzip helpers. The plan keeps low-level gzip helpers here and removes the request helper that mutates headers/body in place.

- `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`  
  Reads request body, currently decodes gzip by overwriting `ctx.body_bytes`, parses introspection JSON, and runs `gateway.request.afterBodyRead`. It will create the request-body model and pass semantic headers/body to the hook.

- `src-tauri/src/gateway/proxy/handler/middleware/mod.rs`  
  Owns `ProxyContext` and conversion to `RequestContextParts`. It will carry `request_body_state` from BodyReader into forwarding.

- `src-tauri/src/gateway/proxy/request_context.rs`  
  Current request single source of truth. It builds `base_headers`, removes hop headers and `Content-Length`, and stores decoded body fields. It will store `GatewayRequestBody` and keep `body_bytes` as the decoded compatibility view during migration.

- `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_iterator.rs`  
  Builds `PreparedProvider.upstream_body_bytes` from request body bytes and provider transforms. It will continue operating on decoded semantic bytes, and it will expose whether the body changed before an attempt.

- `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs`  
  Builds attempt headers, injects auth, sanitizes request body, runs `gateway.request.beforeSend`, fingerprints, and calls `send_upstream`. It will become the only place where decoded semantic body/headers are finalized into upstream wire body/headers.

- `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/send.rs`  
  Sends final `HeaderMap` and `Bytes`. It remains deliberately dumb: no gzip/header policy lives here.

- `src-tauri/src/gateway/proxy/handler/failover_loop/response/upstream_error.rs`  
  Some retry repair paths mutate `prepared.upstream_body_bytes`. The compatibility sweep must ensure those paths still produce mutation signals before the next attempt is finalized.

- `src-tauri/src/gateway/routes.rs`  
  Contains integration-style route tests and raw upstream capture helpers. It will be extended to capture upstream head and body bytes separately.

## Core Design

The implementation must keep these four concepts separate:

- **Wire body:** bytes read from Axum, exactly as the client sent them.
- **Decoded body:** JSON/text bytes used by introspection, provider transforms, and plugin hooks.
- **Wire headers:** headers that describe the final upstream body.
- **Semantic headers:** headers visible to hooks while they operate on decoded body bytes. If a gzip request was successfully decoded, semantic headers remove `Content-Encoding` and `Content-Length` because the hook body is no longer gzip bytes.

Create `src-tauri/src/gateway/proxy/request_body.rs`:

```rust
//! Usage: request body raw/decoded model for gateway passthrough.

use super::http_util::{gunzip_bytes_with_limit, gzip_bytes_with_limit, has_gzip_content_encoding};
use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestBodyEncoding {
    Identity,
    Gzip,
    Unsupported,
}

#[derive(Debug, Clone)]
pub(super) struct GatewayRequestBody {
    raw: Bytes,
    decoded: Bytes,
    encoding: RequestBodyEncoding,
    original_content_encoding: Option<HeaderValue>,
    decoded_from_raw: bool,
    mutated: bool,
}
```

Required behavior:
- `raw` never changes after construction.
- `decoded` changes only through `replace_decoded`.
- `original_content_encoding` stores the original wire `Content-Encoding` value.
- `decoded_from_raw` is `true` only when gzip decode succeeded.
- `mutated` becomes `true` only when decoded body bytes differ from the previous decoded body.
- `semantic_headers()` returns headers for decoded hook semantics, not upstream wire semantics.
- `finalize_for_upstream()` returns the final body bytes and mutates the provided headers into a valid wire header map.

Finalization rules:
- If `mutated == false`, return `raw.clone()`, remove `Content-Length`, and restore the original `Content-Encoding` if one existed.
- If `mutated == true` and the original body decoded from gzip successfully, gzip-encode `decoded`, restore original `Content-Encoding`, and remove `Content-Length`.
- If gzip re-encoding fails or exceeds the encoded limit, send decoded identity bytes, remove `Content-Encoding`, remove `Content-Length`, and log `failed to re-encode request gzip body; sending identity body`.
- If original encoding is unsupported and mutation happens, send decoded identity bytes, remove `Content-Encoding`, remove `Content-Length`, and log `request body mutated after unsupported content encoding; sending identity body`.
- Do not try to preserve the original `Content-Length`. reqwest can set valid transfer metadata from the final body.

---

### Task 1: Add Request Body Model And Unit Tests

**Files:**
- Create: `src-tauri/src/gateway/proxy/request_body.rs`
- Modify: `src-tauri/src/gateway/proxy/mod.rs`
- Modify: `src-tauri/src/gateway/proxy/http_util.rs`
- Test: `src-tauri/src/gateway/proxy/request_body.rs`

- [ ] **Step 1: Write failing request body model tests**

Create `src-tauri/src/gateway/proxy/request_body.rs` with the type skeleton and tests below:

```rust
//! Usage: request body raw/decoded model for gateway passthrough.

use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestBodyEncoding {
    Identity,
    Gzip,
    Unsupported,
}

#[derive(Debug, Clone)]
pub(super) struct GatewayRequestBody {
    raw: Bytes,
    decoded: Bytes,
    encoding: RequestBodyEncoding,
    original_content_encoding: Option<HeaderValue>,
    decoded_from_raw: bool,
    mutated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn gzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).expect("gzip write");
        encoder.finish().expect("gzip finish")
    }

    fn gunzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut decoder = flate2::read::GzDecoder::new(input);
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut out).expect("gzip read");
        out
    }

    fn gzip_headers(content_len: usize) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_len.to_string()).expect("len header"),
        );
        headers
    }

    #[test]
    fn unchanged_gzip_body_uses_semantic_headers_for_hooks_and_raw_bytes_for_upstream() {
        let plain = Bytes::from_static(br#"{"input":"hello 13344441520"}"#);
        let raw = Bytes::from(gzip_bytes(plain.as_ref()));
        let wire_headers = gzip_headers(raw.len());

        let body = GatewayRequestBody::from_wire(raw.clone(), &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(body.decoded(), &plain);
        assert_eq!(body.decoded_clone(), plain);
        assert!(!body.is_mutated());
        assert_eq!(upstream, raw);
        assert!(body.semantic_headers(&wire_headers).get(header::CONTENT_ENCODING).is_none());
        assert!(body.semantic_headers(&wire_headers).get(header::CONTENT_LENGTH).is_none());
        assert_eq!(hook_headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn mutated_gzip_body_is_reencoded_and_length_is_removed() {
        let plain = Bytes::from_static(br#"{"input":"hello 13344441520"}"#);
        let raw = Bytes::from(gzip_bytes(plain.as_ref()));
        let wire_headers = gzip_headers(raw.len());
        let mut body = GatewayRequestBody::from_wire(raw, &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);

        body.replace_decoded(Bytes::from_static(br#"{"input":"hello [电话]"}"#));
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert!(body.is_mutated());
        assert_eq!(hook_headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
        assert_eq!(gunzip_bytes(upstream.as_ref()), br#"{"input":"hello [电话]"}"#);
    }

    #[test]
    fn invalid_gzip_body_stays_raw_when_unchanged() {
        let raw = Bytes::from_static(b"not-gzip");
        let wire_headers = gzip_headers(raw.len());

        let body = GatewayRequestBody::from_wire(raw.clone(), &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(body.decoded(), &raw);
        assert_eq!(upstream, raw);
        assert_eq!(hook_headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn mutated_invalid_gzip_body_falls_back_to_identity() {
        let raw = Bytes::from_static(b"not-gzip");
        let wire_headers = gzip_headers(raw.len());
        let mut body = GatewayRequestBody::from_wire(raw, &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);

        body.replace_decoded(Bytes::from_static(br#"{"input":"changed"}"#));
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(upstream, Bytes::from_static(br#"{"input":"changed"}"#));
        assert!(hook_headers.get(header::CONTENT_ENCODING).is_none());
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn mutated_unsupported_encoding_drops_encoding_header() {
        let raw = Bytes::from_static(br#"{"input":"hello"}"#);
        let mut wire_headers = HeaderMap::new();
        wire_headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("br"));
        wire_headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("17"));
        let mut body = GatewayRequestBody::from_wire(raw, &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);

        body.replace_decoded(Bytes::from_static(br#"{"input":"changed"}"#));
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(upstream, Bytes::from_static(br#"{"input":"changed"}"#));
        assert!(hook_headers.get(header::CONTENT_ENCODING).is_none());
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }
}
```

- [ ] **Step 2: Expose the module**

Modify `src-tauri/src/gateway/proxy/mod.rs` and add this module declaration beside the other proxy modules:

```rust
mod request_body;
```

- [ ] **Step 3: Run the request body tests to verify RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml request_body --lib
```

Expected: compilation fails because `GatewayRequestBody::from_wire`, `decoded`, `decoded_clone`, `semantic_headers`, `replace_decoded`, `is_mutated`, and `finalize_for_upstream` are not implemented.

- [ ] **Step 4: Add reusable gzip helper functions**

Modify `src-tauri/src/gateway/proxy/http_util.rs`. Ensure the imports include both read and write traits:

```rust
use std::io::{Read, Write};
```

Add these helpers near the existing gzip helpers:

```rust
pub(super) fn gunzip_bytes_with_limit(
    input: &[u8],
    max_output_bytes: usize,
) -> Result<Bytes, String> {
    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = decoder
            .read(&mut buf)
            .map_err(|err| format!("failed to decode gzip body: {err}"))?;
        if n == 0 {
            break;
        }
        if out.len().saturating_add(n) > max_output_bytes {
            return Err(format!(
                "gzip decoded body exceeded limit: limit={max_output_bytes} bytes"
            ));
        }
        out.extend_from_slice(&buf[..n]);
    }
    Ok(Bytes::from(out))
}

pub(super) fn gzip_bytes_with_limit(
    input: &[u8],
    max_output_bytes: usize,
) -> Result<Bytes, String> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(input)
        .map_err(|err| format!("failed to encode gzip body: {err}"))?;
    let out = encoder
        .finish()
        .map_err(|err| format!("failed to finish gzip body: {err}"))?;
    if out.len() > max_output_bytes {
        return Err(format!(
            "gzip encoded body exceeded limit: limit={max_output_bytes} bytes"
        ));
    }
    Ok(Bytes::from(out))
}
```

- [ ] **Step 5: Implement `GatewayRequestBody`**

Add this implementation to `src-tauri/src/gateway/proxy/request_body.rs` below the struct:

```rust
use super::http_util::{gunzip_bytes_with_limit, gzip_bytes_with_limit, has_gzip_content_encoding};

impl GatewayRequestBody {
    pub(super) fn from_wire(
        raw: Bytes,
        headers: &HeaderMap,
        max_decoded_bytes: usize,
    ) -> Self {
        let encoding = classify_request_encoding(headers);
        let original_content_encoding = headers.get(header::CONTENT_ENCODING).cloned();
        match encoding {
            RequestBodyEncoding::Gzip => match gunzip_bytes_with_limit(raw.as_ref(), max_decoded_bytes) {
                Ok(decoded) => Self {
                    raw,
                    decoded,
                    encoding,
                    original_content_encoding,
                    decoded_from_raw: true,
                    mutated: false,
                },
                Err(err) => {
                    tracing::warn!(error = %err, "failed to decode request gzip body for inspection; preserving raw body");
                    Self {
                        decoded: raw.clone(),
                        raw,
                        encoding,
                        original_content_encoding,
                        decoded_from_raw: false,
                        mutated: false,
                    }
                }
            },
            RequestBodyEncoding::Identity | RequestBodyEncoding::Unsupported => Self {
                decoded: raw.clone(),
                raw,
                encoding,
                original_content_encoding,
                decoded_from_raw: false,
                mutated: false,
            },
        }
    }

    pub(super) fn decoded(&self) -> &Bytes {
        &self.decoded
    }

    pub(super) fn decoded_clone(&self) -> Bytes {
        self.decoded.clone()
    }

    pub(super) fn semantic_headers(&self, headers: &HeaderMap) -> HeaderMap {
        let mut semantic = headers.clone();
        semantic.remove(header::CONTENT_LENGTH);
        if self.decoded_from_raw {
            semantic.remove(header::CONTENT_ENCODING);
        }
        semantic
    }

    pub(super) fn replace_decoded(&mut self, next: Bytes) {
        if self.decoded != next {
            self.decoded = next;
            self.mutated = true;
        }
    }

    pub(super) fn is_mutated(&self) -> bool {
        self.mutated
    }

    pub(super) fn finalize_for_upstream(
        &self,
        headers: &mut HeaderMap,
        max_encoded_bytes: usize,
    ) -> Bytes {
        headers.remove(header::CONTENT_LENGTH);
        if !self.mutated {
            restore_original_content_encoding(headers, self.original_content_encoding.as_ref());
            return self.raw.clone();
        }

        match self.encoding {
            RequestBodyEncoding::Gzip if self.decoded_from_raw => {
                match gzip_bytes_with_limit(self.decoded.as_ref(), max_encoded_bytes) {
                    Ok(encoded) => {
                        restore_original_content_encoding(headers, self.original_content_encoding.as_ref());
                        encoded
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "failed to re-encode request gzip body; sending identity body");
                        headers.remove(header::CONTENT_ENCODING);
                        self.decoded.clone()
                    }
                }
            }
            RequestBodyEncoding::Gzip | RequestBodyEncoding::Unsupported => {
                tracing::warn!(
                    encoding = ?self.encoding,
                    "request body mutated after unsupported content encoding; sending identity body"
                );
                headers.remove(header::CONTENT_ENCODING);
                self.decoded.clone()
            }
            RequestBodyEncoding::Identity => {
                headers.remove(header::CONTENT_ENCODING);
                self.decoded.clone()
            }
        }
    }
}

fn classify_request_encoding(headers: &HeaderMap) -> RequestBodyEncoding {
    let Some(value) = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
    else {
        return RequestBodyEncoding::Identity;
    };
    let encodings = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if encodings.is_empty() || encodings.iter().all(|item| item.eq_ignore_ascii_case("identity")) {
        return RequestBodyEncoding::Identity;
    }
    if encodings.len() == 1 && has_gzip_content_encoding(headers) {
        return RequestBodyEncoding::Gzip;
    }
    RequestBodyEncoding::Unsupported
}

fn restore_original_content_encoding(headers: &mut HeaderMap, original: Option<&HeaderValue>) {
    match original {
        Some(value) => {
            headers.insert(header::CONTENT_ENCODING, value.clone());
        }
        None => {
            headers.remove(header::CONTENT_ENCODING);
        }
    }
}
```

- [ ] **Step 6: Run the request body tests to verify GREEN**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml request_body --lib
```

Expected: all `request_body` tests pass.

- [ ] **Step 7: Commit Task 1**

```bash
git add src-tauri/src/gateway/proxy/request_body.rs src-tauri/src/gateway/proxy/mod.rs src-tauri/src/gateway/proxy/http_util.rs
git commit -m "feat: add gateway request body model"
```

---

### Task 2: Teach BodyReader To Use Decoded Body And Semantic Headers

**Files:**
- Modify: `src-tauri/src/gateway/proxy/handler/middleware/mod.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`
- Modify: `src-tauri/src/gateway/proxy/request_context.rs`
- Test: `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`

- [ ] **Step 1: Add request body state to `ProxyContext`**

Modify `src-tauri/src/gateway/proxy/handler/middleware/mod.rs` imports:

```rust
use crate::gateway::proxy::request_body::GatewayRequestBody;
```

Add the field near `body_bytes`:

```rust
pub(super) request_body_state: Option<GatewayRequestBody>,
```

Modify `src-tauri/src/gateway/proxy/handler/mod.rs` where `ProxyContext` is constructed:

```rust
request_body_state: None,
```

- [ ] **Step 2: Add request body state to `RequestContext`**

Modify `src-tauri/src/gateway/proxy/request_context.rs` imports:

```rust
use super::request_body::GatewayRequestBody;
```

Add the field to `RequestContext` after `body_bytes`:

```rust
pub(super) request_body_state: GatewayRequestBody,
```

Add the field to `RequestContextParts` after `body_bytes`:

```rust
pub(super) request_body_state: GatewayRequestBody,
```

In `RequestContext::from_handler_parts`, destructure the field:

```rust
request_body_state,
```

In the returned `Self`, assign it:

```rust
request_body_state,
```

In `ProxyContext::into_request_context_parts`, pass it:

```rust
request_body_state: self
    .request_body_state
    .expect("request_body_state must be set by BodyReaderMiddleware"),
```

- [ ] **Step 3: Replace in-place request gunzip in BodyReader**

Modify imports in `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`. Remove these imports:

```rust
use crate::gateway::proxy::http_util::maybe_gunzip_request_body_bytes_with_limit;
use crate::gateway::util::{body_for_introspection, max_request_body_bytes};
```

Add these imports:

```rust
use crate::gateway::proxy::request_body::GatewayRequestBody;
use crate::gateway::util::max_request_body_bytes;
```

Replace the block that calls `maybe_gunzip_request_body_bytes_with_limit` and `body_for_introspection` with:

```rust
let mut request_body_state = GatewayRequestBody::from_wire(
    ctx.body_bytes.clone(),
    &ctx.headers,
    request_body_limit,
);
ctx.body_bytes = request_body_state.decoded_clone();
ctx.introspection_json =
    serde_json::from_slice::<serde_json::Value>(request_body_state.decoded().as_ref()).ok();
```

- [ ] **Step 4: Pass semantic headers/body to `afterBodyRead`**

In the `GatewayRequestHookInput` construction in `BodyReaderMiddleware::run`, replace:

```rust
headers: ctx.headers.clone(),
body: ctx.body_bytes.clone(),
```

with:

```rust
headers: request_body_state.semantic_headers(&ctx.headers),
body: request_body_state.decoded_clone(),
```

In the hook success branch, replace:

```rust
ctx.headers = output.headers;
ctx.body_bytes = output.body;
let introspection_body = body_for_introspection(&ctx.headers, &ctx.body_bytes);
ctx.introspection_json =
    serde_json::from_slice::<serde_json::Value>(introspection_body.as_ref()).ok();
```

with:

```rust
ctx.headers = output.headers;
request_body_state.replace_decoded(output.body);
ctx.body_bytes = request_body_state.decoded_clone();
ctx.introspection_json =
    serde_json::from_slice::<serde_json::Value>(request_body_state.decoded().as_ref()).ok();
```

Before `MiddlewareAction::Continue(Box::new(ctx))`, add:

```rust
ctx.request_body_state = Some(request_body_state);
```

- [ ] **Step 5: Run focused compilation check**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml body_too_large_message_includes_error --lib
```

Expected: compile succeeds and the existing BodyReader unit test passes.

- [ ] **Step 6: Commit Task 2**

```bash
git add src-tauri/src/gateway/proxy/handler/mod.rs src-tauri/src/gateway/proxy/handler/middleware/mod.rs src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs src-tauri/src/gateway/proxy/request_context.rs
git commit -m "refactor: keep gateway request body state"
```

---

### Task 3: Finalize Upstream Wire Body Only After Request Hooks

**Files:**
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_iterator.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_auth.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/request_sanitizer.rs`
- Test: `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs` only if a local unit test harness already exists; otherwise Task 5 route tests validate this behavior.

- [ ] **Step 1: Track provider-preparation mutation explicitly**

Modify `PreparedProvider` in `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_iterator.rs`:

```rust
pub(super) request_body_mutated_before_attempt: bool,
```

At the start of `prepare_provider`, initialize `upstream_body_bytes` from decoded semantic body instead of the raw/legacy body:

```rust
let mut upstream_body_bytes = input.request_body_state.decoded_clone();
let mut strip_request_content_encoding = input.strip_request_content_encoding_seed;
```

Immediately before constructing `PreparedProvider`, compute:

```rust
let request_body_mutated_before_attempt = input.request_body_state.is_mutated()
    || upstream_body_bytes != input.request_body_state.decoded_clone()
    || strip_request_content_encoding;
```

Add the field to `PreparedProvider { ... }`:

```rust
request_body_mutated_before_attempt,
```

- [ ] **Step 2: Stop auth from directly removing `Content-Encoding` for decoded body changes**

In `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_auth.rs`, replace the generic body-change cleanup:

```rust
if prepared.strip_request_content_encoding {
    headers.remove(header::CONTENT_ENCODING);
}
```

with no direct header mutation. Header/body consistency will be handled in `GatewayRequestBody::finalize_for_upstream` after all hooks complete. If the file needs `header` only for this removed block, remove that import.

- [ ] **Step 3: Change body sanitizer header handling into mutation-only handling**

In `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs`, modify `apply_body_sanitizer_outcome` by removing this line:

```rust
headers.remove(header::CONTENT_ENCODING);
```

Keep the special setting log push unchanged.

- [ ] **Step 4: Build semantic headers/body for `beforeSend` and finalize only once before upstream**

In `attempt_executor.rs`, replace the section from:

```rust
let mut upstream_body = clean_outcome.body;
let hook_input = GatewayRequestHookInput {
```

through the hook success assignment:

```rust
headers = output.headers;
upstream_body = output.body;
```

with this shape:

```rust
let mut body_state_for_attempt = input.request_body_state.clone();
let body_changed_before_hook = prepared.request_body_mutated_before_attempt
    || clean_outcome.changed()
    || clean_outcome.body != body_state_for_attempt.decoded_clone();
if body_changed_before_hook {
    body_state_for_attempt.replace_decoded(clean_outcome.body.clone());
}

let mut semantic_headers = body_state_for_attempt.semantic_headers(&headers);
let hook_input = GatewayRequestHookInput {
    hook_name: GatewayPluginHookName::RequestBeforeSend,
    trace_id: input.trace_id.clone(),
    cli_key: input.cli_key.clone(),
    method: input.req_method.clone(),
    path: input.forwarded_path.clone(),
    query: input.query.clone(),
    headers: semantic_headers.clone(),
    body: body_state_for_attempt.decoded_clone(),
    requested_model: input.requested_model.clone(),
};
match ctx.state.plugin_pipeline.run_request_hook(hook_input).await {
    Ok(output) => {
        crate::gateway::plugins::audit::persist_gateway_plugin_audit_events(
            &ctx.state.db,
            &input.trace_id,
            output.audit_events.clone(),
        );
        if let Some(blocked) = output.blocked {
            tracing::warn!(
                trace_id = %input.trace_id,
                provider_id = prepared.provider_id,
                status = blocked.status,
                reason = %blocked.reason,
                "plugin blocked gateway request before upstream send"
            );
            return AttemptSendOutcome::PluginBlocked(blocked.reason);
        }
        semantic_headers = output.headers;
        body_state_for_attempt.replace_decoded(output.body);
    }
    Err(mut err) => {
        crate::gateway::plugins::audit::persist_gateway_plugin_error_audit_events(
            &ctx.state.db,
            &input.trace_id,
            &mut err,
        );
        tracing::warn!(
            trace_id = %input.trace_id,
            provider_id = prepared.provider_id,
            "plugin beforeSend hook failed: {}",
            err
        );
        return AttemptSendOutcome::PluginBlocked(format!(
            "gateway plugin request hook failed: {err}"
        ));
    }
}

headers = semantic_headers;
let upstream_body = body_state_for_attempt.finalize_for_upstream(
    &mut headers,
    crate::gateway::util::max_request_body_bytes(),
);
```

The important invariant is: `RequestBeforeSend` always receives decoded body plus semantic headers; `send_upstream` always receives finalized wire body plus wire headers.

- [ ] **Step 5: Run focused compile test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml request_body --lib
```

Expected: compile succeeds and `request_body` tests pass.

- [ ] **Step 6: Commit Task 3**

```bash
git add src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_iterator.rs src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_auth.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/request_sanitizer.rs
git commit -m "fix: finalize gateway request body before upstream send"
```

---

### Task 4: Remove Obsolete In-Place Request Gunzip Path

**Files:**
- Modify: `src-tauri/src/gateway/proxy/http_util.rs`
- Test: `src-tauri/src/gateway/proxy/http_util.rs`
- Test: `src-tauri/src/gateway/proxy/request_body.rs`

- [ ] **Step 1: Confirm obsolete helper has no production callers**

Run:

```bash
rg -n "maybe_gunzip_request_body_bytes_with_limit|body_for_introspection\(" src-tauri/src/gateway
```

Expected:
- `maybe_gunzip_request_body_bytes_with_limit` has no production callers.
- `body_for_introspection` may remain in fingerprinting or fallback paths, but not in `BodyReaderMiddleware` for the primary request body decode path.

- [ ] **Step 2: Remove obsolete request helper**

In `src-tauri/src/gateway/proxy/http_util.rs`, remove the entire function:

```rust
pub(super) fn maybe_gunzip_request_body_bytes_with_limit(
    body: Bytes,
    headers: &mut HeaderMap,
    max_output_bytes: usize,
) -> Bytes {
    if !has_gzip_content_encoding(headers) {
        return body;
    }

    if body.is_empty() {
        headers.remove(header::CONTENT_ENCODING);
        headers.remove(header::CONTENT_LENGTH);
        return body;
    }

    let mut decoder = flate2::read::GzDecoder::new(body.as_ref());
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match decoder.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if out.len().saturating_add(n) > max_output_bytes {
                    tracing::warn!(
                        max_output_bytes,
                        "request gzip body exceeded decode limit; keeping encoded body"
                    );
                    return body;
                }
                out.extend_from_slice(&buf[..n]);
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to decode request gzip body; keeping encoded body");
                return body;
            }
        }
    }

    headers.remove(header::CONTENT_ENCODING);
    headers.remove(header::CONTENT_LENGTH);
    Bytes::from(out)
}
```

- [ ] **Step 3: Remove obsolete request helper tests**

In `http_util.rs`, remove these two tests:

```rust
#[test]
fn maybe_gunzip_request_decodes_within_limit_and_removes_encoding_headers() {
    let plain = Bytes::from_static(br#"{"input":"hello"}"#);
    let compressed = gzip_bytes(plain.as_ref());
    let mut headers = gzip_headers(compressed.len());

    let decoded =
        maybe_gunzip_request_body_bytes_with_limit(compressed, &mut headers, plain.len());

    assert_eq!(decoded, plain);
    assert!(headers.get(header::CONTENT_ENCODING).is_none());
    assert!(headers.get(header::CONTENT_LENGTH).is_none());
}

#[test]
fn maybe_gunzip_request_preserves_compressed_body_when_output_limit_exceeded() {
    let plain = Bytes::from(vec![b'a'; 128 * 1024]);
    let compressed = gzip_bytes(plain.as_ref());
    let mut headers = gzip_headers(compressed.len());

    let output =
        maybe_gunzip_request_body_bytes_with_limit(compressed.clone(), &mut headers, 1024);

    assert_eq!(output, compressed);
    assert_eq!(headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
    assert!(headers.get(header::CONTENT_LENGTH).is_some());
}
```

- [ ] **Step 4: Add tests for new reusable gzip helpers**

In `http_util.rs` tests, add:

```rust
#[test]
fn gzip_round_trip_helpers_preserve_body() {
    let plain = Bytes::from_static(br#"{"input":"hello"}"#);

    let encoded = super::gzip_bytes_with_limit(plain.as_ref(), 1024).expect("encode");
    let decoded = super::gunzip_bytes_with_limit(encoded.as_ref(), 1024).expect("decode");

    assert_eq!(decoded, plain);
}

#[test]
fn gzip_decode_helper_rejects_oversized_output() {
    let plain = Bytes::from(vec![b'a'; 128 * 1024]);
    let encoded = gzip_bytes(plain.as_ref());

    let err = super::gunzip_bytes_with_limit(encoded.as_ref(), 1024)
        .expect_err("should exceed output limit");

    assert!(err.contains("gzip decoded body exceeded limit"));
}

#[test]
fn gzip_encode_helper_rejects_oversized_output() {
    let plain = vec![b'a'; 128 * 1024];

    let err = super::gzip_bytes_with_limit(&plain, 4).expect_err("should exceed tiny limit");

    assert!(err.contains("gzip encoded body exceeded limit"));
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml "gzip_" --lib
cargo test --manifest-path src-tauri/Cargo.toml request_body --lib
```

Expected: all focused gzip and request-body tests pass.

- [ ] **Step 6: Commit Task 4**

```bash
git add src-tauri/src/gateway/proxy/http_util.rs src-tauri/src/gateway/proxy/request_body.rs
git commit -m "refactor: remove request gzip overwrite helper"
```

---

### Task 5: Add Route Regression Tests For Passthrough And Mutation

**Files:**
- Modify: `src-tauri/src/gateway/routes.rs`

- [ ] **Step 1: Add byte-preserving upstream raw request capture helpers**

Near existing route test helpers in `src-tauri/src/gateway/routes.rs`, add:

```rust
#[derive(Debug)]
struct CapturedRawRequest {
    head: String,
    body: Vec<u8>,
}

impl CapturedRawRequest {
    fn text(&self) -> String {
        let mut out = self.head.clone();
        out.push_str("\r\n\r\n");
        out.push_str(&String::from_utf8_lossy(&self.body));
        out
    }

    fn has_header_line(&self, needle: &str) -> bool {
        self.head.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
    }
}

fn find_http_head_split(bytes: &[u8]) -> Option<(usize, usize)> {
    let marker = b"\r\n\r\n";
    bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .map(|idx| (idx, idx + marker.len()))
}

async fn read_complete_http_request_bytes(socket: &mut tokio::net::TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let Ok(size) = socket.read(&mut chunk).await else {
            break;
        };
        if size == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..size]);
        if buf.len() > 64 * 1024 {
            break;
        }

        let Some((head_end, body_start)) = find_http_head_split(&buf) else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..head_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        if buf.len().saturating_sub(body_start) >= content_length {
            break;
        }
    }
    buf
}

async fn read_complete_http_request(socket: &mut tokio::net::TcpStream) -> String {
    String::from_utf8_lossy(&read_complete_http_request_bytes(socket).await).into_owned()
}

fn split_raw_http_request(raw: Vec<u8>) -> CapturedRawRequest {
    match find_http_head_split(&raw) {
        Some((head_end, body_start)) => CapturedRawRequest {
            head: String::from_utf8_lossy(&raw[..head_end]).into_owned(),
            body: raw[body_start..].to_vec(),
        },
        None => CapturedRawRequest {
            head: String::from_utf8_lossy(&raw).into_owned(),
            body: Vec::new(),
        },
    }
}
```

This keeps existing string-based callers working through `read_complete_http_request`, while raw capture preserves gzip body bytes exactly.

- [ ] **Step 2: Change `spawn_capturing_raw_upstream` receiver type**

Change its return type from:

```rust
tokio::sync::oneshot::Receiver<String>,
```

to:

```rust
tokio::sync::oneshot::Receiver<CapturedRawRequest>,
```

Inside the spawned task, replace the current string capture:

```rust
let request = read_complete_http_request(&mut socket).await;
let _ = tx.send(request);
```

with byte-preserving capture:

```rust
let request = read_complete_http_request_bytes(&mut socket).await;
let _ = tx.send(split_raw_http_request(request));
```

- [ ] **Step 3: Update existing assertions using captured raw request text**

For existing tests that call string methods on `captured`, replace:

```rust
captured.contains("...")
captured.to_ascii_lowercase().contains("...")
```

with:

```rust
captured.text().contains("...")
captured.has_header_line("...")
```

For body-only assertions on JSON text, prefer:

```rust
String::from_utf8_lossy(&captured.body).contains("...")
```

- [ ] **Step 4: Add gzip decode helper for route tests**

Near `gzip_bytes`, add:

```rust
fn gunzip_bytes(input: &[u8]) -> Vec<u8> {
    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut out).expect("gzip read");
    out
}
```

- [ ] **Step 5: Add no-plugin gzip passthrough test**

Add this test near `official_privacy_filter_redacts_gzipped_codex_responses_before_upstream`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn gateway_preserves_gzipped_codex_request_when_plugins_do_not_mutate_body() {
    let _env_lock = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().expect("home dir");
    let _env = isolate_app_env(home.path());
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();

    let mut app_settings = settings::AppSettings::default();
    app_settings.failover_max_attempts_per_provider = 1;
    app_settings.failover_max_providers_to_try = 1;
    settings::write(&app_handle, &app_settings).expect("write settings");
    crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
        .expect("enable codex cli proxy");

    let db_dir = tempfile::tempdir().expect("db dir");
    let db = db::init_for_tests(&db_dir.path().join("gzip-passthrough-test.sqlite"))
        .expect("init test db");
    let (upstream_base_url, captured_rx, upstream_task) =
        spawn_capturing_raw_upstream(r#"{"id":"stub-ok","object":"response","output":[]}"#)
            .await;
    let provider_id = insert_codex_provider(&db, upstream_base_url);
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
    let router = build_router(gateway_state(app_handle, db, log_tx));

    let plain_body = serde_json::json!({
        "model": "gpt-plugin",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "你知道 13344441520 是哪里的手机号嘛"
            }]
        }]
    })
    .to_string();
    let compressed_body = gzip_bytes(plain_body.as_bytes());
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/codex/_aio/provider/{provider_id}/v1/responses"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(Body::from(compressed_body.clone()))
        .expect("request");

    let response = router.oneshot(request).await.expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
        .await
        .expect("captured upstream request")
        .expect("captured request");

    assert!(captured.has_header_line("content-encoding: gzip"));
    assert_eq!(captured.body, compressed_body);
    assert!(!captured.text().contains("13344441520"));
    assert!(!captured.text().contains("[电话]"));

    upstream_task.abort();
}
```

- [ ] **Step 6: Update Privacy Filter gzip mutation test**

In `official_privacy_filter_redacts_gzipped_codex_responses_before_upstream`, replace the upstream assertions:

```rust
assert!(!captured
    .to_ascii_lowercase()
    .contains("content-encoding: gzip"));
assert!(captured.contains("[电话]"));
assert!(!captured.contains("13344441520"));
```

with:

```rust
assert!(captured.has_header_line("content-encoding: gzip"));
assert!(!captured.text().contains("13344441520"));
let decoded_body = gunzip_bytes(&captured.body);
let decoded_text = String::from_utf8(decoded_body).expect("decoded body utf8");
assert!(decoded_text.contains("[电话]"));
assert!(!decoded_text.contains("13344441520"));
```

Keep the request log assertion:

```rust
assert!(!request_log.attempts_json.contains("13344441520"));
```

- [ ] **Step 7: Run route regression tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml gateway_preserves_gzipped_codex_request_when_plugins_do_not_mutate_body --lib
cargo test --manifest-path src-tauri/Cargo.toml official_privacy_filter_redacts_gzipped_codex_responses_before_upstream --lib
```

Expected: both tests pass.

- [ ] **Step 8: Commit Task 5**

```bash
git add src-tauri/src/gateway/routes.rs
git commit -m "test: cover gzipped request passthrough and mutation"
```

---

### Task 6: Sweep Provider And Retry Mutation Paths

**Files:**
- Modify only when the search or tests prove a path still mutates decoded body bytes without mutation tracking:
  - `src-tauri/src/gateway/proxy/gemini_oauth.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/claude_model_mapping.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/claude_metadata_user_id_injection.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_session_id_completion.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/cx2cc_preparation.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/request_sanitizer.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/response/upstream_error.rs`
  - `src-tauri/src/gateway/proxy/handler/failover_loop/response/thinking_signature_rectifier_400.rs`

- [ ] **Step 1: Search every body mutation path**

Run:

```bash
rg -n "\*.*body_bytes\s*=|body_bytes\s*= Bytes::from|upstream_body_bytes\s*=|strip_request_content_encoding" src-tauri/src/gateway/proxy
```

For every hit, classify it into one of these buckets and write a short implementation note in the PR description:
- Prepared before attempt: covered by `request_body_mutated_before_attempt`.
- Sanitized during attempt: covered by `clean_outcome.changed()`.
- Hook mutation: covered by `replace_decoded(output.body)`.
- Retry repair after upstream response: covered because the next retry attempt compares `prepared.upstream_body_bytes` against the original decoded body.
- Response-only mutation: out of request body scope.

- [ ] **Step 2: Add exact test only if a discovered path is untracked**

If the sweep finds an untracked mutation path, add the smallest test in the owning module. Use this concrete pattern for `codex_session_id_completion.rs` if that path is the one missing coverage:

```rust
#[test]
fn codex_session_id_completion_sets_strip_flag_when_body_changes() {
    let mut upstream_body_bytes = Bytes::from(
        serde_json::to_vec(&serde_json::json!({
            "model": "gpt-plugin",
            "input": []
        }))
        .expect("serialize request"),
    );
    let mut strip_request_content_encoding = false;

    apply_if_needed(ApplyInput {
        cli_key: "codex",
        enable_codex_session_id_completion: true,
        upstream_body_bytes: &mut upstream_body_bytes,
        strip_request_content_encoding: &mut strip_request_content_encoding,
        cx2cc_codex_session_id: Some("session-123"),
    });

    assert!(strip_request_content_encoding);
}
```

If no untracked path exists, do not add speculative tests and do not commit an empty change.

- [ ] **Step 3: Run provider and plugin tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml plugin --lib
cargo test --manifest-path src-tauri/Cargo.toml claude_model_mapping --lib
cargo test --manifest-path src-tauri/Cargo.toml codex_chatgpt --lib
cargo test --manifest-path src-tauri/Cargo.toml codex_session_id_completion --lib
cargo test --manifest-path src-tauri/Cargo.toml gemini_oauth --lib
cargo test --manifest-path src-tauri/Cargo.toml request_sanitizer --lib
```

Expected: all selected tests pass. If a filter reports zero tests, note that filter result and rely on the final full backend test command in Task 7.

- [ ] **Step 4: Commit Task 6 only when files changed**

If files changed, stage the exact touched files from this list:

```bash
git add src-tauri/src/gateway/proxy/gemini_oauth.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/claude_model_mapping.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/claude_metadata_user_id_injection.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_session_id_completion.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/cx2cc_preparation.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/request_sanitizer.rs src-tauri/src/gateway/proxy/handler/failover_loop/response/upstream_error.rs src-tauri/src/gateway/proxy/handler/failover_loop/response/thinking_signature_rectifier_400.rs
git commit -m "fix: track request body mutations across provider transforms"
```

If no files changed, skip this commit.

---

### Task 7: Final Verification And PR Update

**Files:**
- No production files unless verification exposes a bug.

- [ ] **Step 1: Run Rust formatting**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
```

Expected: exit 0.

- [ ] **Step 2: Run focused backend tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml request_body --lib
cargo test --manifest-path src-tauri/Cargo.toml official_privacy_filter --lib
cargo test --manifest-path src-tauri/Cargo.toml gateway_preserves_gzipped_codex_request_when_plugins_do_not_mutate_body --lib
cargo test --manifest-path src-tauri/Cargo.toml plugin --lib
```

Expected:
- `request_body`: all tests pass.
- `official_privacy_filter`: all tests pass, including gzip mutation redaction.
- `gateway_preserves_gzipped_codex_request_when_plugins_do_not_mutate_body`: pass.
- `plugin --lib`: pass, except tests already marked ignored by the repo.

- [ ] **Step 3: Run Rust clippy**

Run:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

Expected: exit 0.

- [ ] **Step 4: Run frontend checks only if frontend or generated bindings changed**

If `src/`, `src/generated/bindings.ts`, or package scripts changed, run:

```bash
pnpm typecheck
pnpm lint
```

Expected: both exit 0.

- [ ] **Step 5: Run full pre-push check before pushing**

Run:

```bash
pnpm check:prepush
```

Expected:
- frontend unit shards pass;
- generated bindings check passes;
- `tauri:test` passes;
- `tauri:clippy` passes.

- [ ] **Step 6: Inspect final diff**

Run:

```bash
git status -sb
git diff --stat origin/codex/plugin-system-completion...HEAD
```

Expected:
- Worktree is clean after commits.
- Changed files are limited to the files named in this plan.

- [ ] **Step 7: Push PR branch**

Run:

```bash
git push origin codex/plugin-system-completion
```

Expected: PR #296 branch updates successfully.

## Acceptance Criteria

- Unchanged gzip request bodies are forwarded upstream with byte-for-byte identical compressed body bytes.
- Unchanged gzip request bodies keep `Content-Encoding: gzip` at upstream send time.
- Plugin request hooks receive decoded body bytes and semantic headers, so hooks do not see `Content-Encoding: gzip` when their body is already decoded.
- Privacy Filter redacts `13344441520` to `[电话]` for gzipped Codex request bodies.
- Mutated gzip request bodies are re-encoded as gzip when original gzip decode succeeded.
- Mutated gzip request bodies do not retain stale `Content-Length`.
- Invalid gzip request bodies pass through raw if unmodified.
- Invalid or unsupported encoded request bodies that are modified are sent as identity bytes with `Content-Encoding` removed and an English warning log.
- `send_upstream` remains a simple final send function and does not grow encoding policy.
- Focused tests, route regressions, clippy, and pre-push checks pass.

## Risk Notes

- The biggest compatibility risk is giving `beforeSend` raw gzip bytes. The plan avoids that by keeping `beforeSend` on decoded semantic bytes and finalizing only after the hook returns.
- `Content-Length` is intentionally not preserved, even for raw passthrough, because `RequestContext::build_base_headers` already strips it and reqwest can send valid transfer metadata.
- Provider transforms are JSON-semantic transforms, so they must continue to work on decoded bytes. Raw passthrough is a final-send optimization, not a prepare-phase input.
- Unsupported encodings are deliberately conservative. The gateway preserves raw bytes when untouched, but once a plugin or transform mutates decoded bytes, it cannot honestly claim the old unsupported encoding still applies.

## Self-Review

- Spec coverage: The plan covers raw passthrough, semantic hook inputs, gzip re-encoding after mutation, stale header cleanup, Privacy Filter gzip behavior, invalid gzip behavior, provider mutation sweep, and final verification.
- Placeholder scan: The plan contains no `TBD`, `TODO`, `implement later`, vague placeholder filenames, or empty test stubs.
- Type consistency: `GatewayRequestBody`, `RequestBodyEncoding`, `decoded_clone`, `semantic_headers`, `replace_decoded`, `is_mutated`, and `finalize_for_upstream` are consistently named across all tasks.
- Scope check: The plan is focused on gateway request body handling only. Response encoding, UI work, plugin schema changes, and non-gzip encoding support remain outside this plan.
