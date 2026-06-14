# PR296 Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the PR #296 review findings so the plugin-system PR is CI-green, truthful in docs, safer in the Plugins page, and more useful as a general community plugin foundation.

**Architecture:** Keep the current plugin-platform architecture intact: manifest-driven hooks, permission-trimmed gateway context, schema-driven config UI, and official `native:privacyFilter` as the only host-native engine. This plan makes surgical fixes around the five reviewed gaps without expanding runtime scope or adding speculative plugin APIs. Each task is independently testable and should be committed separately.

**Tech Stack:** Rust/Tauri/Tokio/Axum/SQLite for host and gateway behavior; React/TypeScript/Vitest/Testing Library for the Plugins page; Markdown docs under `docs/plugins`; TypeScript SDK tests under `packages/plugin-sdk`.

---

## Assumptions

- Current branch is `codex/plugin-system-completion`.
- PR #296 base is `origin/main`.
- Review findings to address are:
  - PR merge-ref `rust` CI fails in `process_runtime` tests.
  - official Privacy Filter docs omit `gateway.request.beforeSend`.
  - generic config form allows invalid JSON/number coercion to reach backend too late.
  - Plugins page allows status/update/uninstall actions while config save is pending.
  - manifest/SDK validation does not catch hook-permission scope mismatches early enough.
- Third-party `native` plugins remain unsupported.
- WASM execution policy remains unchanged.
- No new marketplace UI or remote install UI is added in this plan.

## Execution Preconditions

- Start execution from branch `codex/plugin-system-completion`.
- Run `git status --short --branch` before Task 1. If unrelated user changes exist, leave them untouched and stage only files listed in each task.
- Follow RED/GREEN discipline for every code task: add or adjust the focused test first, run it to observe the expected failure, implement the smallest fix, rerun the focused test, then commit only that task's files.
- Do not broaden plugin runtime capabilities in this plan. `native` remains official-only, and WASM remains policy-gated exactly as it is now.

## File Responsibility Map

- `src-tauri/src/app/plugins/process_runtime.rs`: process runtime PoC and its CI-sensitive tests.
- `docs/plugins/official-examples.md`: official Privacy Filter example contract and user-facing limitations.
- `docs/plugins/hooks.md`: public hook timing and upstream/gateway boundary language.
- `src/pages/plugins/pluginConfigValidation.ts`: frontend schema coercion and validation helpers.
- `src/pages/plugins/PluginConfigSchemaForm.tsx`: schema-driven config editing, field errors, submit gating.
- `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`: config form behavior tests.
- `src/pages/PluginsPage.tsx`: plugin page mutation busy state and detail actions.
- `src/pages/__tests__/PluginsPage.test.tsx`: page-level mutation locking tests.
- `src-tauri/src/domain/plugins.rs`: host manifest validation for hook-permission compatibility.
- `packages/plugin-sdk/src/index.ts`: SDK manifest validation mirror.
- `packages/plugin-sdk/src/index.test.ts`: SDK validation regression tests.

## Global Acceptance Criteria

- PR merge-ref `rust` check passes after pushing.
- `pnpm check:plugin-system-docs` passes.
- `pnpm --filter @aio-coding-hub/plugin-sdk test` passes.
- `pnpm test:unit -- src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx src/pages/__tests__/PluginsPage.test.tsx packages/plugin-sdk/src/index.test.ts` passes.
- `cd src-tauri && cargo test --locked plugin_process_runtime_poc_ --lib -- --nocapture` passes on a clean local run.
- `cd src-tauri && cargo test --locked domain::plugins::tests:: --lib -- --nocapture` passes.
- `pnpm check:prepush` passes before final push.

---

## Task 1: Stabilize Process Runtime CI Tests

**Goal:** Fix the PR merge-ref Rust CI failure without changing production process runtime semantics.

**Files:**
- Modify: `src-tauri/src/app/plugins/process_runtime.rs`

- [ ] **Step 1: Write the failing/diagnostic test change**

Replace the test-only timeout helper in `src-tauri/src/app/plugins/process_runtime.rs` so CI startup has realistic slack while timeout-specific tests still use a deliberately tiny timeout.

