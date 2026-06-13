# Plugin System vNext Stabilization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the current plugin-system prototype into a truthful, stable, performant vNext release where every public hook, permission, runtime, SDK type, scaffold command, and official plugin asset matches real host behavior.

**Architecture:** Stabilize the public contract before expanding power. vNext keeps `declarativeRules` and `official.privacy-filter` as the reliable baseline, routes all runtime execution through a small registry, treats WASM/process as policy-gated isolated runtimes, and rejects or clearly labels future-only capabilities until they are actually wired. Performance work focuses on the gateway hot path: pre-index enabled plugins by hook, release runtime-cache locks before execution, and add circuit cooldown so transient failures do not disable a plugin forever.

**Tech Stack:** Rust/Tauri 2/Axum/SQLite/Specta on the backend; React/Vite/TypeScript/React Query/Vitest on the frontend; `pnpm`, Cargo, and repository-local documentation checkers for verification.

---

## Reference Practices

These are the external engineering practices this plan follows:

- VS Code extensions separate manifest contribution points, activation events, and runtime API; extensions are loaded lazily in an extension host for stability and performance: <https://code.visualstudio.com/api/get-started/extension-anatomy> and <https://code.visualstudio.com/api/advanced-topics/extension-host>.
- Chrome extensions require manifest-declared permissions and use declarative request rules for performance-sensitive network modification: <https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions> and <https://developer.chrome.com/docs/extensions/reference/api/declarativeNetRequest>.
- JetBrains plugins declare extension points and compatibility through `plugin.xml`; the host owns the extension-point surface and version compatibility: <https://plugins.jetbrains.com/docs/intellij/plugin-extension-points.html> and <https://plugins.jetbrains.com/docs/intellij/plugin-configuration-file.html>.
- Obsidian plugins use a compact manifest with a minimum app version, which is the same compatibility principle AIO should preserve through `hostCompatibility`: <https://docs.obsidian.md/Reference/Manifest>.

## Current-State Evidence

The plan is based on the current worktree, not only the historical plan:

- `docs/plugin-system-development-plan.md` describes the broad M0-M6 system.
- `docs/plugins/architecture-audit.md` already records the official-catalog decision: only `official.privacy-filter` stays bundled.
- `src-tauri/src/domain/plugins.rs` accepts nine hook names and permissions such as `plugin.storage`, `network.fetch`, `file.read`, `file.write`, and `secret.read`.
- Real gateway calls currently cover `gateway.request.afterBodyRead`, `gateway.request.beforeSend`, `gateway.response.after`, `gateway.response.chunk`, `gateway.error`, and `log.beforePersist`.
- `src-tauri/src/gateway/control_service.rs` wires the live pipeline to `RuleRuntimeGatewayPluginExecutor::default()`.
- `src-tauri/src/app/plugins/rule_runtime.rs` special-cases `official.privacy-filter`, then assumes every other plugin is `declarativeRules`.
- `packages/create-aio-plugin/src/devtools.ts` validates/replays/packs an in-memory sample scaffold instead of the plugin directory passed by the user.
- Official Privacy Filter assets currently live under `src-tauri/tests/fixtures/plugins/official/privacy-filter`, which is not an app-resource location.

## vNext Success Criteria

- Public vNext docs, Rust validation, generated TypeScript bindings, `@aio-coding-hub/plugin-sdk`, and `create-aio-plugin` agree on active hooks, reserved hooks, active permissions, reserved permissions, runtime support, and error messages.
- A community plugin cannot install or enable with a hook/runtime/permission that the host will silently ignore.
- SDK hook-result types, WASM SDK hook-result types, and the gateway `GatewayHookResult` mutation envelope describe the same behavior.
- The Plugins UI gives users an explicit path to approve pending plugin permissions before enablement.
- WASM tooling is honest and binary-safe: either the runtime is fully wired behind host policy, or scaffolds/docs clearly keep it experimental without breaking `.wasm` artifacts.
- Config schema and hook timeout documentation match the actual host implementation, including any deliberately unsupported secret-store behavior.
- `official.privacy-filter` installs and runs from production-packaged assets, not from test fixtures.
- Gateway hook execution avoids avoidable global serialization and has a recovery path after transient plugin failures.
- Developer tooling validates, replays, and packs real plugin directories.
- Every unit below ends with its own tests before the next unit starts.

## Baseline Verification

- [ ] Run `git status -sb`.
  Expected: record existing dirty files; do not revert unrelated work.
- [ ] Run `pnpm check:plugin-system-docs`.
  Expected: either pass or record pre-existing documentation-contract failures.
- [ ] Run `pnpm check:plugin-system-completion`.
  Expected: either pass or record pre-existing plugin-system completion failures.
- [ ] Run `cd src-tauri && cargo test plugin --lib`.
  Expected: plugin-focused backend tests pass before vNext changes, or failures are documented as pre-existing.
- [ ] Run `pnpm create-aio-plugin:test`.
  Expected: current scaffolder tests pass before vNext changes, or failures are documented as pre-existing.
- [ ] Run `pnpm plugin-sdk:typecheck`.
  Expected: SDK type contract passes before vNext changes, or failures are documented as pre-existing.

## File Responsibility Map

- `src-tauri/src/domain/plugins.rs`: manifest model, hook/permission/runtime validation, active/reserved contract constants.
- `packages/plugin-sdk/src/index.ts`: public TypeScript authoring contract and local validation helpers.
- `src/generated/bindings.ts`: generated frontend binding contract; never hand-edit except generated output.
- `docs/plugin-manifest-v1.md`, `docs/plugins/*.md`: public developer-facing contract.
- `scripts/check-plugin-system-docs.mjs`: documentation contract gate.
- `src-tauri/src/app/plugins/official.rs`: official plugin catalog and install-time metadata.
- `src-tauri/src/app/plugins/official_assets.rs`: new packaged official asset materialization.
- `src-tauri/resources/plugins/official/privacy-filter/`: new production resource location for Privacy Filter manifest and rules.
- `src-tauri/src/app/plugins/runtime_executor.rs`: new runtime dispatcher that delegates by runtime kind.
- `src-tauri/src/app/plugins/rule_runtime.rs`: declarative rules runtime and official Privacy Filter engine loading.
- `src-tauri/src/app/plugins/wasm_runtime.rs`: WASM executor foundation, policy-gated until explicitly enabled.
- `src-tauri/src/gateway/plugins/pipeline.rs`: hot-path hook scheduling, circuit state, audit events, plugin snapshot.
- `packages/create-aio-plugin/src/devtools.ts`: real directory validation, replay, packing, signing, verification.
- `packages/create-aio-plugin/src/scaffold.ts`: scaffolds that only advertise supported vNext behavior by default.

---

## Unit 1: Public Contract Truth Table

**Goal:** Make vNext public contracts truthful: active hooks and permissions are accepted; reserved capabilities are documented and rejected with explicit errors until wired.

**Files:**
- Modify: `src-tauri/src/domain/plugins.rs`
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `docs/plugin-manifest-v1.md`
- Modify: `docs/plugins/hooks.md`
- Modify: `docs/plugins/permissions.md`
- Modify: `docs/plugins/manifest.md`
- Modify: `docs/plugins/README.md`
- Modify: `docs/plugins/wasm-runtime.md`
- Modify: `scripts/check-plugin-system-docs.mjs`
- Test: existing tests inside `src-tauri/src/domain/plugins.rs`
- Test: `packages/plugin-sdk/src/index.typecheck.ts`

- [ ] **Step 1: Add failing backend validation tests for inactive hooks**

Add tests in `src-tauri/src/domain/plugins.rs` test module:

```rust
#[test]
fn validate_manifest_rejects_reserved_hook_until_it_is_wired() {
    let mut manifest = valid_manifest();
    manifest.hooks = vec![PluginHook {
        name: "gateway.request.received".to_string(),
        priority: 0,
        failure_policy: Some("fail-open".to_string()),
    }];

    let err = validate_manifest(&manifest, env!("CARGO_PKG_VERSION"))
        .expect_err("reserved hook must not install silently");
    assert_eq!(err.code, "PLUGIN_RESERVED_HOOK");
    assert!(err.message.contains("gateway.request.received"));
}

#[test]
fn validate_manifest_accepts_active_vnext_hooks() {
    for hook_name in [
        "gateway.request.afterBodyRead",
        "gateway.request.beforeSend",
        "gateway.response.chunk",
        "gateway.response.after",
        "gateway.error",
        "log.beforePersist",
    ] {
        let mut manifest = valid_manifest();
        manifest.hooks = vec![PluginHook {
            name: hook_name.to_string(),
            priority: 0,
            failure_policy: Some("fail-open".to_string()),
        }];
        manifest.permissions = permissions_for_hook(hook_name);
        validate_manifest(&manifest, env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|err| panic!("active hook {hook_name} rejected: {err:?}"));
    }
}
```

- [ ] **Step 2: Run backend red test**

Run:

```bash
cd src-tauri && cargo test validate_manifest_rejects_reserved_hook_until_it_is_wired validate_manifest_accepts_active_vnext_hooks --lib
```

Expected: `validate_manifest_rejects_reserved_hook_until_it_is_wired` fails because the current validator accepts all known hooks.

