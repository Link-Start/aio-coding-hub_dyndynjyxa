# Plugin System v1.1 Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the plugin system after vNext stabilization so the public API is truthful, provider-neutral, fast on the gateway hot path, easy for community authors, and maintainable as a small open-source extension platform.

**Architecture:** Keep the host core small and explicit. v1.1 treats `docs/plugins/plugin-api-v1-contract.json` as the source of truth, derives SDK/docs/checks from that contract, exposes a provider-neutral hook context for Claude/Codex/OpenAI-style payloads, and keeps code execution policy-gated until the runtime is fully wired and performance-smoke tested. The gateway pipeline stays snapshot-based and timeout-bounded, with cache eviction and performance budgets added before any new runtime is promoted.

**Tech Stack:** Rust/Tauri 2/Axum/SQLite/Specta on the host; React/Vite/TypeScript/React Query/Vitest on the frontend; `@aio-coding-hub/plugin-sdk`, `create-aio-plugin`, and `aio-plugin-wasm-sdk` for plugin authors; `pnpm`, Cargo, and repository-local contract checkers for verification.

---

## Assumptions

- vNext stabilization in `docs/superpowers/plans/2026-06-12-plugin-system-vnext.md` remains the current release gate.
- This plan is for the next version after vNext stabilization. It must not silently change the current vNext scope.
- `official.privacy-filter` remains the only bundled official plugin.
- `declarativeRules` remains the default community runtime.
- WASM remains policy-gated unless a unit in this plan wires, tests, documents, and performance-smoke tests a real gateway execution path.
- Third-party `native` remains unavailable.
- The plan favors performance and stability over broad capability expansion.

## Reference Practices

This plan follows these mature plugin-system practices:

- VS Code separates manifest-declared contribution points, activation events, and extension-host execution: <https://code.visualstudio.com/api/references/contribution-points>, <https://code.visualstudio.com/api/references/activation-events>, <https://code.visualstudio.com/api/advanced-topics/extension-host>.
- Chrome extensions require manifest-declared permissions so users and the host can reason about capability boundaries: <https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions>.
- Kong Gateway exposes lifecycle phases, plugin priority, configuration schema, and chunk-oriented response filters as explicit extension contracts: <https://developer.konghq.com/custom-plugins/handler.lua/>.
- Figma plugins use a compact manifest with a declared runtime entry and explicit capability fields: <https://developers.figma.com/docs/plugins/manifest/>.
- Backstage favors extension points and composable plugin boundaries instead of letting plugins mutate arbitrary host internals: <https://backstage.io/docs/plugins/>.

## Current-State Evidence To Re-check Before Execution

- `docs/plugins/plugin-api-v1-contract.json` lists active hooks, reserved hooks, active permissions, reserved permissions, and runtime categories.
- `scripts/check-plugin-api-contract.mjs` currently checks contract token presence using text includes.
- `src-tauri/src/domain/plugins.rs` owns host manifest validation.
- `packages/plugin-sdk/src/index.ts` owns public TypeScript authoring types and validation helpers.
- `src-tauri/src/gateway/plugins/context.rs` owns hook context visibility and permission trimming.
- `src-tauri/src/gateway/plugins/pipeline.rs` owns hook ordering, timeout, failure policy, circuit state, and hot-path execution.
- `src-tauri/src/app/plugins/runtime_executor.rs` currently reports WASM as disabled or not wired.
- `src-tauri/src/app/plugins/rule_runtime.rs` owns declarative rules and the official Privacy Filter native engine cache.
- `src/pages/plugins/PluginConfigSchemaForm.tsx` owns config editing state.
- `packages/create-aio-plugin/src/devtools.ts` owns validate/replay/pack/sign/verify commands.

## v1.1 Success Criteria

- API contract drift is caught structurally, not only by substring checks.
- Active hooks have documented context fields, mutation fields, required permissions, timeout behavior, failure policy, and fixture examples.
- The SDK exposes a provider-neutral context shape that works for Claude, Codex/OpenAI Responses, and generic OpenAI-compatible request bodies.
- Privacy Filter behavior is proven against Claude, Codex, OpenAI Responses, plain text, and log persistence fixtures.
- WASM behavior is either fully wired behind explicit host policy or rejected before enablement with a documented error.
- Runtime caches prune entries for disabled, uninstalled, or upgraded plugins.
- Gateway plugin overhead has repeatable performance smoke tests and a performance budget.
- The config form resets draft state when plugin detail changes and does not submit stale config.
- `create-aio-plugin` has a host-compatible replay path that matches Rust declarative rule behavior for supported rules.
- The plugin docs and README guide a new community author from scaffold to validation to package to local install.

## Baseline Verification

- [x] Run `git status -sb`.
  Expected: record existing dirty files; do not revert unrelated work.
- [x] Run `pnpm check:plugin-api-contract`.
  Expected: current contract checker state is known before tightening it.
- [x] Run `pnpm check:plugin-system-docs`.
  Expected: current documentation checker state is known before adding assertions.
- [x] Run `pnpm plugin-sdk:typecheck`.
  Expected: SDK type contract passes before v1.1 changes, or failures are recorded as pre-existing.
- [x] Run `pnpm --filter @aio-coding-hub/plugin-sdk test`.
  Expected: SDK unit tests pass before v1.1 changes, or failures are recorded as pre-existing.
- [x] Run `pnpm create-aio-plugin:test`.
  Expected: scaffolder/devtool tests pass before v1.1 changes, or failures are recorded as pre-existing.
- [x] Run `cd src-tauri && cargo test plugin --lib`.
  Expected: plugin-focused host tests pass before v1.1 changes, or failures are recorded as pre-existing.

## File Responsibility Map