```rust
    fn node_config(script_path: &std::path::Path) -> ProcessRuntimeConfig {
        ProcessRuntimeConfig {
            program: "node".to_string(),
            args: vec![script_path.display().to_string()],
            start_timeout: Duration::from_secs(5),
            hook_timeout: Duration::from_secs(5),
            idle_recycle: Duration::from_millis(50),
            max_line_bytes: 256 * 1024,
        }
    }
```

Then keep `plugin_process_runtime_poc_reports_start_timeout` explicitly overriding `config.start_timeout = Duration::from_millis(50);`.

- [ ] **Step 2: Run focused process runtime tests**

Run:

```bash
cd src-tauri && cargo test --locked plugin_process_runtime_poc_ --lib -- --nocapture
```

Expected: all `plugin_process_runtime_poc_*` tests pass locally.

- [ ] **Step 3: Add timeout diagnostics without changing runtime behavior**

Modify the timeout branch in `JsonRpcProcessRuntime::start` to preserve the same error code but include program and timeout in the English message.

```rust
let ready = tokio::time::timeout(runtime.config.start_timeout, runtime.read_json_line())
    .await
    .map_err(|_| {
        AppError::new(
            "PLUGIN_PROCESS_START_TIMEOUT",
            format!(
                "process plugin did not send ready message before start timeout: program={}, timeout_ms={}",
                runtime.config.program,
                runtime.config.start_timeout.as_millis()
            ),
        )
    });
```

Do not read or log plugin stdout payloads beyond the protocol line. Do not change production default timeout in this task.

- [ ] **Step 4: Run focused tests again**

Run:

```bash
cd src-tauri && cargo test --locked plugin_process_runtime_poc_ --lib -- --nocapture
```

Expected: all focused tests pass. The intentionally slow script still returns `PLUGIN_PROCESS_START_TIMEOUT` in `plugin_process_runtime_poc_reports_start_timeout`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/app/plugins/process_runtime.rs
git commit -m "test: stabilize process runtime ci timeouts"
```

---

## Task 2: Fix Privacy Filter Official Example Docs

**Goal:** Make official docs match the actual manifest and clarify the gateway boundary that confused users on another machine.

**Files:**
- Modify: `docs/plugins/official-examples.md`
- Modify: `docs/plugins/hooks.md`

- [ ] **Step 1: Update official hooks list**

In `docs/plugins/official-examples.md`, replace the Privacy Filter hook list with:

```markdown
Hooks:

- `gateway.request.afterBodyRead`
- `gateway.request.beforeSend`
- `log.beforePersist`
```

- [ ] **Step 2: Add boundary language to official example**

In the same file, under “Provider request shapes”, add this paragraph:

```markdown
Gateway boundary note: Privacy Filter receives the original client-to-gateway body because the gateway must inspect the prompt before it can redact it. The protection guarantee is that the gateway-to-upstream provider request body and persisted request logs are redacted when the plugin is enabled and the matching strategy is selected. If you inspect the local client request before the gateway hook runs, you may still see the original input.
```

- [ ] **Step 3: Update hook timing docs**

In `docs/plugins/hooks.md`, update the existing `## gateway.request.beforeSend` section so its prose says:

```markdown
Runs after provider selection, auth/header preparation, request body sanitizers, and protocol rectifiers for the current attempt, immediately before the gateway sends bytes to the upstream provider. Use this hook when the plugin must guarantee final upstream request-body or request-header mutation.

This hook sees semantic decoded request body content. If a plugin mutates the body, the gateway updates the final upstream body and removes or recalculates wire-level length/encoding semantics as needed. Unchanged requests keep the original passthrough body where possible.
```

- [ ] **Step 4: Run docs check**

Run:

```bash
pnpm check:plugin-system-docs
```

Expected: documentation checker exits 0.

- [ ] **Step 5: Commit**

```bash
git add docs/plugins/official-examples.md docs/plugins/hooks.md
git commit -m "docs: clarify privacy filter upstream boundary"
```

---

## Task 3: Add Field-Level Config Form Validation

**Goal:** Prevent invalid JSON/object/array and accidental numeric `0` coercion from being submitted through the generic schema renderer.

**Files:**
- Modify: `src/pages/plugins/pluginConfigValidation.ts`
- Modify: `src/pages/plugins/PluginConfigSchemaForm.tsx`
- Modify: `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`

- [ ] **Step 1: Add RED tests for invalid JSON and blank numbers**

Append these tests to `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`:

```tsx
it("blocks submit and shows a field error for invalid json fields", () => {
  const onSubmit = vi.fn();

  render(
    <PluginConfigSchemaForm
      identity="publisher.advanced:1"
      schema={{
        type: "object",
        properties: {
          advanced: { type: "object", title: "高级配置" },
        },
      }}
      value={{ advanced: { retries: 2 } }}
      pending={false}
      onSubmit={onSubmit}
    />
  );

  fireEvent.change(screen.getByLabelText("高级配置"), {
    target: { value: "{\"retries\":" },
  });
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

  expect(onSubmit).not.toHaveBeenCalled();
  expect(screen.getByText("请输入合法的 JSON 对象。")).toBeInTheDocument();
});

it("keeps blank optional numbers unset instead of coercing them to zero", () => {
  const onSubmit = vi.fn();

  render(
    <PluginConfigSchemaForm
      identity="publisher.number:1"
      schema={{
        type: "object",
        properties: {
          threshold: { type: "integer", title: "阈值" },
          enabled: { type: "boolean", default: true },
        },
      }}
      value={{ threshold: 3 }}
      pending={false}
      onSubmit={onSubmit}
    />
  );

  fireEvent.change(screen.getByLabelText("阈值"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

  expect(onSubmit).toHaveBeenCalledWith({ enabled: true });
});
```

- [ ] **Step 2: Run tests and confirm RED**

Run:

```bash
pnpm test:unit -- src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

Expected: the invalid JSON test fails because the current form submits; the blank number test fails because current code submits `threshold: 0`.

- [ ] **Step 3: Replace coercion helper with parse result**

In `src/pages/plugins/pluginConfigValidation.ts`, add:

```ts
export type ConfigFieldParseResult =
  | { ok: true; value: JsonValue | undefined }
  | { ok: false; error: string };

export function parseConfigField(raw: string, type: string | null): ConfigFieldParseResult {
  if (type === "integer" || type === "number") {
    if (raw.trim() === "") return { ok: true, value: undefined };
    const parsed = Number(raw);
    if (!Number.isFinite(parsed) || (type === "integer" && !Number.isInteger(parsed))) {
      return { ok: false, error: "请输入有效数字。" };
    }
    return { ok: true, value: parsed };
  }

  if (type === "array" || type === "object") {
    if (raw.trim() === "") return { ok: true, value: undefined };
    try {
      const parsed = JSON.parse(raw) as JsonValue;
      if (type === "array" && !Array.isArray(parsed)) {
        return { ok: false, error: "请输入合法的 JSON 数组。" };
      }
      if (type === "object" && !isRecord(parsed)) {
        return { ok: false, error: "请输入合法的 JSON 对象。" };
      }
      return { ok: true, value: parsed };
    } catch {
      return {
        ok: false,
        error: type === "array" ? "请输入合法的 JSON 数组。" : "请输入合法的 JSON 对象。",
      };
    }
  }

  return { ok: true, value: raw };
}
```

Keep `coerceConfigField` only if existing tests or callers still need it; otherwise remove it after replacing imports.

- [ ] **Step 4: Track field errors in the form**

In `src/pages/plugins/PluginConfigSchemaForm.tsx`, replace `coerceConfigField` import with `parseConfigField`. Add state:

```tsx
const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
```

Reset errors when identity changes:

```tsx
useEffect(() => {
  setDraft(initialObject(valueRef.current));
  setFieldErrors({});
}, [identity]);
```

Add helper:

```tsx
function setParsedField(key: string, raw: string, type: string | null) {
  const parsed = parseConfigField(raw, type);
  if (!parsed.ok) {
    setFieldErrors((current) => ({ ...current, [key]: parsed.error }));
    return;
  }
  setFieldErrors((current) => {
    const next = { ...current };
    delete next[key];
    return next;
  });
  setDraft((current) => {
    const next = { ...current };
    if (parsed.value === undefined) {
      delete next[key];
    } else {
      next[key] = parsed.value;
    }
    return next;
  });
}
```

Use `setParsedField(field.key, event.target.value, field.type)` for text, textarea, json, and number inputs.

- [ ] **Step 5: Render field errors and block submit**

Add a small helper in `PluginConfigSchemaForm` so every branch renders errors consistently:

```tsx
function renderFieldError(field: PluginConfigFieldModel) {
  return fieldErrors[field.key] ? (
    <span className="text-xs text-destructive">{fieldErrors[field.key]}</span>
  ) : null;
}
```

Then render `{renderFieldError(field)}` after warnings in the `select`, `textarea`, `json`, and default input branches, and after the warning block in the `checkboxGroup` branch if future parser-driven errors are introduced there.

```tsx
{renderFieldError(field)}
```

At submit:

```tsx
if (Object.keys(fieldErrors).length > 0) return;
onSubmit(buildSubmitValue());
```

Disable the submit button when there are field errors:

```tsx
<Button type="submit" disabled={pending || Object.keys(fieldErrors).length > 0}>
  保存配置