- [ ] **Step 3: Implement active/reserved hook validation**

In `src-tauri/src/domain/plugins.rs`, add explicit helpers:

```rust
pub fn is_active_gateway_hook(hook: &str) -> bool {
    matches!(
        hook,
        "gateway.request.afterBodyRead"
            | "gateway.request.beforeSend"
            | "gateway.response.chunk"
            | "gateway.response.after"
            | "gateway.error"
            | "log.beforePersist"
    )
}

pub fn is_reserved_gateway_hook(hook: &str) -> bool {
    matches!(
        hook,
        "gateway.request.received"
            | "gateway.request.beforeProviderResolution"
            | "gateway.response.headers"
    )
}
```

Change `validate_hooks` so unknown hooks still return `PLUGIN_UNKNOWN_HOOK`, while reserved hooks return:

```rust
PluginValidationError::new(
    "PLUGIN_RESERVED_HOOK",
    format!("hook is reserved for a future host integration and is not active in plugin API v1: {}", hook.name),
)
```

- [ ] **Step 4: Add failing backend validation tests for reserved permissions**

Add:

```rust
#[test]
fn validate_manifest_rejects_reserved_permissions_until_host_apis_exist() {
    for permission in ["plugin.storage", "network.fetch", "file.read", "file.write", "secret.read"] {
        let mut manifest = valid_manifest();
        manifest.permissions.push(permission.to_string());
        let err = validate_manifest(&manifest, env!("CARGO_PKG_VERSION"))
            .expect_err("reserved permission must not be granted silently");
        assert_eq!(err.code, "PLUGIN_RESERVED_PERMISSION");
        assert!(err.message.contains(permission));
    }
}
```

- [ ] **Step 5: Run permission red test**

Run:

```bash
cd src-tauri && cargo test validate_manifest_rejects_reserved_permissions_until_host_apis_exist --lib
```

Expected: fail because current validation treats those permissions as known and valid.

- [ ] **Step 6: Implement reserved permission validation**

In `src-tauri/src/domain/plugins.rs`, add:

```rust
pub fn is_reserved_permission(permission: &str) -> bool {
    matches!(
        permission,
        "plugin.storage" | "network.fetch" | "file.read" | "file.write" | "secret.read"
    )
}
```

Update `validate_permissions` to return:

```rust
PluginValidationError::new(
    "PLUGIN_RESERVED_PERMISSION",
    format!("permission is reserved for a future host-mediated API and is not active in plugin API v1: {permission}"),
)
```

before risk lookup for reserved permissions.

- [ ] **Step 7: Mirror active/reserved contract in TypeScript SDK**

In `packages/plugin-sdk/src/index.ts`, split hook and permission unions into active and reserved exports:

```typescript
export type ActiveGatewayHookName =
  | "gateway.request.afterBodyRead"
  | "gateway.request.beforeSend"
  | "gateway.response.chunk"
  | "gateway.response.after"
  | "gateway.error"
  | "log.beforePersist";

export type ReservedGatewayHookName =
  | "gateway.request.received"
  | "gateway.request.beforeProviderResolution"
  | "gateway.response.headers";

export type GatewayHookName = ActiveGatewayHookName | ReservedGatewayHookName;
```

Update `validateManifest` to reject reserved hook names with `PLUGIN_RESERVED_HOOK`, matching backend wording.

Add:

```typescript
const RESERVED_PERMISSIONS = new Set<PluginPermission>([
  "plugin.storage",
  "network.fetch",
  "file.read",
  "file.write",
  "secret.read",
]);
```

Return `PLUGIN_RESERVED_PERMISSION` from `validateManifest` when those appear.

- [ ] **Step 8: Update SDK typecheck fixture**

In `packages/plugin-sdk/src/index.typecheck.ts`, add:

```typescript
const reservedHookManifest: PluginManifest = {
  ...manifest,
  hooks: [{ name: "gateway.request.received" }],
};
const reservedHookResult = validateManifest(reservedHookManifest);
if (reservedHookResult.ok || reservedHookResult.error.code !== "PLUGIN_RESERVED_HOOK") {
  throw new Error("reserved hook should be rejected by SDK validation");
}
```

- [ ] **Step 9: Run SDK red/green verification**

Run:

```bash
pnpm plugin-sdk:typecheck
```

Expected after implementation: exit 0.

- [ ] **Step 10: Update docs and docs checker**

Update docs so they include these exact phrases:

- `Active hooks in plugin API v1`
- `Reserved hooks for future host integration`
- `Reserved permissions for future host-mediated APIs`
- `WASM packages are installable only when host policy enables execution`

Update `scripts/check-plugin-system-docs.mjs` to assert those phrases in `docs/plugin-manifest-v1.md`, `docs/plugins/hooks.md`, `docs/plugins/permissions.md`, and `docs/plugins/wasm-runtime.md`.

- [ ] **Step 11: Run Unit 1 gate**

Run:

```bash
cd src-tauri && cargo test validate_manifest_ --lib
pnpm plugin-sdk:typecheck
pnpm check:plugin-system-docs
pnpm check:generated-bindings
```

Expected: all commands exit 0. If backend type changes affect Specta output, run `pnpm tauri:gen-types` before `pnpm check:generated-bindings`.

---

## Unit 2: Official Privacy Filter Packaged Assets

**Goal:** Move `official.privacy-filter` out of test fixtures and into production-packaged resources, then install it by materializing those resources into the plugin install root.

**Files:**
- Create: `src-tauri/resources/plugins/official/privacy-filter/plugin.json`
- Create: `src-tauri/resources/plugins/official/privacy-filter/rules/gitleaks.toml`
- Create: `src-tauri/src/app/plugins/official_assets.rs`
- Modify: `src-tauri/src/app/plugins/mod.rs`
- Modify: `src-tauri/src/app/plugins/official.rs`
- Modify: `src-tauri/src/app/plugin_service.rs`
- Modify: `src-tauri/src/commands/plugins.rs`
- Modify: `src-tauri/tauri.conf.json`
- Test: `src-tauri/src/app/plugins/official.rs`
- Test: `src-tauri/src/app/plugin_service.rs`

- [ ] **Step 1: Copy official assets into production resource directory**

Copy:

```bash
mkdir -p src-tauri/resources/plugins/official/privacy-filter/rules
cp src-tauri/tests/fixtures/plugins/official/privacy-filter/plugin.json src-tauri/resources/plugins/official/privacy-filter/plugin.json
cp src-tauri/tests/fixtures/plugins/official/privacy-filter/rules/gitleaks.toml src-tauri/resources/plugins/official/privacy-filter/rules/gitleaks.toml
```

Keep test fixtures until all official tests are migrated.

- [ ] **Step 2: Add failing test that official catalog no longer depends on test fixture root**

In `src-tauri/src/app/plugins/official.rs`, add:

```rust
#[test]
fn official_catalog_uses_packaged_privacy_filter_resource_root() {
    let fixture = official_plugin("official.privacy-filter").expect("privacy filter fixture");
    let root = fixture.root_dir.to_string_lossy();
    assert!(
        root.contains("resources/plugins/official/privacy-filter"),
        "official plugin root must be a packaged resource path, got {root}"
    );
}
```

- [ ] **Step 3: Run red test**

Run:

```bash
cd src-tauri && cargo test official_catalog_uses_packaged_privacy_filter_resource_root --lib
```

Expected: fail because current root uses `tests/fixtures/plugins/official`.

- [ ] **Step 4: Point official catalog at resource root**

Change the root constant in `src-tauri/src/app/plugins/official.rs`:

```rust
const OFFICIAL_RESOURCE_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/plugins/official"
);
```

Use `OFFICIAL_RESOURCE_ROOT` in `official_plugin_root`.

- [ ] **Step 5: Add materialization helper**

Create `src-tauri/src/app/plugins/official_assets.rs`:

```rust
use crate::shared::error::AppResult;
use std::path::{Path, PathBuf};

pub(crate) fn materialize_official_plugin(
    plugin_id: &str,
    source_root: &Path,
    installed_root: &Path,
    version: &str,
) -> AppResult<PathBuf> {
    let plugin_segment = crate::app_paths::plugin_id_path_segment(plugin_id)?;
    let version_segment = crate::app_paths::plugin_id_path_segment(version)?;
    let target = installed_root.join(plugin_segment).join(version_segment);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("failed to clear official plugin install dir {}: {e}", target.display()))?;
    }
    copy_dir_recursive(source_root, &target)?;
    Ok(target)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> AppResult<()> {
    std::fs::create_dir_all(target)
        .map_err(|e| format!("failed to create official plugin dir {}: {e}", target.display()))?;
    for entry in std::fs::read_dir(source)
        .map_err(|e| format!("failed to read official plugin source {}: {e}", source.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read official plugin entry: {e}"))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read official plugin entry type: {e}"))?;
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &destination).map_err(|e| {
                format!("failed to copy official plugin resource to {}: {e}", destination.display())
            })?;
        }
    }
    Ok(())
}
```

Register it in `src-tauri/src/app/plugins/mod.rs`:

```rust
pub(crate) mod official_assets;
```

- [ ] **Step 6: Change official install service to materialize assets**