- `docs/plugins/plugin-api-v1-contract.json`: canonical hook, permission, runtime, timeout, and failure-policy contract.
- `scripts/check-plugin-api-contract.mjs`: structural contract checker for Rust, TypeScript SDK, docs, and scaffold.
- `src-tauri/src/domain/plugins.rs`: host manifest validation and compatibility checks.
- `packages/plugin-sdk/src/index.ts`: public authoring contract and validation helpers.
- `src-tauri/src/gateway/plugins/context.rs`: permission-trimmed hook context model.
- `docs/plugins/hooks.md`: public hook context/mutation/permission contract.
- `docs/plugins/permissions.md`: public permission capability matrix.
- `src-tauri/src/app/plugins/runtime_executor.rs`: runtime dispatch and WASM policy behavior.
- `src-tauri/src/app/plugins/wasm_runtime.rs`: low-level WASM executor.
- `src-tauri/src/app/plugins/rule_runtime.rs`: declarative rules execution, Privacy Filter execution, runtime caches.
- `src-tauri/src/gateway/plugins/pipeline.rs`: hook ordering, timeout, failure policy, circuit, audit, performance smoke tests.
- `src-tauri/src/gateway/plugins/pipeline.rs`: plugin hot-path performance smoke tests and budget assertions.
- `src/pages/plugins/PluginConfigSchemaForm.tsx`: config form draft lifecycle.
- `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`: config form behavior tests.
- `packages/create-aio-plugin/src/devtools.ts`: validate/replay/pack/sign/verify author tooling.
- `packages/create-aio-plugin/src/scaffold.test.ts`: author-tool regression tests.
- `docs/plugins/getting-started.md`, `docs/plugins/sdk.md`, `docs/plugins/declarative-rules.md`, `README.md`: developer golden path docs.

---

## Unit 1: Structural Plugin API Contract Guard

**Goal:** Replace fragile substring-only contract checks with structural checks that prove Rust validation, TypeScript SDK unions, docs, and scaffold defaults agree with `plugin-api-v1-contract.json`.

**Files:**
- Modify: `docs/plugins/plugin-api-v1-contract.json`
- Modify: `scripts/check-plugin-api-contract.mjs`
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `packages/plugin-sdk/src/index.test.ts`
- Modify: `src-tauri/src/domain/plugins.rs`
- Test: `scripts/check-plugin-api-contract.mjs`
- Test: `packages/plugin-sdk/src/index.test.ts`
- Test: tests inside `src-tauri/src/domain/plugins.rs`

- [x] **Step 1: Extend the contract JSON**

Add default timeout, default failure policy, active mutation fields, and supported config schema types:

```json
{
  "apiVersion": "1.0.0",
  "defaultHookTimeoutMs": 150,
  "defaultFailurePolicy": "fail-open",
  "activeHooks": [
    "gateway.request.afterBodyRead",
    "gateway.request.beforeSend",
    "gateway.response.chunk",
    "gateway.response.after",
    "gateway.error",
    "log.beforePersist"
  ],
  "reservedHooks": [
    "gateway.request.received",
    "gateway.request.beforeProviderResolution",
    "gateway.response.headers"
  ],
  "activeMutationFields": [
    "requestBody",
    "responseBody",
    "streamChunk",
    "logMessage",
    "headers"
  ],
  "configSchemaTypes": ["object", "array", "string", "password", "number", "integer", "boolean"],
  "activePermissions": [
    "request.meta.read",
    "request.header.read",
    "request.header.readSensitive",
    "request.header.write",
    "request.body.read",
    "request.body.write",
    "response.header.read",
    "response.header.write",
    "response.body.read",
    "response.body.write",
    "stream.inspect",
    "stream.modify",
    "log.redact"
  ],
  "reservedPermissions": [
    "plugin.storage",
    "network.fetch",
    "file.read",
    "file.write",
    "secret.read"
  ],
  "communityRuntimes": ["declarativeRules"],
  "policyGatedRuntimes": ["wasm"],
  "officialRuntimes": ["native:privacyFilter"]
}
```

- [x] **Step 2: Add RED tests for contract checker failures**

Create a temporary fixture inside `scripts/check-plugin-api-contract.mjs` by adding a test mode:

```javascript
const testRoot = process.env.AIO_PLUGIN_CONTRACT_TEST_ROOT;
const repoRoot = testRoot ?? dirname(scriptDir);
```

Add a Vitest-free Node test script `scripts/check-plugin-api-contract.selftest.mjs`:

```javascript
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";

const root = join(tmpdir(), `aio-plugin-contract-${Date.now()}`);
mkdirSync(join(root, "docs/plugins"), { recursive: true });
mkdirSync(join(root, "packages/plugin-sdk/src"), { recursive: true });
mkdirSync(join(root, "packages/create-aio-plugin/src"), { recursive: true });
mkdirSync(join(root, "src-tauri/src/domain"), { recursive: true });
writeFileSync(join(root, "docs/plugins/plugin-api-v1-contract.json"), JSON.stringify({
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read"],
  reservedPermissions: ["network.fetch"],
  communityRuntimes: ["declarativeRules"],
  policyGatedRuntimes: ["wasm"],
  officialRuntimes: ["native:privacyFilter"]
}, null, 2));
writeFileSync(join(root, "packages/plugin-sdk/src/index.ts"), "gateway.request.afterBodyRead request.body.read declarativeRules");
writeFileSync(join(root, "packages/create-aio-plugin/src/scaffold.ts"), "declarativeRules gateway.request.afterBodyRead request.body.read");
writeFileSync(join(root, "src-tauri/src/domain/plugins.rs"), "gateway.request.afterBodyRead request.body.read declarativeRules");
writeFileSync(join(root, "docs/plugin-manifest-v1.md"), "gateway.request.afterBodyRead request.body.read");
writeFileSync(join(root, "docs/plugins/hooks.md"), "gateway.request.afterBodyRead");
writeFileSync(join(root, "docs/plugins/permissions.md"), "request.body.read");
writeFileSync(join(root, "docs/plugins/manifest.md"), "declarativeRules wasm native privacyFilter");
writeFileSync(join(root, "docs/plugins/wasm-runtime.md"), "wasm PLUGIN_RUNTIME_DISABLED");
const result = spawnSync("node", ["scripts/check-plugin-api-contract.mjs"], {
  cwd: process.cwd(),
  env: { ...process.env, AIO_PLUGIN_CONTRACT_TEST_ROOT: root },
  encoding: "utf8"
});
if (result.status === 0 || !result.stderr.includes("gateway.response.headers")) {
  throw new Error(`expected structural contract failure, got status ${result.status}\n${result.stderr}`);
}
```

- [x] **Step 3: Run RED checker self-test**

Run:

```bash
node scripts/check-plugin-api-contract.selftest.mjs
```

