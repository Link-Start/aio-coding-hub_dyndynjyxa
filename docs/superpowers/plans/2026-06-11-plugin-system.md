# Plugin System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task in this session. Subagents are not used unless the user explicitly authorizes delegation. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete `docs/plugin-system-development-plan.md` end to end as a stable, modular, tested community plugin system for aio coding hub.

**Architecture:** Build the system in gated units. M0 freezes public contracts in docs, M1 adds storage/service/IPC/GUI management for no-code plugins, M2 integrates a permission-trimmed gateway hook pipeline, M3 validates the pipeline with official declarative plugins, M4 adds safe package distribution and rollback, M5 adds sandboxed code runtime foundations, and M6 adds SDK/tooling/docs. Every unit must finish with focused tests before the next unit begins.

**Tech Stack:** Rust/Tauri 2/Axum/SQLite/Specta on the backend; React/Vite/TypeScript/React Query/Vitest on the frontend; `pnpm` and Cargo verification commands.

---

## Assumptions And Constraints

- The source of truth is `docs/plugin-system-development-plan.md`.
- The user explicitly forbids clarification questions, so ambiguous details are resolved conservatively from existing code and the plan.
- Existing dirty worktree state is preserved; unrelated changes are not reverted.
- Performance and stability win over broad surface area. Each hook and runtime path has a timeout, audit trail, and failure policy.
- Short-term third-party JavaScript/TypeScript execution remains out of scope.
- Any code plugin runtime must be isolated from the Rust main process and Tauri WebView.
- Every production-code unit follows Red/Green/Refactor: write the failing test, verify it fails, implement minimal code, verify it passes, then refactor if needed.

## Baseline Verification

- [ ] Run `pnpm install` only if dependencies are missing.
- [ ] Run `pnpm test:unit -- --runInBand` only if Vitest supports the flag; otherwise use `pnpm test:unit`.
- [ ] Run `pnpm tauri:test`.
- [ ] Run `pnpm tauri:check`.
- [ ] Record any pre-existing failures before changing production code.

## Unit M0: Architecture Freeze And RFC

**Files:**
- Create: `docs/plugin-system-rfc.md`
- Create: `docs/plugin-manifest-v1.md`
- Create: `scripts/check-plugin-system-docs.mjs`
- Modify: `package.json`

**Tests And Verification:**
- Red: `pnpm check:plugin-system-docs` must fail before M0 docs exist.
- Green: `pnpm check:plugin-system-docs` must pass after docs are written.
- Also run `pnpm check:spec-links`.

**Steps:**
- [ ] Add a documentation contract checker that asserts M0 required docs and key contract phrases exist.
- [ ] Verify the checker fails with missing M0 docs.
- [ ] Write `plugin-system-rfc.md` covering goals, non-goals, current hookable architecture, runtime route, safety, cross-platform rules, and Skill-market boundary.
- [ ] Write `plugin-manifest-v1.md` covering manifest schema, ID/SemVer rules, config schema subset, host compatibility, status machine, hook v1, permission v1, and the three sample manifests.
- [ ] Verify the checker passes.
- [ ] Run link/spec validation.

## Unit M1: Plugin Infrastructure