Change `install_official_plugin` signature in `src-tauri/src/app/plugin_service.rs` to accept `installed_root: &Path`:

```rust
pub(crate) fn install_official_plugin(
    db: &crate::db::Db,
    plugin_id: &str,
    host_version: &str,
    installed_root: &Path,
) -> AppResult<PluginDetail> {
    let fixture = crate::app::plugins::official::official_plugin(plugin_id)?;
    let installed_dir = crate::app::plugins::official_assets::materialize_official_plugin(
        plugin_id,
        &fixture.root_dir,
        installed_root,
        &fixture.manifest.version,
    )?;
    install_plugin_manifest(
        db,
        fixture.manifest.clone(),
        PluginInstallSource::Official,
        Some(installed_dir.to_string_lossy().to_string()),
        host_version,
    )?;
    // keep existing config, permissions, audit, and return logic
}
```

Update `src-tauri/src/commands/plugins.rs` to pass:

```rust
let installed_dir = crate::app_paths::plugins_installed_dir(&app)?;
plugin_service::install_official_plugin(
    &db,
    &input.plugin_id,
    env!("CARGO_PKG_VERSION"),
    &installed_dir,
)
```

- [ ] **Step 7: Update service tests**

In existing official install tests in `src-tauri/src/app/plugin_service.rs`, create a temp installed root and call the new signature:

```rust
let installed_root = tempfile::tempdir().expect("installed root");
let detail = install_official_plugin(
    &db,
    "official.privacy-filter",
    env!("CARGO_PKG_VERSION"),
    installed_root.path(),
)
.expect("install official privacy filter");
assert!(detail.installed_dir.as_deref().unwrap_or_default().contains("official.privacy-filter"));
assert!(std::path::Path::new(detail.installed_dir.as_deref().unwrap()).join("rules/gitleaks.toml").exists());
```

- [ ] **Step 8: Include resource directory in Tauri bundle**

In `src-tauri/tauri.conf.json`, add the resource directory according to the existing schema style:

```json
"resources": ["resources/plugins/official/privacy-filter/**"]
```

If the file already has `bundle.resources`, merge this path without removing existing entries.

- [ ] **Step 9: Run Unit 2 gate**

Run:

```bash
cd src-tauri && cargo test official_ --lib
cd src-tauri && cargo test official_plugin_install_enable_and_uninstall_roundtrip --lib
pnpm tauri:check
```

Expected: all commands exit 0.

---

## Unit 3: Runtime Executor Registry And WASM Policy

**Goal:** Remove runtime dispatch from the rule executor, make unsupported/policy-disabled runtimes fail explicitly, and prepare a clean seam for future WASM execution.

**Files:**
- Create: `src-tauri/src/app/plugins/runtime_executor.rs`
- Modify: `src-tauri/src/app/plugins/mod.rs`
- Modify: `src-tauri/src/app/plugins/rule_runtime.rs`
- Modify: `src-tauri/src/gateway/control_service.rs`
- Modify: `src-tauri/src/domain/plugins.rs`
- Modify: `docs/plugins/wasm-runtime.md`
- Test: `src-tauri/src/app/plugins/runtime_executor.rs`

- [ ] **Step 1: Add failing registry tests**

Create `src-tauri/src/app/plugins/runtime_executor.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::plugins::context::GatewayVisibleHookContext;

    #[test]
    fn runtime_executor_returns_clear_error_for_policy_disabled_wasm() {
        let executor = RuntimeGatewayPluginExecutor::for_tests(RuntimeExecutionPolicy {
            wasm_enabled: false,
        });
        let plugin = wasm_plugin_detail("example.wasm");
        let context = GatewayVisibleHookContext::for_tests("gateway.request.afterBodyRead", "trace-1");
        let err = executor.execute_plugin_sync(&plugin, context).expect_err("wasm disabled");
        assert_eq!(err.code(), "PLUGIN_RUNTIME_DISABLED");
        assert!(err.to_string().contains("wasm"));
    }

    #[test]
    fn runtime_executor_delegates_declarative_rules_to_rule_runtime() {
        let executor = RuntimeGatewayPluginExecutor::for_tests(RuntimeExecutionPolicy {
            wasm_enabled: false,
        });
        let plugin = rule_plugin_detail_with_fixture("example.rules");
        let context = GatewayVisibleHookContext::for_tests("gateway.request.afterBodyRead", "trace-2");
        let result = executor.execute_plugin_sync(&plugin, context).expect("rule runtime executes");
        assert_eq!(result.action, crate::gateway::plugins::context::GatewayHookAction::Continue);
    }
}
```

Add tiny `for_tests` helpers behind `#[cfg(test)]` where needed. Do not expose them in production API.

- [ ] **Step 2: Run registry red test**

Run:

```bash
cd src-tauri && cargo test runtime_executor_ --lib
```

Expected: fail because `runtime_executor.rs` does not exist.

- [ ] **Step 3: Implement runtime execution policy**

In `runtime_executor.rs`, add:

```rust
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RuntimeExecutionPolicy {
    pub(crate) wasm_enabled: bool,
}
```

Add:

```rust
pub(crate) struct RuntimeGatewayPluginExecutor {
    rule_runtime: crate::app::plugins::rule_runtime::RuleRuntimeGatewayPluginExecutor,
    policy: RuntimeExecutionPolicy,
}
```

Implement `GatewayPluginExecutor` for `RuntimeGatewayPluginExecutor` and delegate each hook to `execute_plugin_sync`.

- [ ] **Step 4: Move official/native dispatch into the registry**

In `rule_runtime.rs`, remove the top-level `if plugin.summary.plugin_id == "official.privacy-filter"` from generic rule dispatch. Expose two explicit functions on `RuleRuntimeGatewayPluginExecutor`:

```rust
pub(crate) fn execute_declarative_rules_plugin(
    &self,
    plugin: &PluginDetail,
    context: GatewayVisibleHookContext,
) -> Result<GatewayHookResult, GatewayPluginError>

pub(crate) fn execute_official_privacy_filter_plugin(
    &self,
    plugin: &PluginDetail,
    context: GatewayVisibleHookContext,
) -> Result<GatewayHookResult, GatewayPluginError>
```

In the registry, dispatch:

```rust
match &plugin.manifest.runtime {
    PluginRuntime::DeclarativeRules { .. } => self.rule_runtime.execute_declarative_rules_plugin(plugin, context),
    PluginRuntime::Native { engine } if plugin.summary.plugin_id == "official.privacy-filter" && engine == "privacyFilter" => {
        self.rule_runtime.execute_official_privacy_filter_plugin(plugin, context)
    }
    PluginRuntime::Native { engine } => Err(GatewayPluginError::new(
        "PLUGIN_UNSUPPORTED_RUNTIME",
        format!("unsupported native plugin runtime engine: {engine}"),
    )),
    PluginRuntime::Wasm { .. } if !self.policy.wasm_enabled => Err(GatewayPluginError::new(
        "PLUGIN_RUNTIME_DISABLED",
        "wasm runtime execution is disabled by host policy",
    )),
    PluginRuntime::Wasm { .. } => Err(GatewayPluginError::new(
        "PLUGIN_WASM_NOT_WIRED",
        "wasm runtime policy is enabled but gateway execution is not wired in this release",
    )),
}
```

- [ ] **Step 5: Wire live gateway to registry**

In `src-tauri/src/gateway/control_service.rs`, replace:

```rust
Arc::new(RuleRuntimeGatewayPluginExecutor::default())
```

with:

```rust
Arc::new(crate::app::plugins::runtime_executor::RuntimeGatewayPluginExecutor::default())
```

- [ ] **Step 6: Update WASM docs to match policy**

In `docs/plugins/wasm-runtime.md`, state:

```markdown
In vNext, WASM manifests are part of the compatibility contract, but gateway execution is policy-gated. A plugin with `runtime.kind = "wasm"` must not be enabled unless host policy explicitly sets `wasm_enabled = true`; otherwise the gateway returns `PLUGIN_RUNTIME_DISABLED`.
```

- [ ] **Step 7: Run Unit 3 gate**

Run:

```bash
cd src-tauri && cargo test runtime_executor_ --lib
cd src-tauri && cargo test rule_runtime_executor_ --lib
cd src-tauri && cargo test plugin_wasm --lib
pnpm tauri:check
```

Expected: all commands exit 0. `plugin_wasm` may continue to cover the low-level WASM executor foundation without enabling marketplace execution.

---

## Unit 4: Gateway Hot Path Performance And Circuit Recovery

**Goal:** Reduce avoidable per-hook overhead and prevent transient plugin failures from permanently skipping plugins in a running gateway.

**Files:**
- Modify: `src-tauri/src/gateway/plugins/pipeline.rs`
- Modify: `src-tauri/src/app/plugins/rule_runtime.rs`
- Test: existing tests inside those files

- [ ] **Step 1: Add failing test for rule runtime cache lock release**