Expected: exit 0 because the self-test observes the checker failing on intentionally incomplete fixtures.

- [x] **Step 4: Tighten checker semantics**

Update `scripts/check-plugin-api-contract.mjs` so it validates:

- every contract hook appears in SDK literal unions;
- every contract hook appears in host validation helpers;
- every reserved hook appears in a rejection test name or rejection error branch;
- every active mutation field appears in SDK `PluginHookResult`;
- `contextPatch` does not appear in SDK, WASM SDK examples, scaffold output, or docs except in a sentence saying it is unsupported;
- `defaultHookTimeoutMs` appears in `GatewayPluginPipelineConfig::default`;
- `defaultFailurePolicy` appears in `failure_policy`.

Use exact function helpers:

```javascript
function requireNotIncludes(path, text, values, label) {
  for (const value of values) {
    if (text.includes(value)) {
      failures.push(`${path} must not include ${label} ${value}`);
    }
  }
}
```

- [x] **Step 5: Add SDK parity tests**

In `packages/plugin-sdk/src/index.test.ts`, add:

```typescript
import contract from "../../../docs/plugins/plugin-api-v1-contract.json";

it("keeps permissionRisk defined for every v1 permission", () => {
  for (const permission of [...contract.activePermissions, ...contract.reservedPermissions]) {
    expect(permissionRisk(permission as never)).toMatch(/^(low|medium|high|critical)$/);
  }
});

it("rejects every reserved hook from the contract", () => {
  for (const hook of contract.reservedHooks) {
    const result = validateManifest({
      ...manifest,
      hooks: [{ name: hook as never }],
      permissions: ["request.meta.read"],
    });
    expect(result).toMatchObject({ ok: false, error: { code: "PLUGIN_RESERVED_HOOK" } });
  }
});
```

- [x] **Step 6: Run Unit 1 gate**

Run:

```bash
node scripts/check-plugin-api-contract.selftest.mjs
pnpm check:plugin-api-contract
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm plugin-sdk:typecheck
cd src-tauri && cargo test validate_manifest_ --lib
```

Expected: all commands exit 0.

---

## Unit 2: Hook Contract Matrix And Provider-Neutral Context

**Goal:** Make every active hook understandable and testable by plugin authors, with a normalized context that lets one plugin handle Claude, Codex/OpenAI Responses, and generic OpenAI-compatible requests.

**Files:**
- Modify: `docs/plugins/plugin-api-v1-contract.json`
- Modify: `src-tauri/src/gateway/plugins/context.rs`
- Modify: `src-tauri/src/gateway/plugins/permissions.rs`
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `docs/plugins/hooks.md`
- Modify: `docs/plugins/declarative-rules.md`
- Test: tests inside `src-tauri/src/gateway/plugins/context.rs`
- Test: `packages/plugin-sdk/src/index.test.ts`

- [x] **Step 1: Add hook matrix to the contract JSON**

Add a `hookMatrix` object:

```json
{
  "hookMatrix": {
    "gateway.request.afterBodyRead": {
      "phase": "after request body read and before provider send",
      "readPermissions": ["request.meta.read", "request.header.read", "request.body.read"],
      "writePermissions": ["request.header.write", "request.body.write"],
      "mutationFields": ["headers", "requestBody"],
      "contextFields": ["traceId", "request.headers", "request.body", "request.normalizedMessages"]
    },
    "gateway.response.chunk": {
      "phase": "for each bounded streaming response chunk",
      "readPermissions": ["stream.inspect"],
      "writePermissions": ["stream.modify"],
      "mutationFields": ["streamChunk"],
      "contextFields": ["traceId", "stream.sequence", "stream.chunk"]
    },
    "log.beforePersist": {
      "phase": "before gateway request log persistence",
      "readPermissions": ["log.redact"],
      "writePermissions": ["log.redact"],
      "mutationFields": ["logMessage"],
      "contextFields": ["traceId", "log.message"]
    }
  }
}
```

Keep matrix entries for all six active hooks, not only the three shown above.

- [x] **Step 2: Add RED context tests for normalized messages**

In `src-tauri/src/gateway/plugins/context.rs`, add:

```rust
#[test]
fn visible_request_context_extracts_codex_input_text_messages() {
    let input = GatewayRequestHookInput {
        hook_name: GatewayPluginHookName::RequestAfterBodyRead,
        trace_id: "trace-codex".to_string(),
        headers: HeaderMap::new(),
        body: Bytes::from(r#"{"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}]}"#),
    };

    let visible = input.visible_context(&["request.body.read".to_string()]);

    assert_eq!(
        visible.request.normalized_messages,
        vec![GatewayNormalizedMessage {
            role: "user".to_string(),
            text: "hello".to_string(),
            source: "openai.responses.input_text".to_string(),
        }]
    );
}

#[test]
fn visible_request_context_extracts_claude_content_messages() {
    let input = GatewayRequestHookInput {
        hook_name: GatewayPluginHookName::RequestAfterBodyRead,
        trace_id: "trace-claude".to_string(),
        headers: HeaderMap::new(),
        body: Bytes::from(r#"{"messages":[{"role":"user","content":[{"type":"text","text":"hello claude"}]}]}"#),
    };

    let visible = input.visible_context(&["request.body.read".to_string()]);

    assert_eq!(visible.request.normalized_messages[0].text, "hello claude");
}
```

- [x] **Step 3: Run RED context tests**

Run:

```bash
cd src-tauri && cargo test visible_request_context_extracts_ --lib
```

Expected: fail because `GatewayNormalizedMessage` and `normalized_messages` do not exist.

- [x] **Step 4: Implement normalized message extraction**

In `src-tauri/src/gateway/plugins/context.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayNormalizedMessage {
    pub(crate) role: String,
    pub(crate) text: String,
    pub(crate) source: String,
}
```

Add `normalized_messages: Vec<GatewayNormalizedMessage>` to `GatewayVisibleRequestContext`.

Populate it only when `request.body.read` is granted:

```rust
fn normalized_messages_from_body(body: &str) -> Vec<GatewayNormalizedMessage> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_messages(&root, &mut out);
    out
}
```

Implement `collect_messages` for:

- `messages[].content` as string;
- `messages[].content[].text`;
- `input[].content[].text`;
- `input[].content[].type == "input_text"` with `text`;
- fallback `input` string.