**Backend Files:**
- Modify: `src-tauri/src/infra/app_paths.rs`
- Modify: `src-tauri/src/infra/db/migrations.rs`
- Create: `src-tauri/src/domain/plugins.rs`
- Create: `src-tauri/src/infra/plugins/mod.rs`
- Create: `src-tauri/src/infra/plugins/repository.rs`
- Create: `src-tauri/src/app/plugin_service.rs`
- Create: `src-tauri/src/commands/plugins.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Modify: `src-tauri/src/infra/mod.rs`
- Modify: `src-tauri/src/app/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/registry.rs`

**Frontend Files:**
- Create: `src/services/plugins.ts`
- Create: `src/query/plugins.ts`
- Create: `src/pages/PluginsPage.tsx`
- Create: `src/pages/plugins/PluginConfigSchemaForm.tsx`
- Create: `src/pages/plugins/pluginConfigValidation.ts`
- Modify: `src/app/AppRoutes.tsx`
- Modify: `src/layout/AppLayout.tsx`
- Modify: `src/generated/bindings.ts` after running generation.

**Tests And Verification:**
- Backend red/green tests: `cd src-tauri && cargo test plugins`
- DB tests: `cd src-tauri && cargo test db`
- Command tests: `cd src-tauri && cargo test commands`
- Frontend tests: `pnpm test:unit src/services/__tests__/plugins.test.ts src/pages/__tests__/PluginsPage.test.tsx`
- Type generation: `pnpm tauri:gen-types && pnpm check:generated-bindings`
- Final unit gate: `pnpm tauri:check && pnpm typecheck`

**Steps:**
- [ ] Add path-safety tests for plugin IDs and plugin root subdirectories.
- [ ] Implement plugin path helpers that never join unchecked user path segments.
- [ ] Add idempotent SQLite migrations for plugin tables and indexes.
- [ ] Add domain types and manifest validation with structured errors.
- [ ] Add repository CRUD tests, then implement repository methods.
- [ ] Add service tests for install/config/permission/status behavior, then implement service orchestration.
- [ ] Add Specta-exported commands and command tests.
- [ ] Add frontend service/query tests, then service wrappers and React Query keys.
- [ ] Add plugin list/detail/config UI tests, then UI components.
- [ ] Regenerate IPC bindings and verify generated contracts.

## Unit M2: Gateway Hook Pipeline

**Files:**
- Create: `src-tauri/src/gateway/plugins/mod.rs`
- Create: `src-tauri/src/gateway/plugins/context.rs`
- Create: `src-tauri/src/gateway/plugins/permissions.rs`
- Create: `src-tauri/src/gateway/plugins/pipeline.rs`
- Modify: `src-tauri/src/gateway.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/middleware/provider_resolution.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/send.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/response/response_router.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_non_stream.rs`
- Modify: `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_event_stream.rs`
- Modify: `src-tauri/src/gateway/streams/usage_tee.rs`
- Update: `docs/plugin-system-rfc.md` with the final gateway hook chain diagram.

**Tests And Verification:**
- Context and permission tests: `cd src-tauri && cargo test gateway_plugin_context`
- Pipeline tests: `cd src-tauri && cargo test gateway_plugin_pipeline`
- Request hook tests: `cd src-tauri && cargo test gateway_plugin_request`
- Response hook tests: `cd src-tauri && cargo test gateway_plugin_response`
- Stream hook tests: `cd src-tauri && cargo test gateway_plugin_stream`
- Log hook tests: `cd src-tauri && cargo test plugin_log_redaction`
- Final unit gate: `pnpm tauri:test && pnpm tauri:check`

**Steps:**
- [ ] Freeze the current request flow with a failing test or doc checker that expects the final hook chain diagram.
- [ ] Add context trimming tests for body/header/sensitive-header visibility.
- [ ] Implement hook context structs without exposing internal mutable Rust references.
- [ ] Add permission-enforcement tests for unauthorized read/write actions.
- [ ] Implement permission trimming and result enforcement.
- [ ] Add pipeline tests for ordering, timeout, audit, fail-open, fail-closed, and circuit behavior.
- [ ] Implement the pipeline executor with bounded timeouts and English tracing logs.
- [ ] Add request-body hook integration tests, then wire after-body-read and before-provider-resolution hooks.
- [ ] Add send-before-upstream tests, then wire header/body mutations before `reqwest` send.
- [ ] Add response-header and non-stream response tests, then wire transformations with body size limits.
- [ ] Add streaming chunk tests for sliding-window detection and block events, then wire SSE handling.
- [ ] Add error hook and log-before-persist tests, then wire audit/redaction fallbacks.

## Unit M3: Official Declarative Plugins

**Files:**
- Create: `src-tauri/src/app/plugins/mod.rs`
- Create: `src-tauri/src/app/plugins/rule_runtime.rs`
- Create: `src-tauri/src/app/plugins/official.rs`
- Create plugin package fixtures under `src-tauri/tests/fixtures/plugins/official/`
- Add frontend fixtures if the GUI needs examples.

**Tests And Verification:**
- Rule runtime: `cd src-tauri && cargo test rule_plugin_runtime`
- Prompt optimizer: `cd src-tauri && cargo test official_prompt_optimizer_plugin`
- Safety detector: `cd src-tauri && cargo test official_safety_detector_plugin`
- Redactor: `cd src-tauri && cargo test official_redactor_plugin`
- Final unit gate: `pnpm tauri:test && pnpm tauri:check`

**Steps:**
- [ ] Add rule-runtime tests for JSON path, regex detect, replace, block, warn, and append-message actions.
- [ ] Implement rule runtime with regex size limits and timeout protection.
- [ ] Add prompt optimizer fixture tests for `messages`, `input`, and `prompt`.
- [ ] Implement official prompt optimizer manifest/rules/config.
- [ ] Add safety detector tests for non-stream and stream hits.
- [ ] Implement official safety detector manifest/rules/config.
- [ ] Add redactor tests for Bearer, GitHub token, URL query token, and DB connection string.
- [ ] Implement official redactor manifest/rules/config and default log/GUI-only mode.

## Unit M4: Package Distribution, Market, Signing, Update, Rollback

**Files:**
- Create: `src-tauri/src/infra/plugins/package.rs`
- Create: `src-tauri/src/infra/plugins/market.rs`
- Create: `src-tauri/src/infra/plugins/signing.rs`
- Create: `src-tauri/src/app/plugins/update.rs`
- Extend commands/services from M1.
- Extend GUI risk/update states from M1.

**Tests And Verification:**
- Package security: `cd src-tauri && cargo test plugin_package_security`
- Local install: `cd src-tauri && cargo test plugin_local_install`
- Market index: `cd src-tauri && cargo test plugin_market_index`
- Signature verification: `cd src-tauri && cargo test plugin_signature_verification`
- Update rollback: `cd src-tauri && cargo test plugin_update_rollback`
- Final unit gate: `pnpm tauri:test && pnpm tauri:check && pnpm typecheck`

**Steps:**
- [ ] Add zip-slip, missing-manifest, and size-limit package tests.
- [ ] Implement `.aio-plugin` extraction into temp directories with path canonicalization.
- [ ] Add local install rollback tests, then implement cache/temp/installed move flow.
- [ ] Add market index parsing and compatibility tests, then implement index service.
- [ ] Add checksum/signature/revoked tests, then implement Ed25519 verification.
- [ ] Add unsigned offline install risk tests, then implement developer-mode restrictions.
- [ ] Add update permission-delta and config-migration rollback tests, then implement rollback snapshots.
- [ ] Add GUI tests for unsigned/revoked/update/risk labels, then implement UI states.

## Unit M5: Safe Code Runtime Foundations

**Files:**
- Create: `docs/plugins/wasm-runtime.md`
- Create: `docs/plugins/process-runtime-poc.md`
- Add backend runtime modules only after the ABI docs and tests freeze.

**Tests And Verification:**
- WASM design checker: `pnpm check:plugin-system-docs`
- Runtime tests after implementation: `cd src-tauri && cargo test plugin_wasm`
- Process PoC tests: `cd src-tauri && cargo test plugin_process_runtime_poc`
- Final unit gate: `pnpm tauri:test && pnpm tauri:check`

**Steps:**
- [ ] Extend docs checker for WASM ABI and process runtime boundaries.
- [ ] Write WASM runtime design with memory/time/filesystem/network restrictions.
- [ ] Add WASM host tests for valid execution, file denial, and dead-loop termination.
- [ ] Add the minimal Wasmtime dependency and executor only after failing tests exist.
- [ ] Write JSON-RPC-over-stdio process PoC design.
- [ ] Add process lifecycle tests for start timeout, hook timeout, crash, and idle recycle.
- [ ] Implement the minimal PoC without enabling marketplace use by default.

## Unit M6: SDK, Scaffolding, Debugging, Developer Docs

**Files:**
- Create: `packages/plugin-sdk/`
- Create: `packages/create-aio-plugin/`
- Create: `docs/plugins/getting-started.md`
- Create: `docs/plugins/manifest.md`
- Create: `docs/plugins/hooks.md`
- Create: `docs/plugins/permissions.md`
- Create: `docs/plugins/config-schema.md`
- Create: `docs/plugins/security.md`
- Create: `docs/plugins/streaming.md`
- Create: `docs/plugins/publishing.md`
- Create: `docs/plugins/compatibility.md`

**Tests And Verification:**
- SDK compile: `pnpm --filter @aio-coding-hub/plugin-sdk typecheck`
- Scaffolder tests: `pnpm --filter create-aio-plugin test`
- Docs checker: `pnpm check:plugin-system-docs`
- Final unit gate: `pnpm test:unit && pnpm typecheck && pnpm tauri:test && pnpm tauri:check`

**Steps:**
- [ ] Add SDK type tests, then implement manifest/context/result/permission exports.
- [ ] Add scaffolder snapshot tests, then implement rule/WASM templates.
- [ ] Add local dev command fixture tests, then implement manifest validation and hook replay.
- [ ] Add pack/sign/verify CLI tests, then implement commands on top of M4 package/signing code.
- [ ] Write developer docs and validate required pages with the docs checker.

## Final Completion Gate

- [ ] Re-read `docs/plugin-system-development-plan.md` and check every `Mx-Txx` item against implemented code/docs/tests.
- [ ] Run `pnpm check:plugin-system-docs`.
- [ ] Run `pnpm check:generated-bindings`.
- [ ] Run `pnpm test:unit`.
- [ ] Run `pnpm tauri:test`.
- [ ] Run `pnpm tauri:check`.
- [ ] Run `pnpm tauri:clippy` if earlier gates pass.
- [ ] Inspect `git status --short` and summarize all changed files.