In `src-tauri/src/app/plugins/rule_runtime.rs`, add a concurrency test that uses a rule runtime with two simultaneous executions and an instrumented executor hook. The assertion is that both hook calls finish under a timeout larger than a single execution but smaller than serialized execution:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_runtime_cache_does_not_hold_mutex_during_execution() {
    let executor = RuleRuntimeGatewayPluginExecutor::default();
    let plugin = slow_test_rule_plugin();
    let context = slow_test_context();

    let start = std::time::Instant::now();
    let (first, second) = tokio::join!(
        async { executor.execute_declarative_rules_plugin(&plugin, context.clone()) },
        async { executor.execute_declarative_rules_plugin(&plugin, context.clone()) },
    );

    first.expect("first execution");
    second.expect("second execution");
    assert!(
        start.elapsed() < std::time::Duration::from_millis(180),
        "runtime executions appear serialized by cache lock"
    );
}
```

If the current rule runtime is too fast for timing to be reliable, use a test-only executor delay hook guarded by `#[cfg(test)]`.

- [ ] **Step 2: Run cache-lock red test**

Run:

```bash
cd src-tauri && cargo test rule_runtime_cache_does_not_hold_mutex_during_execution --lib
```

Expected: fail or hang beyond assertion until cache execution is moved outside the mutex.

- [ ] **Step 3: Store cached runtimes in `Arc` and release locks before execution**

Change:

```rust
cache: Mutex<HashMap<String, RuleRuntime>>,
privacy_filter_cache: Mutex<HashMap<String, PrivacyFilter>>,
```

to:

```rust
cache: Mutex<HashMap<String, std::sync::Arc<RuleRuntime>>>,
privacy_filter_cache: Mutex<HashMap<String, std::sync::Arc<PrivacyFilter>>>,
```

Add helper:

```rust
fn get_or_load_rule_runtime(&self, plugin: &PluginDetail) -> Result<std::sync::Arc<RuleRuntime>, GatewayPluginError> {
    let cache_key = rule_runtime_cache_key(plugin);
    {
        let cache = self.cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(runtime) = cache.get(&cache_key) {
            return Ok(std::sync::Arc::clone(runtime));
        }
    }
    let runtime = std::sync::Arc::new(load_rule_runtime(plugin).map_err(to_gateway_plugin_error)?);
    let mut cache = self.cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(std::sync::Arc::clone(
        cache.entry(cache_key).or_insert_with(|| runtime),
    ))
}
```

Execute after the lock is released.

- [ ] **Step 4: Add failing test for circuit cooldown**

In `pipeline.rs`, add:

```rust
#[tokio::test(flavor = "current_thread")]
async fn gateway_plugin_pipeline_allows_half_open_probe_after_cooldown() {
    let executor = InMemoryGatewayPluginExecutor::new()
        .with_request_async_handler("plugin.flaky", |_ctx| async {
            GatewayHookResult {
                request_body: Some("recovered".to_string()),
                ..GatewayHookResult::continue_unchanged()
            }
        });
    let pipeline = GatewayPluginPipeline::for_tests(
        vec![plugin("plugin.flaky", 10, vec!["request.body.read", "request.body.write"])],
        Arc::new(executor),
        GatewayPluginPipelineConfig {
            hook_timeout: Duration::from_secs(1),
            circuit_failure_threshold: 1,
            circuit_cooldown: Duration::from_millis(1),
        },
    );

    pipeline.force_open_circuit_for_tests("plugin.flaky");
    tokio::time::sleep(Duration::from_millis(2)).await;

    let output = pipeline.run_request_hook(request_input()).await.expect("half-open probe");
    assert_eq!(output.body.as_ref(), b"recovered");
    assert!(!pipeline.circuit_snapshot("plugin.flaky").open);
}
```

- [ ] **Step 5: Implement cooldown and half-open probe**

Extend `GatewayPluginPipelineConfig`:

```rust
pub(crate) circuit_cooldown: Duration,
```

Extend `GatewayPluginCircuitSnapshot`:

```rust
pub(crate) opened_at: Option<std::time::Instant>,
pub(crate) half_open: bool,
```

Change skip logic so an open circuit is skipped only while `opened_at.elapsed() < circuit_cooldown`; after cooldown, allow one probe and mark `half_open = true`. A successful probe calls `record_success`; a failed probe reopens the circuit and updates `opened_at`.

- [ ] **Step 6: Add hook-index snapshot test**

Add:

```rust
#[tokio::test(flavor = "current_thread")]
async fn gateway_plugin_pipeline_reuses_hook_index_after_refresh() {
    let pipeline = GatewayPluginPipeline::for_tests(
        vec![plugin("plugin.a", 10, vec!["request.body.read"])],
        Arc::new(InMemoryGatewayPluginExecutor::new()),
        GatewayPluginPipelineConfig::default(),
    );
    assert_eq!(pipeline.plugins_for_hook_count_for_tests(GatewayPluginHookName::RequestAfterBodyRead), 1);
    pipeline.replace_plugins(vec![plugin("plugin.b", 20, vec!["request.body.read"])]);
    assert_eq!(pipeline.plugins_for_hook_count_for_tests(GatewayPluginHookName::RequestAfterBodyRead), 1);
}
```

- [ ] **Step 7: Pre-index enabled plugins by hook**

Replace `plugins: RwLock<Arc<Vec<PluginDetail>>>` with a snapshot struct:

```rust
struct GatewayPluginSnapshot {
    all: Arc<Vec<PluginDetail>>,
    by_hook: std::collections::HashMap<GatewayPluginHookName, Arc<Vec<PluginDetail>>>,
}
```

Build `by_hook` in `for_runtime`, `for_tests`, and `replace_plugins`; sort once by `(priority, plugin_id)` during snapshot creation. `plugins_for_hook` clones only the `Arc<Vec<PluginDetail>>` and iterates without sorting.

- [ ] **Step 8: Run Unit 4 gate**

Run:

```bash
cd src-tauri && cargo test gateway_plugin_pipeline_ --lib
cd src-tauri && cargo test rule_runtime_executor_ --lib
cd src-tauri && cargo test official_privacy_filter --lib
pnpm tauri:check
```

Expected: all commands exit 0.

---

## Unit 5: Real Developer Tooling

**Goal:** Make `create-aio-plugin validate`, `replay`, and `pack` operate on real plugin directories so authors do not need to read host source code to know whether a plugin works.

**Files:**
- Modify: `packages/create-aio-plugin/src/devtools.ts`
- Modify: `packages/create-aio-plugin/src/cli.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.test.ts`
- Modify: `docs/plugins/getting-started.md`
- Modify: `docs/plugins/sdk.md`

- [ ] **Step 1: Add failing tests for real directory validation and packing**

In `packages/create-aio-plugin/src/scaffold.test.ts`, add:

```typescript
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

it("validate command reads plugin.json from a real plugin directory", () => {
  const root = mkdtempSync(join(tmpdir(), "aio-plugin-"));
  const files = createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" });
  for (const [path, content] of Object.entries(files)) {
    const fullPath = join(root, path);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, content);
  }

  const result = validatePluginDirectory(root);
  expect(result).toEqual({ ok: true });
});

it("pack command writes package bytes from a real plugin directory", () => {
  const root = mkdtempSync(join(tmpdir(), "aio-plugin-"));
  const files = createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" });
  for (const [path, content] of Object.entries(files)) {
    const fullPath = join(root, path);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, content);
  }

  const packed = packPluginDirectory(root);
  expect(packed.checksum).toMatch(/^sha256:/);
  expect(packed.bytes.length).toBeGreaterThan(64);
});
```

- [ ] **Step 2: Run devtools red test**

Run:

```bash
pnpm --filter create-aio-plugin test -- scaffold.test.ts
```

Expected: fail because `validatePluginDirectory` and `packPluginDirectory` do not exist.

- [ ] **Step 3: Implement directory readers**

In `devtools.ts`, add:

```typescript
export function readPluginDirectory(root: string): ScaffoldFiles {
  const files: ScaffoldFiles = {};
  walkPluginDirectory(root, root, files);
  return files;
}

function walkPluginDirectory(root: string, dir: string, files: ScaffoldFiles): void {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".git") continue;
    const fullPath = join(dir, entry.name);
    const relativePath = relative(root, fullPath).replace(/\\/g, "/");
    if (entry.isDirectory()) {
      walkPluginDirectory(root, fullPath, files);
    } else if (entry.isFile()) {
      files[relativePath] = readFileSync(fullPath, "utf8");
    }
  }
}

export function validatePluginDirectory(root: string): ValidationResult {
  return validatePluginFiles(readPluginDirectory(root));
}

export function packPluginDirectory(root: string): PackedPlugin {
  return packPlugin(readPluginDirectory(root));
}
```

Import `readdirSync`, `readFileSync`, and `relative`.

- [ ] **Step 4: Add failing replay test for actual declarative rule behavior**

Add:

```typescript
it("replay command applies scaffold rule to fixture context", () => {
  const files = createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" });
  const result = replayHook(files, "gateway.request.afterBodyRead", {
    request: { body: JSON.stringify({ messages: [{ role: "user", content: "SECRET_TOKEN" }] }) },
  });
  expect(result).toMatchObject({ action: "replace" });
  expect(JSON.stringify(result)).toContain("[REDACTED]");
});
```

- [ ] **Step 5: Implement minimal TypeScript replay for declarative rules**

In `devtools.ts`, change `replayHook` so it:

1. validates `plugin.json`;
2. loads each `runtime.rules` file from `files`;
3. filters rules by hook;
4. supports the scaffold-compatible subset: `target.kind === "jsonPath"` or `target.field === "request.body"` with `jsonPath`, `matcher.regex` or `match.regex`, and `action.kind === "replace"`;
5. returns:

```typescript
{
  action: "replace",
  contextPatch: {
    request: { body: nextBody }
  }
}
```

When no rule matches, return `{ action: "pass" }`.

- [ ] **Step 6: Update CLI command parsing**

Make CLI commands use real paths:

```bash
pnpm create-aio-plugin validate ./acme.redactor
pnpm create-aio-plugin replay ./acme.redactor fixtures/request.json gateway.request.afterBodyRead
pnpm create-aio-plugin pack ./acme.redactor
```

Keep scaffold command syntax:

```bash
pnpm create-aio-plugin acme.redactor rule
```

The CLI should print English errors such as:

```text
failed to validate plugin directory: missing plugin.json
failed to replay plugin hook: unsupported rule target
```

- [ ] **Step 7: Update docs**

Update `docs/plugins/getting-started.md` so the local flow uses real paths:

```bash
pnpm create-aio-plugin acme.redactor rule
pnpm create-aio-plugin validate ./acme.redactor
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/request.json gateway.request.afterBodyRead
pnpm create-aio-plugin pack ./acme.redactor
```

- [ ] **Step 8: Run Unit 5 gate**

Run:

```bash
pnpm create-aio-plugin:test
pnpm plugin-sdk:typecheck
pnpm check:plugin-system-docs
pnpm typecheck
```

Expected: all commands exit 0.

---

## Unit 6: Contract Drift Guard

**Goal:** Prevent Rust, generated bindings, SDK, docs, and scaffolder from drifting again.

**Files:**
- Create: `docs/plugins/plugin-api-v1-contract.json`
- Create: `scripts/check-plugin-api-contract.mjs`
- Modify: `package.json`
- Modify: `scripts/check-plugin-system-docs.mjs`
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
- Modify: `src-tauri/src/domain/plugins.rs`

- [ ] **Step 1: Add contract JSON**

Create `docs/plugins/plugin-api-v1-contract.json`:

```json
{
  "apiVersion": "1.0.0",
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

- [ ] **Step 2: Add failing contract checker**

Create `scripts/check-plugin-api-contract.mjs` that:

1. reads `docs/plugins/plugin-api-v1-contract.json`;
2. reads `packages/plugin-sdk/src/index.ts`;
3. reads `packages/create-aio-plugin/src/scaffold.ts`;
4. reads `src-tauri/src/domain/plugins.rs`;
5. fails if any hook/permission/runtime from the JSON is missing from the corresponding file;
6. fails if reserved hooks are missing from docs.

The script should print English diagnostics:

```text
Plugin API contract check failed:
- packages/plugin-sdk/src/index.ts is missing active hook gateway.response.after
```

- [ ] **Step 3: Wire script**

Add package script:

```json
"check:plugin-api-contract": "node scripts/check-plugin-api-contract.mjs"
```

Add it to `check:plugin-system-completion` or the nearest plugin-system aggregate checker.

- [ ] **Step 4: Run red test**

Run:

```bash
pnpm check:plugin-api-contract
```

Expected before synchronization: fail if any current file still advertises stale/future-only behavior.

- [ ] **Step 5: Synchronize files to contract**

Update:

- `packages/plugin-sdk/src/index.ts` constants.
- `packages/create-aio-plugin/src/scaffold.ts` default runtime and hooks.
- `src-tauri/src/domain/plugins.rs` active/reserved helpers.
- Docs references that list hooks, permissions, and runtimes.

- [ ] **Step 6: Run Unit 6 gate**

Run:

```bash
pnpm check:plugin-api-contract
pnpm check:plugin-system-docs
pnpm check:plugin-system-completion
pnpm check:generated-bindings
```

Expected: all commands exit 0.

---

## Unit 7: End-To-End Privacy Filter Behavior Matrix

**Goal:** Prove the official Privacy Filter protects Codex and Claude gateway payload shapes consistently before upstream and before log persistence.

**Files:**
- Modify: `src-tauri/src/app/plugins/official.rs`
- Modify: `src-tauri/src/app/plugins/privacy_filter.rs`
- Modify: `src-tauri/tests/fixtures/plugins/official/privacy-filter/plugin.json`
- Modify: `src-tauri/resources/plugins/official/privacy-filter/plugin.json`
- Test: `src-tauri/src/app/plugins/official.rs`
- Test: `src-tauri/src/app/plugins/privacy_filter.rs`

- [ ] **Step 1: Add request-shape matrix tests**

Add tests for these bodies:

```rust
#[tokio::test]
async fn official_privacy_filter_redacts_codex_responses_input_text() {
    assert_privacy_filter_redacts_request_json(serde_json::json!({
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "你知道 13344441520 是哪里的手机号嘛"
            }]
        }]
    }), "13344441520", "[电话]").await;
}

#[tokio::test]
async fn official_privacy_filter_redacts_claude_messages_content_text() {
    assert_privacy_filter_redacts_request_json(serde_json::json!({
        "messages": [{
            "role": "user",
            "content": [{
                "type": "text",
                "text": "邮箱 test.user@example.com 手机 13812345678"
            }]
        }]
    }), "test.user@example.com", "[邮箱]").await;
}

#[tokio::test]
async fn official_privacy_filter_redacts_plain_prompt_field() {
    assert_privacy_filter_redacts_request_json(serde_json::json!({
        "prompt": "OpenAI sk-proj-abcdefghijklmnopqrstuvwxyz123456"
    }), "sk-proj-abcdefghijklmnopqrstuvwxyz123456", "[密钥]").await;
}
```

- [ ] **Step 2: Run matrix red/green tests**

Run:

```bash
cd src-tauri && cargo test official_privacy_filter_redacts_ --lib
```

Expected after implementation: all matrix cases pass.

- [ ] **Step 3: Add log persistence matrix test**

Add:

```rust
#[tokio::test]
async fn official_privacy_filter_redacts_log_payload_before_persist() {
    let output = run_privacy_filter_log_hook(
        "trace-log",
        r#"{"requestBody":"phone=13344441520","responseBody":"token=ghp_abcdefghijklmnopqrstuvwxyzABCDEFGHIJ"}"#,
    )
    .await;
    assert!(output.message.contains("[电话]"));
    assert!(output.message.contains("[密钥]"));
    assert!(!output.message.contains("13344441520"));
    assert!(!output.message.contains("ghp_abcdefghijklmnopqrstuvwxyzABCDEFGHIJ"));
}
```

- [ ] **Step 4: Align fixture and resource manifests**

If tests require manifest changes, update both:

- `src-tauri/tests/fixtures/plugins/official/privacy-filter/plugin.json`
- `src-tauri/resources/plugins/official/privacy-filter/plugin.json`

Then add a test:

```rust
#[test]
fn official_privacy_filter_fixture_and_resource_manifests_match() {
    let fixture = std::fs::read_to_string("tests/fixtures/plugins/official/privacy-filter/plugin.json").unwrap();
    let resource = std::fs::read_to_string("resources/plugins/official/privacy-filter/plugin.json").unwrap();
    assert_eq!(fixture, resource);
}
```

- [ ] **Step 5: Run Unit 7 gate**

Run:

```bash
cd src-tauri && cargo test privacy_filter --lib
cd src-tauri && cargo test official_privacy_filter --lib
pnpm tauri:check
```

Expected: all commands exit 0.

---

## Unit 8: Final vNext Verification And Documentation Closeout

**Goal:** Make the release state auditable and ready for PR review.

**Files:**
- Modify: `docs/plugins/architecture-audit.md`
- Modify: `docs/plugins/README.md`
- Modify: `README.md`
- Modify: `docs/plugin-system-development-plan.md` only if it still claims retired official examples are bundled.

- [ ] **Step 1: Update architecture audit with vNext decisions**

Add a `vNext Stabilization Decisions` section to `docs/plugins/architecture-audit.md` with:

```markdown
## vNext Stabilization Decisions