- [x] **Step 5: Mirror normalized context in SDK**

In `packages/plugin-sdk/src/index.ts`, add:

```typescript
export type GatewayNormalizedMessage = {
  role: string;
  text: string;
  source: string;
};

export type GatewayVisibleRequestContext = {
  body?: string;
  headers?: Record<string, string>;
  normalizedMessages?: GatewayNormalizedMessage[];
};
```

Update `PluginHookContext` so `context` can be typed as:

```typescript
export type PluginHookContext = {
  hook: GatewayHookName;
  traceId?: string;
  config: JsonValue;
  context: {
    request?: GatewayVisibleRequestContext;
    response?: GatewayVisibleResponseContext;
    stream?: GatewayVisibleStreamContext;
    log?: GatewayVisibleLogContext;
  };
};
```

- [x] **Step 6: Expand hook docs with matrix tables**

Update `docs/plugins/hooks.md` with one section per active hook:

```markdown
## gateway.request.afterBodyRead

- Phase: after request body read and before upstream provider send.
- Default timeout: 150 ms.
- Default failure policy: fail-open.
- Read permissions: `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read`.
- Write permissions: `request.header.write`, `request.body.write`.
- Mutation fields: `headers`, `requestBody`.
- Provider-neutral field: `request.normalizedMessages`.
```

Add fixture snippets for Claude and Codex/OpenAI Responses request bodies.

- [x] **Step 7: Run Unit 2 gate**

Run:

```bash
cd src-tauri && cargo test visible_request_context_extracts_ --lib
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm plugin-sdk:typecheck
pnpm check:plugin-api-contract
pnpm check:plugin-system-docs
pnpm check:generated-bindings
```

Expected: all commands exit 0. If `GatewayVisibleRequestContext` Specta output changes, run `pnpm tauri:gen-types` before `pnpm check:generated-bindings`.

---

## Unit 3: Privacy Filter Provider Matrix

**Goal:** Prove `official.privacy-filter` redacts sensitive content consistently across Claude, Codex/OpenAI Responses, plain OpenAI-compatible messages, raw text, and log persistence.

**Files:**
- Modify: `src-tauri/src/app/plugins/rule_runtime.rs`
- Modify: `src-tauri/src/app/plugins/privacy_filter.rs`
- Modify: `src-tauri/resources/plugins/official/privacy-filter/rules/gitleaks.toml`
- Modify: `docs/plugins/official-examples.md`
- Test: tests inside `src-tauri/src/app/plugins/rule_runtime.rs`
- Test: tests inside `src-tauri/src/app/plugins/privacy_filter.rs`

- [x] **Step 1: Add RED request matrix tests**

In `src-tauri/src/app/plugins/rule_runtime.rs`, add:

```rust
#[test]
fn official_privacy_filter_redacts_phone_numbers_in_provider_request_shapes() {
    let executor = RuleRuntimeGatewayPluginExecutor::default();
    let plugin = official_privacy_filter_detail_for_tests(serde_json::json!({
        "redactBeforeUpstream": true,
        "redactLogs": true
    }));
    for (name, body) in [
        ("claude", r#"{"messages":[{"role":"user","content":[{"type":"text","text":"phone 13344441520"}]}]}"#),
        ("openai_chat", r#"{"messages":[{"role":"user","content":"phone 13344441520"}]}"#),
        ("codex_responses", r#"{"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"phone 13344441520"}]}]}"#),
        ("raw_text", "phone 13344441520")
    ] {
        let context = request_context_for_tests(body);
        let result = executor
            .execute_official_privacy_filter_plugin(&plugin, context)
            .unwrap_or_else(|err| panic!("{name} privacy filter failed: {err}"));
        let output = result.request_body.expect("request body should be redacted");
        assert!(!output.contains("13344441520"), "{name} leaked phone number: {output}");
    }
}
```

- [x] **Step 2: Run RED request matrix**

Run:

```bash
cd src-tauri && cargo test official_privacy_filter_redacts_phone_numbers_in_provider_request_shapes --lib
```

Expected: fail if the bundled gitleaks rules do not detect the phone-number class or if any provider shape is skipped.

- [x] **Step 3: Add or align Privacy Filter rule coverage**

If the test fails on phone numbers, add a focused rule to `src-tauri/resources/plugins/official/privacy-filter/rules/gitleaks.toml`:

```toml
[[rules]]
id = "phone-number-cn-mainland"
description = "Mainland China mobile phone number"
regex = '''(?i)(?:^|[^0-9])((?:\+?86[-\s]?)?1[3-9]\d{9})(?:$|[^0-9])'''
secretGroup = 1
keywords = ["phone", "mobile", "手机号", "电话", "联系"]
```

Keep fixture/resource copies aligned if test fixtures remain.

- [x] **Step 4: Add log persistence test**

Add:

```rust
#[test]
fn official_privacy_filter_redacts_log_messages_after_request_redaction() {
    let executor = RuleRuntimeGatewayPluginExecutor::default();
    let plugin = official_privacy_filter_detail_for_tests(serde_json::json!({
        "redactBeforeUpstream": true,
        "redactLogs": true
    }));
    let context = log_context_for_tests("trace log 13344441520");

    let result = executor
        .execute_official_privacy_filter_plugin(&plugin, context)
        .expect("privacy filter log hook");

    let message = result.log_message.expect("log message should be redacted");
    assert!(!message.contains("13344441520"));
}
```

- [x] **Step 5: Document exact behavior**

Update `docs/plugins/official-examples.md`:

```markdown
`official.privacy-filter` redacts matching string values anywhere inside JSON request bodies. It also supports raw text bodies. For Codex/OpenAI Responses payloads, `input[].content[].text` and `input_text` content are covered because the engine walks every JSON string value before upstream send.
```

- [x] **Step 6: Run Unit 3 gate**

Run:

```bash
cd src-tauri && cargo test official_privacy_filter_ --lib
cd src-tauri && cargo test privacy_filter --lib
pnpm check:plugin-system-docs
```

Expected: all commands exit 0.

---

## Unit 4: WASM Runtime Policy Or Wiring Decision

**Goal:** Make WASM impossible to misunderstand: either it executes in the gateway behind an explicit policy and performance smoke tests, or it is rejected before enablement with clear SDK/docs/tooling messages.