</Button>
```

- [ ] **Step 6: Run focused tests**

Run:

```bash
pnpm test:unit -- src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

Expected: all config form tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/pages/plugins/pluginConfigValidation.ts src/pages/plugins/PluginConfigSchemaForm.tsx src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
git commit -m "fix: validate plugin config fields before submit"
```

---

## Task 4: Lock Plugin Detail Actions During Config Save

**Goal:** Prevent status/update/uninstall/permission actions from running while the selected plugin config save is pending.

**Files:**
- Modify: `src/pages/PluginsPage.tsx`
- Modify: `src/pages/__tests__/PluginsPage.test.tsx`

- [ ] **Step 1: Add RED page test**

Add this test to `src/pages/__tests__/PluginsPage.test.tsx`:

```tsx
it("disables plugin actions while config save is pending", () => {
  vi.mocked(usePluginsListQuery).mockReturnValue({
    data: [summary({ status: "disabled", update_available: true })],
    isLoading: false,
    isFetching: false,
    error: null,
  } as any);
  vi.mocked(usePluginSaveConfigMutation).mockReturnValue(
    mutation({ isPending: true }) as any
  );

  renderWithProviders(<PluginsPage />);

  expect(screen.getByRole("button", { name: /启用/ })).toBeDisabled();
  expect(screen.getByRole("button", { name: /卸载/ })).toBeDisabled();
  expect(screen.getByRole("button", { name: /授权待审批权限/ })).toBeDisabled();
});
```

Also update the test imports and query mock in `src/pages/__tests__/PluginsPage.test.tsx`:

```tsx
import {
  usePluginDisableMutation,
  usePluginEnableMutation,
  usePluginGrantPermissionsMutation,
  usePluginInstallFromFileMutation,
  usePluginInstallOfficialMutation,
  usePluginQuery,
  usePluginRollbackMutation,
  usePluginSaveConfigMutation,
  usePluginUpdateFromFileMutation,
  usePluginsListQuery,
  usePluginUninstallMutation,
} from "../../query/plugins";
```

```tsx
vi.mock("../../query/plugins", async () => {
  const actual = await vi.importActual<typeof import("../../query/plugins")>("../../query/plugins");
  return {
    ...actual,
    usePluginsListQuery: vi.fn(),
    usePluginQuery: vi.fn(),
    usePluginInstallFromFileMutation: vi.fn(),
    usePluginInstallOfficialMutation: vi.fn(),
    usePluginUpdateFromFileMutation: vi.fn(),
    usePluginRollbackMutation: vi.fn(),
    usePluginEnableMutation: vi.fn(),
    usePluginGrantPermissionsMutation: vi.fn(),
    usePluginDisableMutation: vi.fn(),
    usePluginUninstallMutation: vi.fn(),
    usePluginSaveConfigMutation: vi.fn(),
  };
});
```

And add the default mock in `beforeEach`:

```tsx
vi.mocked(usePluginSaveConfigMutation).mockReturnValue(mutation() as any);
```

- [ ] **Step 2: Run test and confirm RED**

Run:

```bash
pnpm test:unit -- src/pages/__tests__/PluginsPage.test.tsx
```

Expected: new test fails because list/detail actions are not all disabled by config-save pending state.

- [ ] **Step 3: Include config save in the busy state**

In `src/pages/PluginsPage.tsx`, update:

```tsx
const busy =
  installMutation.isPending ||
  installOfficialMutation.isPending ||
  updateMutation.isPending ||
  rollbackMutation.isPending ||
  enableMutation.isPending ||
  grantPermissionsMutation.isPending ||
  disableMutation.isPending ||
  uninstallMutation.isPending ||
  saveConfigMutation.isPending;