- Active plugin API v1 only accepts hooks that are wired into the gateway/log pipeline.
- Reserved hooks and reserved permissions are documented but rejected during manifest validation until the host implements them.
- WASM remains policy-gated. The runtime foundation exists, but community WASM execution is not enabled silently.
- Official Privacy Filter assets are packaged as app resources and materialized into the plugin install root.
- Runtime dispatch goes through a registry rather than rule-runtime-specific branching.
```

- [ ] **Step 2: Update README plugin section**

Ensure `README.md` links to:

- `docs/plugins/README.md`
- `docs/superpowers/plans/2026-06-12-plugin-system-vnext.md`

Use concise wording that the plugin API is vNext-stabilized around `declarativeRules` and `official.privacy-filter`.

- [ ] **Step 3: Remove or rewrite stale claims**

Run:

```bash
rg -n "prompt optimizer|safety detector|generic redactor|gateway.request.received|gateway.response.headers|network.fetch|secret.read|WASM runtime is the first supported community code-plugin runtime" README.md docs packages src-tauri/src
```

For each result, either:

- keep it only if it is clearly marked as a community pattern, reserved capability, or policy-gated runtime; or
- rewrite it to match vNext truth.

- [ ] **Step 4: Run focused backend gates**

Run:

```bash
cd src-tauri && cargo test official_ --lib
cd src-tauri && cargo test plugin --lib
cd src-tauri && cargo test privacy_filter --lib
cd src-tauri && cargo fmt -- --check
cd src-tauri && cargo check --locked
```

Expected: all commands exit 0.

- [ ] **Step 5: Run focused frontend/tooling gates**

Run:

```bash
pnpm plugin-sdk:typecheck
pnpm create-aio-plugin:test
pnpm plugin-wasm-sdk:test
pnpm check:plugin-api-contract
pnpm check:plugin-system-docs
pnpm check:plugin-system-completion
pnpm check:generated-bindings
pnpm check:precommit:src
```

Expected: all commands exit 0.

- [ ] **Step 6: Run final diff hygiene**

Run:

```bash
git diff --check
git status -sb
```

Expected: no whitespace errors. Status shows only intentional vNext files.

---

## Unit 9: Hook Result ABI Truth

**Goal:** Remove the current SDK/host ABI ambiguity by making TypeScript SDK, Rust/WASM SDK, WASM docs, and host conversion tests agree on one hook-result envelope.

**Why:** Mature plugin systems keep their public API and runtime ABI boringly stable. A plugin author should not need to know that the host internally uses `request_body` while the SDK exposes `contextPatch`.

**Files:**
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `packages/plugin-sdk/src/index.typecheck.ts`
- Modify: `packages/plugin-sdk/src/index.test.ts`
- Modify: `packages/plugin-wasm-sdk/src/lib.rs`
- Modify: `packages/plugin-wasm-sdk/tests/sdk_contract.rs`
- Modify: `packages/plugin-wasm-sdk/examples/redactor/src/lib.rs`
- Modify: `docs/plugins/sdk.md`
- Modify: `docs/plugins/wasm-runtime.md`
- Modify: `src-tauri/src/app/plugins/wasm_runtime.rs`
- Test: `packages/plugin-sdk/src/index.test.ts`
- Test: `packages/plugin-wasm-sdk/tests/sdk_contract.rs`
- Test: `src-tauri/src/app/plugins/wasm_runtime.rs`

- [x] **Step 1: Add failing SDK type tests for host mutation fields**

In `packages/plugin-sdk/src/index.typecheck.ts`, add examples that use host-visible mutation names:

```typescript
const replaceRequestResult: PluginHookResult = {
  action: "replace",
  requestBody: "{\"messages\":[]}",
};

const replaceResponseHeadersResult: PluginHookResult = {
  action: "replace",
  headers: { "x-plugin-redacted": "1" },
  responseBody: "{\"ok\":true}",
};
```

Run:

```bash
pnpm plugin-sdk:typecheck
```

Expected before implementation: TypeScript rejects `requestBody`, `responseBody`, or `headers` if the SDK still only exposes `contextPatch`.

- [x] **Step 2: Update TypeScript hook-result contract**

Change `PluginHookResult` in `packages/plugin-sdk/src/index.ts` to this shape:

```typescript
export type PluginHookResult =
  | { action: "pass"; audit?: JsonValue[] }
  | { action: "warn"; message: string; audit?: JsonValue[] }
  | { action: "block"; reason: string; audit?: JsonValue[] }
  | {
      action: "replace";
      requestBody?: string;
      responseBody?: string;
      streamChunk?: string;
      logMessage?: string;
      headers?: Record<string, string>;
      audit?: JsonValue[];
    };
```

Do not keep `contextPatch` as an active vNext result field unless a host converter and tests are implemented in the same unit.

- [x] **Step 3: Add failing Rust/WASM SDK serialization test**

In `packages/plugin-wasm-sdk/tests/sdk_contract.rs`, add:

```rust
#[test]
fn hook_result_serializes_host_mutation_fields() {
    let result = aio_plugin_wasm_sdk::HookResult::replace_request_body("{\"messages\":[]}");
    let json = serde_json::to_value(result).expect("serialize hook result");

    assert_eq!(json["action"], "replace");
    assert_eq!(json["requestBody"], "{\"messages\":[]}");
    assert!(json.get("contextPatch").is_none());
}
```

Run:

```bash
cargo test --manifest-path packages/plugin-wasm-sdk/Cargo.toml hook_result_serializes_host_mutation_fields
```

Expected before implementation: compile failure or assertion failure if the SDK still serializes `contextPatch`.

- [x] **Step 4: Update Rust/WASM SDK result model**

In `packages/plugin-wasm-sdk/src/lib.rs`, replace `context_patch` with explicit optional fields:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub request_body: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub response_body: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub stream_chunk: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub log_message: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub headers: Option<std::collections::BTreeMap<String, String>>,
```

Add constructors:

```rust
pub fn replace_request_body(body: impl Into<String>) -> Self
pub fn replace_response_body(body: impl Into<String>) -> Self
pub fn replace_stream_chunk(chunk: impl Into<String>) -> Self
pub fn replace_log_message(message: impl Into<String>) -> Self
```

Update the redactor example to use `replace_request_body`.

- [x] **Step 5: Add host converter tests even while WASM remains policy-gated**

In `src-tauri/src/app/plugins/wasm_runtime.rs`, add a small pure converter function:

```rust
fn gateway_hook_result_from_wasm_output(value: serde_json::Value) -> AppResult<GatewayHookResult>
```

Add tests:

```rust
#[test]
fn wasm_output_replace_request_body_maps_to_gateway_hook_result() {
    let result = gateway_hook_result_from_wasm_output(serde_json::json!({
        "action": "replace",
        "requestBody": "{\"messages\":[]}",
        "headers": { "x-plugin-redacted": "1" }
    }))
    .expect("wasm output maps");

    assert_eq!(result.request_body.as_deref(), Some("{\"messages\":[]}"));
    assert_eq!(result.headers.get("x-plugin-redacted").map(String::as_str), Some("1"));
}

#[test]
fn wasm_output_rejects_legacy_context_patch_in_vnext() {
    let err = gateway_hook_result_from_wasm_output(serde_json::json!({
        "action": "replace",
        "contextPatch": { "request": { "body": "x" } }
    }))
    .expect_err("legacy contextPatch is not active vNext ABI");

    assert_eq!(err.code(), "PLUGIN_WASM_INVALID_OUTPUT");
}
```

- [x] **Step 6: Run Unit 9 gate**

Run:

```bash
pnpm plugin-sdk:typecheck
pnpm --filter @aio-coding-hub/plugin-sdk test
cargo test --manifest-path packages/plugin-wasm-sdk/Cargo.toml
cargo test --manifest-path packages/plugin-wasm-sdk/examples/redactor/Cargo.toml
cd src-tauri && cargo test wasm_output_ --lib
pnpm check:plugin-api-contract
```

Expected: all commands exit 0, and no public docs describe `contextPatch` as an active vNext gateway mutation field.

---

## Unit 10: Permission Authorization UX Loop

**Goal:** Make community plugin installation, permission approval, and enablement a complete user-facing workflow.

**Why:** A plugin that installs but cannot be enabled without discovering an invisible backend command is not a usable plugin system.

**Files:**
- Modify: `src/pages/PluginsPage.tsx`
- Modify: `src/pages/__tests__/PluginsPage.test.tsx`
- Modify: `src/query/plugins.ts`
- Modify: `src/services/plugins.ts`
- Modify: `src-tauri/src/app/plugin_service.rs`
- Modify: `src-tauri/src/infra/plugins/repository.rs`
- Test: `src/pages/__tests__/PluginsPage.test.tsx`
- Test: `src-tauri/src/app/plugin_service.rs`

- [x] **Step 1: Add failing frontend test for pending permission approval**

In `src/pages/__tests__/PluginsPage.test.tsx`, import `usePluginGrantPermissionsMutation` in the mocked hook list and add:

```typescript
it("lets the user approve pending plugin permissions from the detail panel", async () => {
  const grantMutation = mutation();
  vi.mocked(usePluginGrantPermissionsMutation).mockReturnValue(grantMutation as any);
  vi.mocked(usePluginsListQuery).mockReturnValue({
    data: [summary()],
    isLoading: false,
    isFetching: false,
    error: null,
  } as any);

  renderWithProviders(<PluginsPage />);

  fireEvent.click(screen.getByRole("button", { name: "授权待审批权限" }));

  await waitFor(() => {
    expect(grantMutation.mutateAsync).toHaveBeenCalledWith({
      pluginId: "community.prompt-helper",
      permissions: ["request.body.write"],
    });
  });
});
```

Run:

```bash
pnpm test:unit src/pages/__tests__/PluginsPage.test.tsx -t "approve pending plugin permissions"
```

Expected before implementation: the button is missing or the mutation is not wired.

- [x] **Step 2: Wire the frontend mutation**

In `src/pages/PluginsPage.tsx`:

1. Import `usePluginGrantPermissionsMutation`.
2. Create the mutation in `PluginsPage`.
3. Pass `onGrantPendingPermissions` into `PluginDetailPanel`.
4. Render a button when `detail.pending_permissions.length > 0`:

```tsx
<Button
  size="sm"
  disabled={busy}
  onClick={() => onGrantPendingPermissions(detail.summary.plugin_id, detail.pending_permissions)}
>
  <ShieldAlert className="h-3.5 w-3.5" />
  授权待审批权限
</Button>
```

Use existing toast/error patterns for success and failure.

- [x] **Step 3: Add backend test that local installs create pending permissions**

In `src-tauri/src/app/plugin_service.rs`, add a test that installs a local declarative plugin package with `request.body.read` and `request.body.write`, then asserts:

```rust
assert!(detail.granted_permissions.is_empty());
assert_eq!(
    detail.pending_permissions,
    vec!["request.body.read".to_string(), "request.body.write".to_string()]
);
```

Run:

```bash
cd src-tauri && cargo test local_plugin_install_records_manifest_permissions_as_pending --lib
```

Expected before implementation: pending permissions are empty unless this behavior already exists.

- [x] **Step 4: Persist pending permissions on local and remote install**

After `repository::insert_plugin`, call:

```rust
repository::save_plugin_permissions(db, &plugin_id, &[], &manifest.permissions)?;
```

for local, marketplace, GitHub release, and offline installs. Keep official plugin install behavior granting official Privacy Filter permissions automatically because it is host-owned.

- [x] **Step 5: Add enable failure copy test**

Add a frontend test that clicking `启用` when pending permissions exist shows the pending-permission call to action before or alongside the backend failure toast.

Expected UX: the user can recover by clicking `授权待审批权限`; the page must not leave them at a dead end.

- [x] **Step 6: Run Unit 10 gate**

Run:

```bash
pnpm test:unit src/pages/__tests__/PluginsPage.test.tsx
cd src-tauri && cargo test plugin_service --lib
pnpm check:generated-bindings
pnpm check:precommit:src
```

Expected: all commands exit 0. If command signatures change, run `pnpm tauri:gen-types` before `pnpm check:generated-bindings`.

---

## Unit 11: WASM Tooling Honesty And Binary-Safe Packaging

**Goal:** Keep WASM clearly policy-gated while making plugin packages safe for binary artifacts.

**Why:** It is acceptable for WASM execution to remain disabled by policy. It is not acceptable for the official tooling to corrupt `plugin.wasm` or imply that generated WASM plugins will run in the gateway today.

**Files:**
- Modify: `packages/create-aio-plugin/src/devtools.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.test.ts`
- Modify: `docs/plugins/getting-started.md`
- Modify: `docs/plugins/wasm-runtime.md`
- Modify: `docs/plugins/sdk.md`
- Modify: `scripts/check-plugin-system-completion.mjs`
- Test: `packages/create-aio-plugin/src/scaffold.test.ts`

- [x] **Step 1: Add failing binary packaging test**

In `packages/create-aio-plugin/src/scaffold.test.ts`, add:

```typescript
it("packs binary wasm artifacts without utf8 rewriting", () => {
  const wasmBytes = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0xff, 0x00, 0x80]);
  const packed = packPluginBytes({
    "plugin.json": new TextEncoder().encode(JSON.stringify(validWasmManifest())),
    "plugin.wasm": wasmBytes,
  });

  expect(packed.checksum).toMatch(/^sha256:/);
  expect(readStoredZipEntry(packed.bytes, "plugin.wasm")).toEqual(wasmBytes);
});
```

Run:

```bash
pnpm create-aio-plugin:test -t "binary wasm"
```

Expected before implementation: `packPluginBytes` or `readStoredZipEntry` does not exist, or the assertion fails because packaging is text-only.

- [x] **Step 2: Introduce a binary-safe file map**

In `packages/create-aio-plugin/src/devtools.ts`, add:

```typescript
export type PluginFileBytes = Record<string, Uint8Array>;

export function readPluginDirectoryBytes(root: string): PluginFileBytes
export function packPluginBytes(files: PluginFileBytes): PackedPlugin
```

Keep existing `ScaffoldFiles` helpers for text scaffolds, but route real directory packing through byte readers. `validatePluginDirectory` may read `plugin.json` as UTF-8; `packPluginDirectory` must not.

- [x] **Step 3: Make the WASM scaffold impossible to mistake for enabled runtime**

Keep `create-aio-plugin <id> wasm`, but generate a README that says:

```text
WASM gateway execution is policy-gated and disabled by default in vNext.
```

Do not add a new CLI flag in this unit; the goal is truthful generated copy and binary-safe packaging, not a CLI mode redesign.

- [x] **Step 4: Add scaffold test for explicit experimental copy**

Assert the generated WASM README contains:

```text
policy-gated and disabled by default in vNext
```

Run:

```bash
pnpm create-aio-plugin:test -t "wasm scaffold"
```

- [x] **Step 5: Update docs and completion checker**

Update docs so they never say WASM is the default community code runtime. Required phrases:

- `WASM gateway execution is policy-gated`
- `declarativeRules is the default community runtime`
- `plugin.wasm artifacts are packaged as binary files`

Update `scripts/check-plugin-system-completion.mjs` to assert those phrases in `docs/plugins/wasm-runtime.md` and `docs/plugins/getting-started.md`.

- [x] **Step 6: Run Unit 11 gate**

Run:

```bash
pnpm create-aio-plugin:test
pnpm check:plugin-system-completion
pnpm check:plugin-system-docs
git diff --check
```

Expected: all commands exit 0.

---

## Unit 12: Config Schema And Timeout Contract Reconciliation

**Goal:** Make config schema and hook timeout/failure-policy docs exactly match host behavior.

**Why:** The small-tool version of a strong plugin platform is not a huge API surface; it is a truthful one.

**Files:**
- Modify: `docs/plugin-manifest-v1.md`
- Modify: `docs/plugins/config-schema.md`
- Modify: `docs/plugins/hooks.md`
- Modify: `docs/plugins/security.md`
- Modify: `scripts/check-plugin-system-docs.mjs`
- Modify: `src/pages/plugins/PluginConfigSchemaForm.tsx`
- Modify: `src/pages/plugins/pluginConfigValidation.ts`
- Modify: `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx` or create it if missing.
- Test: frontend config schema tests.

- [ ] **Step 1: Add failing config form tests for enum and password truth**

Create or update `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`:

```typescript
it("renders enum as a select when enum is a string-field keyword", () => {
  render(
    <PluginConfigSchemaForm
      schema={{
        type: "object",
        properties: {
          mode: { type: "string", enum: ["strict", "balanced"] },
        },
      }}
      value={{ mode: "strict" }}
      pending={false}
      onSubmit={vi.fn()}
    />
  );

  expect(screen.getByRole("combobox", { name: "mode" })).toBeInTheDocument();
});

it("renders password fields as password inputs without claiming host secret storage", () => {
  render(
    <PluginConfigSchemaForm
      schema={{
        type: "object",
        properties: {
          token: { type: "password" },
        },
      }}
      value={{ token: "saved-token" }}
      pending={false}
      onSubmit={vi.fn()}
    />
  );

  expect(screen.getByLabelText("token")).toHaveAttribute("type", "password");
});
```

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

Expected: pass after the form contract is explicit. If it already passes, keep the test as coverage for the documented contract.

- [ ] **Step 2: Freeze the timeout contract for vNext**

Use the current implementation as the vNext source of truth:

```text
Default vNext hook timeout: 150 ms for every hook.
Default vNext failure policy: fail-open unless the hook manifest explicitly sets fail-closed.
```

Update docs that currently list per-hook defaults so they no longer imply unimplemented behavior.

- [ ] **Step 3: Correct config schema docs**

Update docs to say:

- `enum` is supported as a keyword on scalar fields, not as `type: "enum"`.
- `password` renders as a password input in the GUI.
- vNext does not provide host-managed secret storage for community plugin config; do not claim saved secret values are absent from backend detail payloads unless the implementation is changed in this unit.

- [ ] **Step 4: Add docs checker assertions**

In `scripts/check-plugin-system-docs.mjs`, assert:

```text
Default vNext hook timeout: 150 ms
enum is supported as a keyword
vNext does not provide host-managed secret storage
```