**Files:**
- Modify: `src-tauri/src/app/plugins/runtime_executor.rs`
- Modify: `src-tauri/src/app/plugins/wasm_runtime.rs`
- Modify: `src-tauri/src/app/plugin_service.rs`
- Modify: `src-tauri/src/domain/plugins.rs`
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
- Modify: `docs/plugins/wasm-runtime.md`
- Test: `src-tauri/src/app/plugins/runtime_executor.rs`
- Test: `packages/create-aio-plugin/src/scaffold.test.ts`

- [x] **Step 1: Add enable-time RED test for policy-disabled WASM**

In `src-tauri/src/app/plugin_service.rs`, add:

```rust
#[test]
fn enable_plugin_rejects_wasm_when_host_policy_disables_execution() {
    let dir = tempfile::tempdir().expect("db dir");
    let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
    let manifest = wasm_manifest_for_tests("acme.wasm-policy");
    install_plugin_manifest(
        &db,
        manifest,
        PluginInstallSource::Local,
        Some(dir.path().to_string_lossy().to_string()),
        env!("CARGO_PKG_VERSION"),
    ).expect("install");
    grant_plugin_permissions(&db, "acme.wasm-policy", vec!["request.body.read".to_string()])
        .expect("grant");

    let err = enable_plugin(&db, "acme.wasm-policy", env!("CARGO_PKG_VERSION"))
        .expect_err("wasm should not enable without policy");

    assert_eq!(err.code(), "PLUGIN_RUNTIME_DISABLED");
}
```

- [x] **Step 2: Run RED enable test**

Run:

```bash
cd src-tauri && cargo test enable_plugin_rejects_wasm_when_host_policy_disables_execution --lib
```

Expected: fail if the host validates the manifest but allows a WASM plugin to become enabled while runtime execution is disabled.

- [x] **Step 3: Add runtime support check before enablement**

In `plugin_service.rs`, add:

```rust
fn ensure_runtime_enabled(manifest: &PluginManifest) -> AppResult<()> {
    match manifest.runtime {
        PluginRuntime::DeclarativeRules { .. } => Ok(()),
        PluginRuntime::Native { .. } if manifest.id == "official.privacy-filter" => Ok(()),
        PluginRuntime::Wasm { .. } => Err(AppError::new(
            "PLUGIN_RUNTIME_DISABLED",
            "wasm runtime execution is disabled by host policy",
        )),
        PluginRuntime::Native { .. } => Err(AppError::new(
            "PLUGIN_UNSUPPORTED_RUNTIME",
            "native runtime is reserved for official plugins",
        )),
    }
}
```

Call it inside `enable_plugin` after manifest validation and before status update.

- [x] **Step 4: Keep scaffold copy honest**

Update the WASM scaffold README text in `packages/create-aio-plugin/src/scaffold.ts`:

```text
This template packages a WASM artifact and validates the ABI, but gateway execution remains policy-gated. The host rejects enablement with PLUGIN_RUNTIME_DISABLED until WASM execution is enabled by host policy.
```

- [x] **Step 5: Add docs contract assertion**

Update `scripts/check-plugin-system-docs.mjs` so `docs/plugins/wasm-runtime.md` must include:

```text
PLUGIN_RUNTIME_DISABLED
WASM enablement is rejected while host policy disables execution
```

- [x] **Step 6: Run Unit 4 gate**

Run:

```bash
cd src-tauri && cargo test enable_plugin_rejects_wasm_when_host_policy_disables_execution --lib
cd src-tauri && cargo test runtime_executor_returns_clear_error_for_policy_disabled_wasm --lib
pnpm create-aio-plugin:test
pnpm check:plugin-system-docs
pnpm check:plugin-system-completion
```

Expected: all commands exit 0.

---

## Unit 5: Runtime Cache Eviction And Refresh Semantics

**Goal:** Prevent long-running gateway processes from retaining stale rule/native runtime caches after plugins are disabled, uninstalled, upgraded, or moved.

**Files:**
- Modify: `src-tauri/src/app/plugins/rule_runtime.rs`
- Modify: `src-tauri/src/app/plugins/runtime_executor.rs`
- Modify: `src-tauri/src/gateway/plugins/pipeline.rs`
- Modify: `src-tauri/src/gateway/runtime.rs`
- Test: tests inside `src-tauri/src/app/plugins/rule_runtime.rs`
- Test: tests inside `src-tauri/src/app/plugins/runtime_executor.rs`
- Test: tests inside `src-tauri/src/gateway/plugins/pipeline.rs`

- [x] **Step 1: Add RED cache retain test**

In `src-tauri/src/app/plugins/rule_runtime.rs`, add test-only cache sizing helpers:

```rust
#[cfg(test)]
impl RuleRuntimeGatewayPluginExecutor {
    fn cache_sizes_for_tests(&self) -> (usize, usize) {
        (
            self.cache.lock().unwrap().len(),
            self.privacy_filter_cache.lock().unwrap().len(),
        )
    }
}
```

Add:

```rust
#[test]
fn rule_runtime_prunes_cache_entries_not_in_active_plugin_keys() {
    let executor = RuleRuntimeGatewayPluginExecutor::default();
    let first = rule_plugin_detail_for_tests("acme.rules", "1.0.0");
    let second = rule_plugin_detail_for_tests("acme.other", "1.0.0");
    executor.execute_declarative_rules_plugin(&first, request_context_for_tests("{}")).unwrap();
    executor.execute_declarative_rules_plugin(&second, request_context_for_tests("{}")).unwrap();
    assert_eq!(executor.cache_sizes_for_tests().0, 2);

    executor.retain_runtime_caches_for_plugins(&[first.clone()]);

    assert_eq!(executor.cache_sizes_for_tests().0, 1);
}
```

- [x] **Step 2: Run RED cache test**

Run:

```bash
cd src-tauri && cargo test rule_runtime_prunes_cache_entries_not_in_active_plugin_keys --lib
```

Expected: fail because no retain API exists.

- [x] **Step 3: Implement retain API**

In `rule_runtime.rs`, add:

```rust
pub(crate) fn retain_runtime_caches_for_plugins(&self, plugins: &[PluginDetail]) {
    let rule_keys: std::collections::HashSet<String> = plugins
        .iter()
        .filter(|plugin| matches!(plugin.manifest.runtime, PluginRuntime::DeclarativeRules { .. }))
        .map(rule_runtime_cache_key)
        .collect();
    self.cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .retain(|key, _| rule_keys.contains(key));

    let privacy_keys: std::collections::HashSet<String> = plugins
        .iter()
        .filter(|plugin| plugin.summary.plugin_id == "official.privacy-filter")
        .map(privacy_filter_cache_key)
        .collect();
    self.privacy_filter_cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .retain(|key, _| privacy_keys.contains(key));
}
```

- [x] **Step 4: Expose executor refresh hook**

In `runtime_executor.rs`, add:

```rust
pub(crate) fn retain_runtime_caches_for_plugins(&self, plugins: &[PluginDetail]) {
    self.rule_runtime.retain_runtime_caches_for_plugins(plugins);
}
```

Change the pipeline executor type from opaque `Arc<dyn GatewayPluginExecutor>` to a small wrapper that can optionally receive refresh events:

```rust
pub(crate) trait GatewayPluginExecutor: Send + Sync {
    fn retain_runtime_caches_for_plugins(&self, _plugins: &[PluginDetail]) {}
    // existing hook methods
}
```

Call this from `GatewayPluginPipeline::replace_plugins` after snapshot replacement:

```rust
self.executor.retain_runtime_caches_for_plugins(&plugins);
```

- [x] **Step 5: Add pipeline refresh test**

In `src-tauri/src/gateway/plugins/pipeline.rs`, add:

```rust
#[test]
fn replace_plugins_notifies_executor_to_prune_runtime_caches() {
    let executor = Arc::new(PruneRecordingExecutor::default());
    let pipeline = GatewayPluginPipeline::for_tests_shared(vec![plugin("acme.a", 1, vec![])], executor.clone(), GatewayPluginPipelineConfig::default());

    pipeline.replace_plugins(vec![plugin("acme.b", 1, vec![])]);

    assert_eq!(executor.last_retain_ids(), vec!["acme.b"]);
}
```

- [x] **Step 6: Run Unit 5 gate**

Run:

```bash
cd src-tauri && cargo test rule_runtime_prunes_cache_entries_not_in_active_plugin_keys --lib
cd src-tauri && cargo test replace_plugins_notifies_executor_to_prune_runtime_caches --lib
cd src-tauri && cargo test runtime_executor_ --lib
```

Expected: all commands exit 0.

---

## Unit 6: Plugin Pipeline Performance Smoke And Budget

**Goal:** Add repeatable performance smoke tests for zero-plugin, one-rule-plugin, Privacy Filter, and response chunk paths so future plugin work cannot accidentally slow down the gateway hot path.

**Files:**
- Modify: `src-tauri/src/gateway/plugins/pipeline.rs`
- Modify: `src-tauri/src/app/plugins/rule_runtime.rs`
- Modify: `src-tauri/src/gateway/streams/plugin_chunk.rs`
- Modify: `docs/plugins/architecture-audit.md`
- Test: ignored tests inside `src-tauri/src/gateway/plugins/pipeline.rs`
- Test: ignored tests inside `src-tauri/src/app/plugins/rule_runtime.rs`

- [x] **Step 1: Add ignored empty-pipeline performance smoke test**

In `src-tauri/src/gateway/plugins/pipeline.rs`, inside the existing `#[cfg(test)] mod tests`, add:

```rust
#[tokio::test(flavor = "current_thread")]
#[ignore = "performance smoke: run manually before plugin API releases"]
async fn perf_empty_pipeline_request_hook_budget() {
    let pipeline = GatewayPluginPipeline::empty_shared();
    let iterations = 10_000_u32;
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let output = pipeline
            .run_request_hook(request_input())
            .await
            .expect("empty pipeline should pass");
        assert_eq!(output.body.as_ref(), b"hello");
    }
    let elapsed = start.elapsed();
    let avg_nanos = elapsed.as_nanos() / u128::from(iterations);
    eprintln!("plugin perf empty request hook avg_nanos={avg_nanos}");
    assert!(
        avg_nanos < 25_000,
        "empty plugin pipeline exceeded 25us budget: {avg_nanos}ns"
    );
}
```

- [x] **Step 2: Add ignored one-plugin performance smoke test**

Add:

```rust
#[tokio::test(flavor = "current_thread")]
#[ignore = "performance smoke: run manually before plugin API releases"]
async fn perf_one_noop_plugin_request_hook_budget() {
    let pipeline = GatewayPluginPipeline::for_tests_shared(
        vec![plugin("plugin.noop", 10, vec!["request.body.read"])],
        Arc::new(InMemoryGatewayPluginExecutor::new()),
        GatewayPluginPipelineConfig::default(),
    );
    let iterations = 5_000_u32;
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let output = pipeline
            .run_request_hook(request_input())
            .await
            .expect("one-plugin pipeline should pass");
        assert_eq!(output.body.as_ref(), b"hello");
    }
    let avg_nanos = start.elapsed().as_nanos() / u128::from(iterations);
    eprintln!("plugin perf one noop request hook avg_nanos={avg_nanos}");
    assert!(
        avg_nanos < 250_000,
        "one noop plugin exceeded 250us budget: {avg_nanos}ns"
    );
}
```

- [x] **Step 3: Add performance budget doc**

Update `docs/plugins/architecture-audit.md`:

```markdown
## v1.1 Performance Budgets

- Empty plugin pipeline request hook: no allocation-heavy runtime dispatch and below 25 microseconds on the maintainer laptop performance smoke.
- One noop declarative plugin request hook: below 250 microseconds on the maintainer laptop performance smoke.
- No `gateway.response.chunk` plugins: direct stream pass-through path must remain active.
- One declarative rule plugin: parsed rule runtime must be cached after first execution.
- Privacy Filter: compiled detector must be cached by plugin ID, version, installed directory, and runtime key.
```

- [x] **Step 4: Add performance smoke script**

In `package.json`, add:

```json
"plugin:perf-smoke": "cd src-tauri && cargo test perf_ --lib -- --ignored --nocapture"
```

- [x] **Step 5: Run Unit 6 gate**

Run:

```bash
pnpm plugin:perf-smoke
cd src-tauri && cargo test plugin_chunk --lib
cd src-tauri && cargo test gateway_plugin_pipeline_ --lib
```

Expected: ignored performance smoke tests compile and print average timings; focused tests exit 0.

---

## Unit 7: Config Schema Form State And Validation UX

**Goal:** Prevent stale config submissions when users switch selected plugins or when refreshed plugin detail arrives while the config form is mounted.

**Files:**
- Modify: `src/pages/plugins/PluginConfigSchemaForm.tsx`
- Modify: `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`
- Modify: `src/pages/PluginsPage.tsx`
- Test: `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`

- [x] **Step 1: Add RED rerender test**

In `PluginConfigSchemaForm.test.tsx`, add:

```tsx
it("resets draft when the plugin config identity changes", () => {
  const onSubmit = vi.fn();
  const schema = {
    type: "object",
    properties: {
      mode: { type: "string", enum: ["strict", "balanced"] },
    },
  };

  const { rerender } = render(
    <PluginConfigSchemaForm
      identity="official.privacy-filter:1"
      schema={schema}
      value={{ mode: "strict" }}
      pending={false}
      onSubmit={onSubmit}
    />
  );
  fireEvent.change(screen.getByLabelText("mode"), { target: { value: "balanced" } });

  rerender(
    <PluginConfigSchemaForm
      identity="acme.other:1"
      schema={schema}
      value={{ mode: "strict" }}
      pending={false}
      onSubmit={onSubmit}
    />
  );
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

  expect(onSubmit).toHaveBeenCalledWith({ mode: "strict" });
});
```

- [x] **Step 2: Run RED form test**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

Expected: fail because `identity` does not exist and local draft is not reset on prop identity changes.

- [x] **Step 3: Add identity-driven reset**

In `PluginConfigSchemaForm.tsx`, change props:

```tsx
export type PluginConfigSchemaFormProps = {
  identity: string;
  schema: JsonValue | null | undefined;
  value: JsonValue;
  pending: boolean;
  onSubmit: (value: JsonValue) => void;
};
```

Add:

```tsx
useEffect(() => {
  setDraft(initialObject(value));
}, [identity, value]);
```

Import `useEffect`.

- [x] **Step 4: Pass stable identity from page**

In `PluginsPage.tsx`, pass:

```tsx
identity={`${detail.summary.plugin_id}:${detail.manifest.configVersion ?? 1}:${detail.summary.updated_at}`}
```

to `PluginConfigSchemaForm`.

- [x] **Step 5: Run Unit 7 gate**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
pnpm test:unit src/pages/__tests__/PluginsPage.test.tsx
pnpm check:precommit:src
```

Expected: all commands exit 0.

---

## Unit 8: Developer Replay Parity With Host Rule Runtime

**Goal:** Make `create-aio-plugin replay` match the supported Rust declarative rule runtime for JSONPath, regex replace, block, warn, and appendMessage behavior.

**Files:**
- Modify: `packages/create-aio-plugin/src/devtools.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.test.ts`
- Modify: `docs/plugins/declarative-rules.md`
- Test: `packages/create-aio-plugin/src/scaffold.test.ts`
- Test: tests inside `src-tauri/src/app/plugins/rule_runtime.rs`

- [x] **Step 1: Add RED parity tests for JSONPath replace**

In `scaffold.test.ts`, add:

```typescript
it("replay applies same-target JSONPath replacement like the host rule runtime", () => {
  const files = createPluginScaffold({ id: "acme.redactor", name: "Redactor", template: "rule" });
  const result = replayHook(files, "gateway.request.afterBodyRead", {
    request: {
      body: JSON.stringify({
        messages: [{ role: "user", content: "SECRET_TOKEN" }],
      }),
    },
  });

  expect(result).toEqual({
    action: "replace",
    requestBody: JSON.stringify({
      messages: [{ role: "user", content: "[REDACTED]" }],
    }),
  });
});
```

- [x] **Step 2: Add RED parity tests for block and appendMessage**

Add:

```typescript
it("replay supports block and appendMessage actions", () => {
  const blockFiles = rulePluginFilesWithAction({ kind: "block", reason: "blocked" });
  expect(replayHook(blockFiles, "gateway.request.afterBodyRead", { request: { body: "danger" } }))
    .toEqual({ action: "block", reason: "blocked" });

  const appendFiles = rulePluginFilesWithAction({
    kind: "appendMessage",
    role: "developer",
    content: "Use safe mode",
  });
  const result = replayHook(appendFiles, "gateway.request.afterBodyRead", {
    request: { body: JSON.stringify({ messages: [{ role: "user", content: "hello" }] }) },
  });
  expect(JSON.stringify(result)).toContain("Use safe mode");
  expect(result).not.toHaveProperty("contextPatch");
});
```

- [x] **Step 3: Run RED replay tests**

Run:

```bash
pnpm create-aio-plugin:test
```

Expected: fail for unsupported replay behavior.

- [x] **Step 4: Implement host-compatible replay subset**

In `devtools.ts`, implement:

- `bodyFromFixture(context)` reading `request.body`, `body`, or raw string;
- `applyJsonPathReplace(body, jsonPath, regex, replacement)`;
- supported JSONPath forms: `$.messages[*].content`, `$.input[*].content[*].text`, `$.input`;
- action mapping to `requestBody`, `reason`, or `message`;
- `appendMessage` only for `system` and `developer` roles.

Keep unsupported rule shapes as `{ action: "pass" }` with no throw, matching current lightweight replay behavior.

- [x] **Step 5: Cross-check host rule runtime tests**

Run:

```bash
cd src-tauri && cargo test rule_runtime --lib
pnpm create-aio-plugin:test
```

Expected: all commands exit 0.

- [x] **Step 6: Update declarative rules docs**

Add a `Local replay compatibility` section to `docs/plugins/declarative-rules.md`:

```markdown
`create-aio-plugin replay` implements the host-supported v1.1 declarative rule subset for local fixtures. It is intentionally deterministic and does not execute WASM, process plugins, network calls, or host-only native engines.
```

- [x] **Step 7: Run Unit 8 gate**

Run:

```bash
pnpm create-aio-plugin:test
pnpm check:plugin-system-docs
```

Expected: all commands exit 0.

---

## Unit 9: Documentation Golden Path And Official Example Readiness

**Goal:** Make the documentation good enough for `packyme/privacy-filter` or another community plugin to become an official-style example without reading host internals.

**Files:**
- Modify: `README.md`
- Modify: `docs/plugins/README.md`
- Modify: `docs/plugins/getting-started.md`
- Modify: `docs/plugins/sdk.md`
- Modify: `docs/plugins/hooks.md`
- Modify: `docs/plugins/permissions.md`
- Modify: `docs/plugins/official-examples.md`
- Modify: `docs/plugins/publishing.md`
- Modify: `scripts/check-plugin-system-docs.mjs`
- Test: `scripts/check-plugin-system-docs.mjs`

- [x] **Step 1: Add docs checker assertions for golden path**

In `scripts/check-plugin-system-docs.mjs`, require:

```javascript
{
  path: "docs/plugins/getting-started.md",
  phrases: [
    "pnpm create-aio-plugin",
    "pnpm create-aio-plugin validate",
    "pnpm create-aio-plugin replay",
    "pnpm create-aio-plugin pack",
    "Install locally from the Plugins page",
    "Claude and Codex request shapes"
  ]
}
```

- [x] **Step 2: Run RED docs checker**

Run:

```bash
pnpm check:plugin-system-docs
```

Expected: fail if golden path phrases are missing.

- [x] **Step 3: Update getting started guide**

Add a complete flow:

```markdown
## Golden Path

