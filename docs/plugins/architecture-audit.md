# Plugin Architecture Audit

This audit records the current plugin-system architecture after narrowing the official catalog to `official.privacy-filter`.

## Decision

Keep only `official.privacy-filter` as a bundled official plugin.

Remove the previous built-in prompt optimizer, safety detector, and generic redactor examples from the official catalog. Their behaviors remain valid extension scenarios, but they should be implemented as community plugins through `declarativeRules`, WASM, or a future isolated process runtime.

## Architecture Rationale

Mature plugin systems keep a small trusted host core and expose stable extension points instead of accumulating host-owned examples:

- VS Code uses manifest-declared [contribution points](https://code.visualstudio.com/api/references/contribution-points) and [activation events](https://code.visualstudio.com/api/references/activation-events).
- Chrome extensions require manifest-declared [permissions](https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions) and use constrained background [service workers](https://developer.chrome.com/docs/extensions/develop/migrate/to-service-workers).
- IDE plugin platforms expose explicit [extension points](https://plugins.jetbrains.com/docs/intellij/plugin-extension-points.html) and versioned compatibility contracts.

AIO Coding Hub follows the same shape:

- `plugin.json` declares ID, runtime, hooks, permissions, config schema, and host compatibility.
- Hooks are explicit gateway/log extension points with bounded timeouts and permission-trimmed contexts.
- Community code execution stays out of the Rust main process and WebView.
- `native` is reserved for built-in official engines. Third-party packages cannot declare host-native engines.

## Trust Boundaries

The host trust boundary is:

- Trusted: Rust host, gateway pipeline, database, packaged official native privacy engine.
- Semi-trusted: signed marketplace metadata and package checksums.
- Untrusted by default: local packages, marketplace packages, GitHub release packages, rule files, WASM bytecode, process runtime binaries.

The `official.*` namespace must remain host-owned. Local, marketplace, and GitHub packages must use publisher namespaces such as `acme.plugin-name`.

## Extension Model

Recommended runtime order:

1. `declarativeRules` for JSON path selection, regex detection, replacement, warning, blocking, and message append behavior.
2. WASM for deterministic code plugins that need logic beyond rule files.
3. Managed process runtime only for future cases that cannot fit WASM, with disabled-by-default marketplace enablement.

Do not open third-party `native` plugins without a separate signed binary policy, ABI stability story, crash isolation model, upgrade story, and platform-specific security review.

## Performance And Stability Guidance

Keep the hot path predictable:

- Execute hooks in priority order with fixed timeout budgets.
- Keep request and response bodies bounded before exposing them to plugins.
- Keep stream hooks chunk-based with sliding-window context instead of buffering full streams.
- Cache parsed rule/native engine state by plugin ID, version, and runtime key.
- Fail open for non-security enrichment; fail closed only for security/privacy gates that users explicitly enable.
- Record runtime failures and circuit-open skips so repeated bad plugins do not keep degrading the gateway.
- Keep official native engines few and focused so host startup, binary size, and maintenance risk stay controlled.

## v1.1 Performance Budgets

- Empty plugin pipeline request hook: no allocation-heavy runtime dispatch and below 25 microseconds on the maintainer laptop performance smoke.
- One noop declarative plugin request hook: below 250 microseconds on the maintainer laptop performance smoke.
- No `gateway.response.chunk` plugins: direct stream pass-through path must remain active.
- One declarative rule plugin: parsed rule runtime must be cached after first execution.
- Privacy Filter: compiled detector must be cached by plugin ID, version, installed directory, and runtime key.

## Current Shape

Bundled official plugin:

- `official.privacy-filter`: native host engine aligned with `packyme/privacy-filter`, used for irreversible pre-upstream privacy redaction and log redaction.

Open community capability:

- Declarative prompt helpers.
- Declarative response safety checks.
- Declarative or WASM log redactors.
- WASM examples and SDK contracts.
- Process runtime proof-of-concept documentation, disabled by default.

## Follow-Up Review Points

Before promoting plugin API v1 as stable:

- Confirm hook names and permission names are final enough for semantic versioning.
- Add marketplace policy for WASM enablement and package signing.
- Keep official examples in documentation as community patterns, not bundled host plugins.
- Add benchmarks around plugin hook overhead and Privacy Filter redaction latency on large but allowed payloads.
- Add telemetry-safe counters for plugin timeouts, skips, and quarantines without logging sensitive payloads.

## v1.1 Hardening Decisions

- Plugin API v1.1 uses `plugin-api-v1-contract.json` as the source of truth.
- Provider-neutral request context is available through `request.normalizedMessages`.
- WASM enablement remains rejected while host policy disables execution.
- Runtime caches are pruned on plugin refresh.
- Plugin hot-path performance smoke tests are part of release readiness.
- `create-aio-plugin replay` matches the supported declarative rule subset.