- [ ] **Step 5: Run Unit 12 gate**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
pnpm check:plugin-system-docs
pnpm check:precommit:src
```

Expected: all commands exit 0.

---

## Unit 13: Hot-Path Performance Guardrails

**Goal:** Add focused performance guardrails for the gateway plugin hot path without overbuilding a benchmarking framework.

**Why:** The gateway path is latency-sensitive. The current architecture already has snapshots and caches; vNext should lock those wins and prevent easy regressions.

**Files:**
- Modify: `src-tauri/src/gateway/plugins/pipeline.rs`
- Modify: `src-tauri/src/gateway/streams/plugin_chunk.rs`
- Modify: `src-tauri/src/app/plugins/rule_runtime.rs`
- Test: `src-tauri/src/gateway/plugins/pipeline.rs`
- Test: `src-tauri/src/gateway/streams/plugin_chunk.rs`
- Test: `src-tauri/src/app/plugins/rule_runtime.rs`

- [x] **Step 1: Add test for stream fast path when no chunk plugins are active**

Expose this small method on `GatewayPluginPipeline`:

```rust
pub(crate) fn has_plugins_for_hook(&self, hook_name: GatewayPluginHookName) -> bool
```

Add a test in `src-tauri/src/gateway/plugins/pipeline.rs`:

```rust
assert!(!pipeline.has_plugins_for_hook(GatewayPluginHookName::ResponseChunk));
```

Run:

```bash
cd src-tauri && cargo test has_plugins_for_hook --lib
```

- [x] **Step 2: Use the fast path at the stream integration point**

In stream construction code, only wrap the upstream stream with `PluginChunkStream` when `has_plugins_for_hook(ResponseChunk)` returns true. Keep behavior unchanged when chunk plugins are enabled.

- [x] **Step 3: Add rule-runtime target batching regression test**

Add a test fixture with two replace rules targeting the same JSON request body path. Instrument the test-only path with a counter or expose a helper so the test proves the JSON body is parsed once per target field instead of once per rule.

Expected assertion:

```rust
assert_eq!(parse_count, 1);
```

Run:

```bash
cd src-tauri && cargo test rule_runtime_batches_json_target_rewrites --lib
```

Expected before implementation: fail because the current runtime parses JSON once per matching rule.

- [x] **Step 4: Implement same-target JSON rewrite batching**

In `src-tauri/src/app/plugins/rule_runtime.rs`, batch consecutive rules that share the same hook and target field/path for replace actions. Parse the JSON text once, apply all matching regex replacements to the selected string nodes, then serialize once.

Keep non-JSON targets and non-replace actions on the existing simple path.

- [x] **Step 5: Add circuit recovery regression test**

Keep or add a test that:

1. opens a plugin circuit after repeated failures;
2. verifies the plugin is skipped during cooldown;
3. advances time or uses a short test cooldown;
4. verifies one half-open attempt is allowed;
5. verifies success closes the circuit.

Run:

```bash
cd src-tauri && cargo test circuit --lib
```

- [x] **Step 6: Run Unit 13 gate**

Run:

```bash
cd src-tauri && cargo test gateway_plugin_pipeline_ --lib
cd src-tauri && cargo test plugin_chunk --lib
cd src-tauri && cargo test rule_runtime --lib
cd src-tauri && cargo fmt -- --check
```

Expected: all commands exit 0, with no global lock held while plugin runtime code executes.

---

## Unit 14: Developer Golden Path ABI Alignment

**Goal:** Make the default `create-aio-plugin` declarative-rule scaffold and replay output match the active Host/plugin SDK ABI.

**Why:** The official developer golden path must be boringly reliable. A plugin author who runs `create-aio-plugin acme.redactor rule` should get a rule file that the Rust host accepts, and local replay should return the same active mutation envelope (`requestBody`, `responseBody`, `streamChunk`, `logMessage`, `headers`) used by the SDK and gateway. Legacy `contextPatch` must not be emitted by vNext tooling.

**Files:**
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
- Modify: `packages/create-aio-plugin/src/devtools.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.test.ts`
- Modify: `docs/superpowers/plans/2026-06-12-plugin-system-vnext.md`
- Test: `packages/create-aio-plugin/src/scaffold.test.ts`

- [x] **Step 1: Add RED tests for Host rule ABI and replay result ABI**

In `packages/create-aio-plugin/src/scaffold.test.ts`, add tests that prove:

```typescript
const document = JSON.parse(files["rules/main.json"] ?? "{}");
expect(document.rules[0].target).toEqual({
  field: "request.body",
  jsonPath: "$.messages[*].content",
});
expect(document.rules[0].match).toMatchObject({
  regex: "SECRET_[A-Za-z0-9_]+",
  caseSensitive: true,
});
expect(document.rules[0]).not.toHaveProperty("matcher");
```

and:

```typescript
expect(result).toMatchObject({
  action: "replace",
  requestBody: expect.stringContaining("[REDACTED]"),
});
expect(result).not.toHaveProperty("contextPatch");
```

Run:

```bash
pnpm --filter create-aio-plugin test -- -t "host declarative rule ABI|active mutation envelope"
```

Expected before implementation: fail because the scaffold still emits `target.kind/path` plus `matcher`, and replay still emits `contextPatch`.

- [x] **Step 2: Update the rule scaffold to Host ABI**

Change the default rule file in `packages/create-aio-plugin/src/scaffold.ts` to:

```json
{
  "target": {
    "field": "request.body",
    "jsonPath": "$.messages[*].content"
  },
  "match": {
    "regex": "SECRET_[A-Za-z0-9_]+",
    "caseSensitive": true
  },
  "action": {
    "kind": "replace",
    "replacement": "[REDACTED]"
  }
}
```

- [x] **Step 3: Update local replay to emit vNext mutation fields**

Change `replayDeclarativeRule` in `packages/create-aio-plugin/src/devtools.ts` so request-body replacements return:

```typescript
{
  action: "replace",
  requestBody: nextBody,
}
```

Keep the existing support for reading old `matcher` and `target.kind/path` for developer convenience, but make newly generated output use the active vNext envelope only.

- [x] **Step 4: Run Unit 14 focused gate**

Run:

```bash
pnpm --filter create-aio-plugin test -- -t "host declarative rule ABI|active mutation envelope|replay command applies scaffold rule"
```

Expected: all focused tests pass.

- [x] **Step 5: Run Unit 14 package gate**

Run:

```bash
pnpm --filter create-aio-plugin test
pnpm plugin-sdk:typecheck
git diff --check -- packages/create-aio-plugin/src/scaffold.ts packages/create-aio-plugin/src/devtools.ts packages/create-aio-plugin/src/scaffold.test.ts docs/superpowers/plans/2026-06-12-plugin-system-vnext.md
```

Expected: all commands exit 0.

---

## Unit 15: SDK Permission Risk Truth

**Goal:** Align `@aio-coding-hub/plugin-sdk` permission risk values with the Rust Host and public manifest/permissions docs.

**Why:** The SDK is the developer-facing truth source for scaffolds and marketplace tooling. If `permissionRisk()` disagrees with the Host, authors and UI surfaces will show different risk levels for the same plugin.

**Files:**
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `packages/plugin-sdk/src/index.test.ts`
- Modify: `docs/superpowers/plans/2026-06-12-plugin-system-vnext.md`
- Test: `packages/plugin-sdk/src/index.test.ts`

- [x] **Step 1: Add RED test for Host risk table parity**

Add a test that verifies:

```typescript
expect(permissionRisk("response.header.read")).toBe("low");
expect(permissionRisk("response.header.write")).toBe("medium");
expect(permissionRisk("file.read")).toBe("high");
expect(permissionRisk("file.write")).toBe("high");
expect(permissionRisk("secret.read")).toBe("critical");
```

Run:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk test -- -t permissionRisk
```

Expected before implementation: fail because the SDK still reports response header and file permissions with different risk levels from the Host.

- [x] **Step 2: Update SDK risk table**

Update `PERMISSION_RISKS` in `packages/plugin-sdk/src/index.ts` to match `permission_risk()` in `src-tauri/src/domain/plugins.rs`.

- [x] **Step 3: Run Unit 15 gate**

Run:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk test -- -t permissionRisk
pnpm plugin-sdk:typecheck
pnpm check:plugin-api-contract
```

Expected: all commands exit 0.

## Self-Review Checklist

- [ ] Every active public hook listed in docs has a real gateway/log call site.
- [ ] Every reserved hook is rejected by backend and SDK validators.
- [ ] Every active permission has a runtime effect or context/mutation enforcement.
- [ ] Every reserved permission is rejected by backend and SDK validators.
- [ ] SDK hook-result ABI and host `GatewayHookResult` use the same active mutation envelope.
- [x] The Plugins UI exposes a clear pending-permission approval action.
- [ ] WASM behavior is explicit: policy-gated, never silently routed through the rule runtime.
- [x] WASM packaging is binary-safe for `plugin.wasm` artifacts.
- [ ] Config schema docs match the form/backend behavior for `enum` and `password`.
- [ ] Hook timeout/failure-policy docs match actual host defaults.
- [ ] Official Privacy Filter install path works without `tests/fixtures`.
- [x] Runtime cache locks are not held during hook execution.
- [x] Plugin circuit opens have a cooldown/half-open recovery path.
- [x] Stream chunk hooks have a no-plugin fast path.
- [ ] `create-aio-plugin` commands operate on real plugin directories.
- [x] `create-aio-plugin` default rule scaffold matches the Host declarative rule ABI.
- [x] `create-aio-plugin replay` emits active vNext mutation fields and never emits legacy `contextPatch`.
- [x] SDK `permissionRisk()` matches the Host permission risk table for active and reserved v1 permissions.
- [ ] Docs and SDK agree with generated bindings.

## Execution Notes

- Execute units in order. Do not start Unit 2 until Unit 1 tests pass.
- Commit after each unit with a scoped message such as `fix(plugin): align vnext public contract`.
- Keep unrelated dirty files untouched.
- Prefer English `tracing::info!`, `tracing::warn!`, and error messages in new code.
- Keep frontend UI changes restrained and operational; this plan mostly targets backend/API/tooling truthfulness.