1. Scaffold a declarative rule plugin.
2. Validate `plugin.json`.
3. Replay a Claude fixture and a Codex/OpenAI Responses fixture.
4. Pack `.aio-plugin`.
5. Install locally from the Plugins page.
6. Grant requested permissions.
7. Enable the plugin.
8. Inspect audit logs.
```

Include command blocks for each step.

- [x] **Step 4: Update official examples guide**

Add:

```markdown
An official-style example must include:

- a minimal manifest;
- a fixture for Claude messages;
- a fixture for Codex/OpenAI Responses input;
- a local replay command;
- a package command;
- the exact permissions it requests;
- a short explanation of what is intentionally unsupported.
```

- [x] **Step 5: Update README plugin section**

Add a short README link block:

```markdown
Plugin authors should start with [Plugin Development](docs/plugins/README.md). The stable community runtime is `declarativeRules`; WASM packaging is available for ABI experimentation but gateway execution is policy-gated.
```

- [x] **Step 6: Run Unit 9 gate**

Run:

```bash
pnpm check:plugin-system-docs
pnpm check:plugin-system-completion
pnpm check:spec-links
```

Expected: all commands exit 0.

---

## Unit 10: Final Verification And Release Readiness

**Goal:** Prove the v1.1 hardening work is complete with requirement-by-requirement evidence before any PR or release claim.

**Files:**
- Modify: `docs/plugins/architecture-audit.md`
- Modify: `docs/superpowers/plans/2026-06-13-plugin-system-v1-1-hardening.md`
- Test: full focused gates listed below

- [x] **Step 1: Update architecture audit**

Append:

```markdown
## v1.1 Hardening Decisions

- Plugin API v1.1 uses `plugin-api-v1-contract.json` as the source of truth.
- Provider-neutral request context is available through `request.normalizedMessages`.
- WASM enablement remains rejected while host policy disables execution.
- Runtime caches are pruned on plugin refresh.
- Plugin hot-path performance smoke tests are part of release readiness.
- `create-aio-plugin replay` matches the supported declarative rule subset.
```

- [x] **Step 2: Fill this plan's completion checklist**

At the end of this file, mark each checklist item only after fresh verification output proves it.

- [x] **Step 3: Run focused backend gates**

Run:

```bash
cd src-tauri && cargo test validate_manifest_ --lib
cd src-tauri && cargo test gateway_plugin_pipeline_ --lib
cd src-tauri && cargo test plugin_chunk --lib
cd src-tauri && cargo test rule_runtime --lib
cd src-tauri && cargo test official_privacy_filter_ --lib
cd src-tauri && cargo test runtime_executor_ --lib
cd src-tauri && cargo fmt -- --check
```

Expected: all commands exit 0.

- [x] **Step 4: Run focused frontend and tooling gates**

Run:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm plugin-sdk:typecheck
pnpm create-aio-plugin:test
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
pnpm check:plugin-api-contract
pnpm check:plugin-system-docs
pnpm check:plugin-system-completion
pnpm check:generated-bindings
```

Expected: all commands exit 0.

- [x] **Step 5: Run build-level gates**

Run:

```bash
pnpm check:precommit:src
pnpm tauri:check
```

Expected: both commands exit 0.

- [x] **Step 6: Run diff hygiene**

Run:

```bash
git status --short
git diff -- docs/superpowers/plans/2026-06-13-plugin-system-v1-1-hardening.md
```

Expected: changes are limited to v1.1 hardening scope; unrelated dirty files remain untouched.

---

## Completion Checklist

- [x] Contract checker structurally verifies Rust, SDK, docs, and scaffold against `plugin-api-v1-contract.json`.
- [x] Hook docs include context fields, mutation fields, permissions, timeout, failure policy, and examples.
- [x] Provider-neutral `request.normalizedMessages` covers Claude and Codex/OpenAI Responses fixtures.
- [x] Privacy Filter matrix tests cover Claude, Codex/OpenAI Responses, OpenAI-compatible chat, raw text, and logs.
- [x] WASM enablement cannot be misunderstood: it is wired and tested, or rejected before enablement with `PLUGIN_RUNTIME_DISABLED`.
- [x] Runtime caches are pruned during plugin refresh.
- [x] Plugin pipeline performance smoke tests compile and have documented budgets.
- [x] Config form resets stale draft state when plugin identity changes.
- [x] `create-aio-plugin replay` matches the supported host declarative rule subset.
- [x] README and plugin docs provide a complete community author golden path.
- [x] All Unit 10 verification gates have fresh successful output.

## Execution Notes

- Execute one Unit at a time.
- Do not start the next Unit until the current Unit gate has fresh passing output.
- Prefer RED tests before implementation changes.
- Use English tracing/log messages for new host runtime events.
- Do not remove unrelated dirty work.
- If a gate fails, switch to `superpowers:systematic-debugging` before changing production code.