```

Keep `savingConfig={saveConfigMutation.isPending}` for the save button spinner/disable semantics.

- [ ] **Step 4: Run focused page tests**

Run:

```bash
pnpm test:unit -- src/pages/__tests__/PluginsPage.test.tsx
```

Expected: all PluginsPage tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/pages/PluginsPage.tsx src/pages/__tests__/PluginsPage.test.tsx
git commit -m "fix: lock plugin actions during config save"
```

---

## Task 5: Validate Hook-Permission Scope In Host And SDK

**Goal:** Give plugin authors early, deterministic feedback when permissions do not make sense for declared hooks.

**Files:**
- Modify: `src-tauri/src/domain/plugins.rs`
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `packages/plugin-sdk/src/index.test.ts`

- [ ] **Step 1: Add RED host tests**

Append to `src-tauri/src/domain/plugins.rs` tests:

```rust
#[test]
fn manifest_rejects_permissions_that_do_not_apply_to_declared_hooks() {
    let mut raw = valid_manifest();
    raw["hooks"] = serde_json::json!([
        { "name": "log.beforePersist", "priority": 10, "failurePolicy": "fail-open" }
    ]);
    raw["permissions"] = serde_json::json!(["request.body.read", "log.redact"]);
    let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
    let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
    assert_eq!(err.code, "PLUGIN_PERMISSION_SCOPE_MISMATCH");
    assert!(err.message.contains("request.body.read"));
}
```

- [ ] **Step 2: Run host test and confirm RED**

Run:

```bash
cd src-tauri && cargo test --locked manifest_rejects_permissions_that_do_not_apply_to_declared_hooks --lib -- --nocapture
```

Expected: test fails because host currently accepts extra active permissions.

- [ ] **Step 3: Implement host scope validation**

In `src-tauri/src/domain/plugins.rs`, add:

```rust
fn hook_allows_permission(hook_name: &str, permission: &str) -> bool {
    match permission {
        "request.meta.read"
        | "request.header.read"
        | "request.header.readSensitive"
        | "request.header.write"
        | "request.body.read"
        | "request.body.write" => matches!(
            hook_name,
            "gateway.request.afterBodyRead" | "gateway.request.beforeSend"
        ),
        "response.header.read" | "response.header.write" => matches!(
            hook_name,
            "gateway.response.after" | "gateway.error"
        ),
        "response.body.read" | "response.body.write" => matches!(
            hook_name,
            "gateway.response.after" | "gateway.error"
        ),
        "stream.inspect" | "stream.modify" => hook_name == "gateway.response.chunk",
        "log.redact" => hook_name == "log.beforePersist",
        _ => false,
    }
}

fn validate_permission_scope(
    hooks: &[PluginHook],
    permissions: &[String],
) -> Result<(), PluginValidationError> {
    for permission in permissions {
        if is_reserved_permission(permission) {
            continue;
        }
        let allowed = hooks
            .iter()
            .any(|hook| hook_allows_permission(&hook.name, permission));
        if !allowed {
            return Err(PluginValidationError::new(
                "PLUGIN_PERMISSION_SCOPE_MISMATCH",
                format!("permission {permission} does not apply to any declared hook"),
            ));
        }
    }
    Ok(())
}
```

Call it from `validate_manifest` after `validate_hook_permissions`.

```rust
validate_permission_scope(&manifest.hooks, &manifest.permissions)?;
```

- [ ] **Step 4: Add SDK RED test**

Append to `packages/plugin-sdk/src/index.test.ts`:

```ts
it("rejects permissions that do not apply to declared hooks", () => {
  const scopedManifest = {
    ...manifest,
    hooks: [{ name: "log.beforePersist" as const, priority: 10 }],
    permissions: ["request.body.read", "log.redact"] as const,
  };

  expect(validateManifest(scopedManifest as never)).toEqual({
    ok: false,
    error: {
      code: "PLUGIN_PERMISSION_SCOPE_MISMATCH",
      message: "permission request.body.read does not apply to any declared hook",
    },
  });
});
```

- [ ] **Step 5: Implement SDK scope validation**

In `packages/plugin-sdk/src/index.ts`, add:

```ts
function hookAllowsPermission(hookName: GatewayHookName, permission: PluginPermission): boolean {
  if (
    permission === "request.meta.read" ||
    permission === "request.header.read" ||
    permission === "request.header.readSensitive" ||
    permission === "request.header.write" ||
    permission === "request.body.read" ||
    permission === "request.body.write"
  ) {
    return hookName === "gateway.request.afterBodyRead" || hookName === "gateway.request.beforeSend";
  }
  if (
    permission === "response.header.read" ||
    permission === "response.header.write" ||
    permission === "response.body.read" ||
    permission === "response.body.write"
  ) {
    return hookName === "gateway.response.after" || hookName === "gateway.error";
  }
  if (permission === "stream.inspect" || permission === "stream.modify") {
    return hookName === "gateway.response.chunk";
  }
  if (permission === "log.redact") return hookName === "log.beforePersist";
  return false;
}

function validatePermissionScope(manifest: PluginManifest): ValidationResult | null {
  for (const permission of manifest.permissions) {
    if (RESERVED_PERMISSIONS.has(permission)) continue;
    if (!manifest.hooks.some((hook) => hookAllowsPermission(hook.name, permission))) {
      return invalid(
        "PLUGIN_PERMISSION_SCOPE_MISMATCH",
        `permission ${permission} does not apply to any declared hook`
      );
    }
  }
  return null;
}
```

Call it after `validatePermissionSet`:

```ts
const permissionScopeError = validatePermissionScope(manifest);
if (permissionScopeError) return permissionScopeError;
```

- [ ] **Step 6: Run host and SDK tests**

Run:

```bash
cd src-tauri && cargo test --locked domain::plugins::tests:: --lib -- --nocapture
pnpm --filter @aio-coding-hub/plugin-sdk test
```

Expected: both pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/domain/plugins.rs packages/plugin-sdk/src/index.ts packages/plugin-sdk/src/index.test.ts
git commit -m "fix: validate plugin permission scope"
```

---

## Task 6: Final Verification And Push

**Goal:** Prove all review fixes work together and update PR #296.

**Files:**
- No planned source edits.

- [ ] **Step 1: Run focused verification**

Run:

```bash
cd src-tauri && cargo test --locked plugin_process_runtime_poc_ --lib -- --nocapture
cd src-tauri && cargo test --locked domain::plugins::tests:: --lib -- --nocapture
pnpm test:unit -- src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx src/pages/__tests__/PluginsPage.test.tsx packages/plugin-sdk/src/index.test.ts
pnpm check:plugin-system-docs
```

Expected: every command exits 0.

- [ ] **Step 2: Run full pre-push verification**

Run:

```bash
pnpm check:prepush
```

Expected: frontend shards, generated bindings, Tauri tests, and clippy pass.

- [ ] **Step 3: Check git state**

Run:

```bash
git status --short --branch
git log --oneline -6
```

Expected: branch is ahead of origin by the task commits before push, with no uncommitted files.

- [ ] **Step 4: Push**

Run:

```bash
git push
```

Expected: branch `codex/plugin-system-completion` pushes successfully.

- [ ] **Step 5: Verify PR checks**

Run:

```bash
gh pr view 296 --json statusCheckRollup,mergeStateStatus --jq '{mergeStateStatus, checks:[.statusCheckRollup[]? | {name:.name, status:.status, conclusion:.conclusion}]}'
```

Expected: `rust` no longer reports failure for the latest PR head/merge ref. If checks are still running, wait until completion before claiming the PR is ready.

---

## Rollback Plan

- If Task 1 still fails CI, keep the increased test timeout commit and add a follow-up change to skip Node-based process runtime PoC tests when `node` is missing or fails a preflight command; do not skip tests solely because they are slow.
- If Task 3 causes frontend regressions, revert only the config-form commit and keep the backend/domain/docs fixes.
- If Task 5 rejects existing official Privacy Filter manifest, adjust `hook_allows_permission` to include `log.beforePersist` for `log.redact` and request hooks for request permissions, then rerun host and SDK tests.

## Self-Review

- Spec coverage: the plan covers all five review findings: CI, docs, config UI, page busy state, and permission scope validation.
- Placeholder scan: no task uses unfinished placeholder language.
- Type consistency: `parseConfigField`, `ConfigFieldParseResult`, `PLUGIN_PERMISSION_SCOPE_MISMATCH`, and hook names are defined before use and match existing file naming style.
- Boundary check: this plan does not add arbitrary native plugins, WASM enablement, marketplace UI, or new plugin capabilities outside the review findings.
