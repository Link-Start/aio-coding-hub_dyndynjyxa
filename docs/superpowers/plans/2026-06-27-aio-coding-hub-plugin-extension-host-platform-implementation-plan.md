# aio-coding-hub Plugin Extension Host Platform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an Extension Host first plugin platform that lets TypeScript plugins safely extend UI slots, providers, protocol bridges, commands, diagnostics, storage, and gateway contributions in the Tauri desktop app.

**Architecture:** Keep the Rust/Tauri host as the owner of transport, credentials, SQLite, WebView, routing, stream framing, lifecycle, and rendering. Plugins are TypeScript projects compiled to JavaScript and executed in a host-managed worker process launched from the same AIO executable with `--extension-host-worker`; the worker communicates with the app process through bounded JSON-RPC and never runs inside the Tauri WebView. Declarative gateway rules and the bundled privacy filter are retained as legacy contribution families so current behavior remains equivalent, but the public plugin direction becomes Extension Manifest v1 with declarative contributions plus host-mediated APIs.

**Tech Stack:** Rust, Tauri 2, SQLite via rusqlite/r2d2, Specta generated bindings, JSON-RPC over stdio, rquickjs or an equivalent embedded JS engine selected during implementation, TypeScript, React 19, TanStack Query, Vitest, Cargo tests, `@aio-coding-hub/plugin-sdk`, `create-aio-plugin`, Markdown docs.

---

## Scope Boundaries

- The app remains a Linux/macOS/Windows Tauri 2 desktop GUI, not an H5 plugin container.
- Third-party plugins do not inject React components, do not control the Tauri WebView, and do not open an in-app browser.
- UI plugins provide host-rendered schemas and command IDs; AIO React components render every contributed field, section, action, tab, panel, card, and badge.
- The plugin language is TypeScript. Plugin packages ship compiled JavaScript at `main`, plus `plugin.json`.
- The extension worker is host-owned. It is started, activated, deactivated, killed, and reaped by the Rust host.
- The worker mode must be checked before `ensure_webview2_or_exit()` in `src-tauri/src/main.rs`, so Windows extension workers do not require or initialize WebView2.
- Existing package install, preview, update, rollback, quarantine, official plugin, runtime report, and replay infrastructure should be reused instead of replaced wholesale.
- `official.privacy-filter` must keep equivalent behavior.
- `declarativeRules` must stop being documented as the ecosystem main path. It may remain as `contributes.gatewayRules` or as an explicit legacy loader for current tests and official behavior.
- Arbitrary native plugin loading is out of scope.
- Arbitrary file, network, secret, and system command APIs are out of scope unless represented as denied or reserved capability names.
- Enterprise marketplace social features, ratings, payment, and remote operator services are out of scope.

## File Structure

- Modify: `packages/plugin-sdk/src/index.ts`
  - Own the public Extension Manifest v1 TypeScript contract, contribution types, UI schema types, capability names, validation helpers, and extension API type declarations.
- Modify: `packages/plugin-sdk/src/index.test.ts`
  - Prove SDK validation accepts target Extension Host manifests and rejects unknown slots, missing `main`, undeclared command actions, and malformed provider/bridge declarations.
- Modify: `packages/plugin-sdk/src/index.typecheck.ts`
  - Prove plugin authors get typed `activate`, command handlers, UI schema, provider storage values, and bridge declarations.
- Modify: `docs/plugins/plugin-api-v1-contract.json`
  - Make the machine-readable contract describe Extension Host mainline runtimes, contributions, capabilities, active UI slots, and legacy gateway rules.
- Modify: `src-tauri/src/domain/plugins.rs`
  - Mirror SDK manifest/contribution/runtime types, validation, lifecycle summaries, install preview/update diff DTOs, active contribution DTOs, execution reports, and replay DTO changes.
- Create: `src-tauri/src/domain/plugin_contributions.rs`
  - Keep contribution normalization, slot constants, contribution summaries, contribution impact diffing, and registry input/output types out of the large plugin domain file.
- Modify: `src-tauri/src/domain/mod.rs`
  - Export `plugin_contributions`.
- Create: `src-tauri/src/app/plugins/contribution_registry.rs`
  - Build immutable snapshots of active contributions from enabled plugins and reject unknown or malformed contribution points.
- Modify: `src-tauri/src/app/plugins/runtime_lifecycle.rs`
  - Upgrade cache-only lifecycle retention into an extension instance lifecycle boundary while preserving `PluginRuntimeCache` support for legacy runtimes.
- Create: `src-tauri/src/app/plugins/extension_host.rs`
  - Manage extension worker processes, activation, deactivation, command dispatch, timeouts, crash handling, and worker disposal.
- Create: `src-tauri/src/app/plugins/extension_host_worker.rs`
  - Implement `--extension-host-worker` stdin/stdout JSON-RPC loop and JS module loading.
- Modify: `src-tauri/src/app/plugins/process_runtime.rs`
  - Extract generic JSON-RPC line client behavior from hook-specific `call_hook` so legacy process plugins and Extension Host runtime share timeout and byte-limit handling.
- Modify: `src-tauri/src/app/plugins/runtime_executor.rs`
  - Route legacy gateway hooks through contribution-aware runtime execution without changing privacy filter behavior.
- Modify: `src-tauri/src/app/plugins/runtime_manager.rs`
  - Register new extension host runtime managers and lifecycle retain/dispose integration.
- Modify: `src-tauri/src/app/plugins/mod.rs`
  - Export new plugin runtime modules.
- Modify: `src-tauri/src/app/plugin_service.rs`
  - Load active contribution snapshots, expose contribution queries, include contribution impact in package preview/update diff, and refresh contribution/runtime snapshots on plugin lifecycle changes.
- Modify: `src-tauri/src/commands/plugins.rs`
  - Add active contribution query, command execution, generalized runtime report query, and replay export input changes.
- Modify: `src-tauri/src/commands/registry.rs`
  - Register new Tauri commands and Specta types.
- Modify: `src-tauri/src/infra/plugins/package.rs`
  - Validate `main` path and compiled extension assets during package extraction.
- Modify: `src-tauri/src/infra/plugins/repository.rs`
  - Add provider extension value helpers and contribution snapshot support where persistence is needed.
- Modify: `src-tauri/src/infra/plugins/runtime_reports.rs`
  - Generalize hook execution reports into extension execution reports while keeping compatibility query support for current UI.
- Modify: `src-tauri/src/infra/plugins/replay_export.rs`
  - Export replay fixtures by contribution type and contribution ID, not only gateway hook name.
- Create: `src-tauri/src/infra/db/migrations/v34_to_v35.rs`
  - Add provider extension value storage and generalized extension execution report columns.
- Modify: `src-tauri/src/infra/db/migrations/mod.rs`
  - Add the v34→v35 migration and set `LATEST_SCHEMA_VERSION` to `35`.
- Modify: `src-tauri/src/infra/db/migrations/ensure.rs`
  - Ensure provider extension value storage and report columns exist on repaired/dev databases.
- Modify: `src-tauri/src/infra/db/migrations/baseline_v25.rs`
  - Keep fresh database schema equivalent after migrations.
- Modify: `src-tauri/src/infra/db/migrations/tests.rs`
  - Cover fresh install, idempotent ensure, provider extension values, report columns, and duplicate provider behavior.
- Modify: `src-tauri/src/domain/providers/types.rs`
  - Add provider extension value DTOs and extension values to `ProviderSummary`, `ProviderUpsertParams`, and `ProviderForGateway`.
- Modify: `src-tauri/src/domain/providers/queries.rs`
  - Persist, read, duplicate, and expose provider extension values without mixing plugin-owned namespaces into the fixed provider columns.
- Modify: `src-tauri/src/domain/providers/tests.rs`
  - Cover save/read/edit/duplicate/delete behavior for provider extension values.
- Modify: `src-tauri/src/app/provider_service.rs`
  - Accept provider extension values in `ProviderUpsertInput`, preserve unrelated namespaces, and clear route runtime state when extension values change.
- Modify: `src-tauri/src/commands/providers/crud.rs`
  - Export the changed provider upsert type through Specta.
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/registry.rs`
  - Combine host built-in bridges and plugin-declared bridge contributions.
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/traits.rs`
  - Add plugin bridge call DTOs that match JSON-RPC envelopes and preserve Rust ownership of HTTP transport.
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/bridge.rs`
  - Route bridge selection through contribution-aware lookup.
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/e2e_tests.rs`
  - Cover plugin-declared bridge type selection and stable failure reports.
- Modify: `src-tauri/src/gateway/plugins/pipeline.rs`
  - Treat gateway hooks/rules as contribution families and write generalized execution reports.
- Modify: `src-tauri/src/gateway/plugins/registry.rs`
  - Normalize legacy hook manifests into gateway contribution entries.
- Modify: `src-tauri/src/gateway/plugins/contract.rs`
  - Keep active hook contracts and expose them as legacy gateway contribution contracts.
- Create: `src/services/pluginContributions.ts`
  - Frontend IPC wrappers and validation helpers for active contributions and command execution.
- Modify: `src/services/plugins.ts`
  - Export new manifest/contribution/report/update diff types and command wrappers.
- Modify: `src/services/providers/providers.ts`
  - Add typed provider extension values to provider summaries and upsert payloads.
- Modify: `src/query/keys.ts`
  - Add contribution, command, and provider extension query keys.
- Modify: `src/query/plugins.ts`
  - Add active contribution queries, command execution mutations, and invalidation on plugin lifecycle changes.
- Create: `src/plugins/contributions/types.ts`
  - Frontend-only narrow types for page IDs, slot IDs, UI schema, action wiring, and renderer props.
- Create: `src/plugins/contributions/useActiveContributions.ts`
  - React query hook and selector helpers for filtering contributions by slot.
- Create: `src/plugins/contributions/HostRenderedContribution.tsx`
  - Render host-owned schemas into existing UI primitives.
- Create: `src/plugins/contributions/ContributionSlot.tsx`
  - Reusable slot component for pages to opt into plugin UI.
- Create: `src/plugins/contributions/__tests__/HostRenderedContribution.test.tsx`
  - Cover supported controls, ordering, invalid schema fallback, command action dispatch, disabled state, and no page crash.
- Modify: `src/pages/providers/ProviderEditorDialog.tsx`
  - Add `providers.editor.sections` and `providers.editor.fields` slots.
- Modify: `src/pages/providers/useProviderEditorForm.ts`
  - Load, edit, validate, and save host-rendered plugin field values.
- Modify: `src/pages/providers/providerEditorActionContext.ts`
  - Add extension values to the editor save context.
- Modify: `src/pages/providers/providerEditorSubmitModel.ts`
  - Include extension values in provider upsert payloads after schema validation.
- Modify: `src/pages/providers/__tests__/ProviderEditorDialog.test.tsx`
  - Cover plugin field display, save payload, disabled plugin hiding fields, and preserved stored values.
- Modify: `src/pages/settings/SettingsMainColumn.tsx`
  - Add `settings.sections` slot without changing existing settings behavior.
- Modify: `src/pages/settings/__tests__/SettingsMainColumn.test.tsx`
  - Cover contributed settings section rendering and command action dispatch.
- Modify: `src/components/home/RequestLogDetailDialog.tsx`
  - Add `logs.detail.tabs` and `logs.detail.actions` slots.
- Modify: `src/components/home/__tests__/RequestLogDetailDialog.test.tsx`
  - Cover contributed trace tab rendering and no crash on invalid schema.
- Modify: `src/pages/PluginsPage.tsx`
  - Show contribution categories, lifecycle status, and extension reports in plugin detail.
- Modify: `src/pages/plugins/PluginInstallPreviewDialog.tsx`
  - Show contribution impact for install preview.
- Modify: `src/pages/plugins/PluginUpdatePreviewDialog.tsx`
  - Show contribution diff for updates.
- Modify: `src/pages/plugins/PluginRuntimeReportsPanel.tsx`
  - Read generalized extension execution reports while keeping current hook report display.
- Modify: `src/pages/__tests__/PluginsPage.test.tsx`
  - Cover contribution impact and detail panels.
- Modify: `src/generated/bindings.ts`
  - Regenerate via `pnpm tauri:gen-types` after Rust Specta type changes.
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
  - Add `extension` template with TypeScript source, `plugin.json`, tests, `tsconfig.json`, and package scripts.
- Modify: `packages/create-aio-plugin/src/scaffold.test.ts`
  - Cover extension scaffolding and manifest validation.
- Modify: `packages/create-aio-plugin/src/devtools.ts`
  - Validate Extension Manifest v1, run contribution checks, and print contribution impact in `publish-check`.
- Modify: `packages/create-aio-plugin/src/cli.ts`
  - Route `create-aio-plugin <id> extension`, `validate`, and `publish-check` for extension plugins.
- Create: `docs/plugins/extension-host.md`
  - Document TypeScript plugin authoring, activation, contributions, UI schema, provider storage, bridge declarations, commands, reports, and package layout.
- Modify: `docs/plugin-manifest-v1.md`
  - Replace rule-first wording with Extension Manifest v1 and mark old runtime+hooks shape as legacy.
- Modify: `docs/plugins/README.md`
  - Point developers to Extension Host first docs.
- Modify: `docs/plugins/reference/manifest.md`
  - Mirror manifest changes.
- Modify: `docs/plugins/reference/sdk.md`
  - Document SDK extension types and scaffolder flow.
- Modify: `docs/plugins/reference/compatibility.md`
  - Document Extension Host compatibility, platform support, and legacy gateway compatibility.
- Modify: `docs/plugins/reference/declarative-rules.md`
  - Mark declarative rules as a gateway contribution family, not the recommended ecosystem path.
- Modify: `docs/plugins/runtime/README.md`
  - Describe Extension Host lifecycle and legacy runtime boundaries.

## Task 1: Extension Manifest v1 and SDK Contract

**Files:**
- Modify: `packages/plugin-sdk/src/index.ts`
- Modify: `packages/plugin-sdk/src/index.test.ts`
- Modify: `packages/plugin-sdk/src/index.typecheck.ts`
- Modify: `docs/plugins/plugin-api-v1-contract.json`
- Modify: `src-tauri/src/domain/plugins.rs`
- Create: `src-tauri/src/domain/plugin_contributions.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: Write failing SDK validation tests**

Add tests to `packages/plugin-sdk/src/index.test.ts`:

```ts
const openRouterManifest: PluginManifest = {
  id: "acme.openrouter",
  name: "OpenRouter Provider",
  version: "0.1.0",
  apiVersion: "1.0.0",
  main: "dist/extension.js",
  runtime: { kind: "extensionHost", language: "typescript" },
  activationEvents: ["onStartup", "onProviderEditor:openrouter"],
  contributes: {
    providers: [
      {
        providerType: "openrouter",
        displayName: "OpenRouter",
        targetCliKeys: ["claude", "codex"],
        extensionNamespace: "openrouter",
      },
    ],
    ui: {
      "providers.editor.sections": [
        {
          id: "openrouter-routing",
          title: "OpenRouter 路由",
          order: 100,
          schema: {
            type: "section",
            fields: [
              { type: "text", key: "route", label: "Route" },
              { type: "boolean", key: "fallbackEnabled", label: "启用模型兜底" },
            ],
          },
        },
      ],
    },
    commands: [
      {
        command: "acme.openrouter.refreshModels",
        title: "刷新 OpenRouter 模型",
        category: "Provider",
      },
    ],
  },
  capabilities: ["provider.extensionValues", "commands.execute"],
  hostCompatibility: {
    app: ">=0.62.0 <1.0.0",
    pluginApi: "^1.0.0",
    platforms: ["macos", "windows", "linux"],
  },
};

test("validates extension host provider manifest", () => {
  expect(validateManifest(openRouterManifest)).toEqual({ ok: true });
});

test("rejects extension host manifest without main", () => {
  const manifest = { ...openRouterManifest, main: undefined };
  expect(validateManifest(manifest as PluginManifest)).toEqual({
    ok: false,
    error: {
      code: "PLUGIN_MISSING_MAIN",
      message: "extensionHost runtime requires main",
    },
  });
});

test("rejects unknown UI contribution slot", () => {
  const manifest = {
    ...openRouterManifest,
    contributes: {
      ui: {
        "providers.editor.unknown": [],
      },
    },
  };
  expect(validateManifest(manifest as PluginManifest).ok).toBe(false);
});
```

Add a bridge manifest test:

```ts
test("validates protocol bridge manifest", () => {
  const manifest: PluginManifest = {
    id: "acme.bridge",
    name: "Claude OpenAI Gemini Bridge",
    version: "0.1.0",
    apiVersion: "1.0.0",
    main: "dist/extension.js",
    runtime: { kind: "extensionHost", language: "typescript" },
    activationEvents: ["onProtocolBridge:acme.bridge.openai-gemini"],
    contributes: {
      protocols: [
        { protocolId: "openai.chat", direction: "both" },
        { protocolId: "gemini.generateContent", direction: "both" },
      ],
      protocolBridges: [
        {
          bridgeType: "acme.bridge.openai-gemini",
          inboundProtocol: "openai.chat",
          outboundProtocol: "gemini.generateContent",
          supportsStreaming: true,
        },
      ],
    },
    capabilities: ["protocol.bridge"],
    hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^1.0.0" },
  };

  expect(validateManifest(manifest)).toEqual({ ok: true });
});
```

- [ ] **Step 2: Run SDK tests and see them fail**

Run:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
```

Expected: tests fail because `extensionHost`, `main`, `activationEvents`, `contributes`, UI slots, provider contributions, bridge contributions, and capabilities are not defined.

- [ ] **Step 3: Add SDK types and validators**

In `packages/plugin-sdk/src/index.ts`, add these exported shapes and use them from `PluginManifest`:

```ts
export type ExtensionRuntime = {
  kind: "extensionHost";
  language: "typescript";
};

export type LegacyPluginRuntime =
  | { kind: "declarativeRules"; rules: string[] }
  | { kind: "wasm"; abiVersion: string; memoryLimitBytes?: number };

export type PluginRuntime = ExtensionRuntime | LegacyPluginRuntime;

export type ActivationEvent =
  | "onStartup"
  | `onCommand:${string}`
  | `onProviderEditor:${string}`
  | `onProtocolBridge:${string}`
  | `onGatewayHook:${string}`;

export type UiContributionSlot =
  | "app.sidebar.items"
  | "home.overview.cards"
  | "providers.editor.sections"
  | "providers.editor.fields"
  | "providers.card.badges"
  | "providers.card.actions"
  | "settings.sections"
  | "logs.detail.tabs"
  | "logs.detail.actions"
  | "usage.panels"
  | "plugins.detail.panels";

export type PluginCapability =
  | "commands.execute"
  | "storage.plugin"
  | "diagnostics.read"
  | "provider.extensionValues"
  | "provider.requestPreparation"
  | "provider.modelDiscovery"
  | "provider.healthCheck"
  | "protocol.bridge"
  | "gateway.hooks";

export type HostRenderedField =
  | { type: "text"; key: string; label: string; placeholder?: string; required?: boolean }
  | { type: "password"; key: string; label: string; placeholder?: string; required?: boolean }
  | { type: "number"; key: string; label: string; min?: number; max?: number; step?: number }
  | { type: "boolean"; key: string; label: string }
  | { type: "select"; key: string; label: string; options: Array<{ value: string; label: string }> }
  | { type: "textarea"; key: string; label: string; rows?: number }
  | { type: "info"; key: string; label: string; value: string }
  | { type: "button"; key: string; label: string; command: string };

export type HostRenderedSchema =
  | { type: "section"; fields: HostRenderedField[] }
  | { type: "panel"; fields: HostRenderedField[] }
  | { type: "badge"; label: string; tone?: "neutral" | "success" | "warning" | "danger" };

export type UiContribution = {
  id: string;
  title?: string;
  order?: number;
  schema: HostRenderedSchema;
  when?: string;
};

export type ProviderContribution = {
  providerType: string;
  displayName: string;
  targetCliKeys: Array<"claude" | "codex" | "gemini">;
  extensionNamespace: string;
};

export type ProtocolContribution = {
  protocolId: string;
  direction: "inbound" | "outbound" | "both";
};

export type ProtocolBridgeContribution = {
  bridgeType: string;
  inboundProtocol: string;
  outboundProtocol: string;
  supportsStreaming?: boolean;
};

export type CommandContribution = {
  command: string;
  title: string;
  category?: string;
};

export type GatewayHookContribution = PluginHook;

export type GatewayRuleContribution = {
  id?: string;
  rules: string[];
  hooks?: GatewayHookName[];
};

export type PluginContributes = {
  providers?: ProviderContribution[];
  protocols?: ProtocolContribution[];
  protocolBridges?: ProtocolBridgeContribution[];
  commands?: CommandContribution[];
  gatewayHooks?: GatewayHookContribution[];
  gatewayRules?: GatewayRuleContribution[];
  ui?: Partial<Record<UiContributionSlot, UiContribution[]>>;
};
```

Then update `validateManifest()` so:

```ts
if (manifest.runtime.kind === "extensionHost") {
  if (!manifest.main || manifest.main.trim() === "") {
    return invalid("PLUGIN_MISSING_MAIN", "extensionHost runtime requires main");
  }
  if (manifest.runtime.language !== "typescript") {
    return invalid("PLUGIN_INVALID_RUNTIME", "extensionHost language must be typescript");
  }
  const contributionError = validateContributes(manifest.contributes ?? {});
  if (contributionError) return contributionError;
  return validateCapabilities(manifest.capabilities ?? []);
}
```

Keep legacy validation for `declarativeRules` and `wasm`, but rewrite the missing hook error so only legacy top-level runtimes require `hooks`.

- [ ] **Step 4: Add Rust domain parity tests**

Add tests to `src-tauri/src/domain/plugins.rs`:

```rust
#[test]
fn validates_extension_host_provider_manifest() {
    let manifest = serde_json::json!({
        "id": "acme.openrouter",
        "name": "OpenRouter Provider",
        "version": "0.1.0",
        "apiVersion": "1.0.0",
        "main": "dist/extension.js",
        "runtime": { "kind": "extensionHost", "language": "typescript" },
        "activationEvents": ["onStartup", "onProviderEditor:openrouter"],
        "contributes": {
            "providers": [{
                "providerType": "openrouter",
                "displayName": "OpenRouter",
                "targetCliKeys": ["claude", "codex"],
                "extensionNamespace": "openrouter"
            }],
            "ui": {
                "providers.editor.sections": [{
                    "id": "openrouter-routing",
                    "title": "OpenRouter 路由",
                    "order": 100,
                    "schema": {
                        "type": "section",
                        "fields": [{ "type": "text", "key": "route", "label": "Route" }]
                    }
                }]
            },
            "commands": [{
                "command": "acme.openrouter.refreshModels",
                "title": "刷新 OpenRouter 模型"
            }]
        },
        "capabilities": ["provider.extensionValues", "commands.execute"],
        "hostCompatibility": { "app": ">=0.62.0 <1.0.0", "pluginApi": "^1.0.0" }
    });
    let manifest: PluginManifest = serde_json::from_value(manifest).unwrap();

    validate_manifest(&manifest, "0.62.0").unwrap();
}

#[test]
fn extension_host_manifest_rejects_unknown_slot() {
    let manifest = serde_json::json!({
        "id": "acme.bad-slot",
        "name": "Bad Slot",
        "version": "0.1.0",
        "apiVersion": "1.0.0",
        "main": "dist/extension.js",
        "runtime": { "kind": "extensionHost", "language": "typescript" },
        "activationEvents": ["onStartup"],
        "contributes": { "ui": { "providers.editor.unknown": [] } },
        "capabilities": [],
        "hostCompatibility": { "app": ">=0.62.0 <1.0.0", "pluginApi": "^1.0.0" }
    });
    let manifest: PluginManifest = serde_json::from_value(manifest).unwrap();

    let err = validate_manifest(&manifest, "0.62.0").unwrap_err();
    assert_eq!(err.code, "PLUGIN_UNKNOWN_UI_SLOT");
}
```

- [ ] **Step 5: Implement Rust manifest and contribution types**

Add `PluginRuntime::ExtensionHost { language: String }`, `main`, `activation_events`, `contributes`, and `capabilities` to `PluginManifest`. Put contribution types and validators in `src-tauri/src/domain/plugin_contributions.rs`:

```rust
pub(crate) const ACTIVE_UI_SLOTS: &[&str] = &[
    "app.sidebar.items",
    "home.overview.cards",
    "providers.editor.sections",
    "providers.editor.fields",
    "providers.card.badges",
    "providers.card.actions",
    "settings.sections",
    "logs.detail.tabs",
    "logs.detail.actions",
    "usage.panels",
    "plugins.detail.panels",
];

pub fn is_known_ui_slot(slot: &str) -> bool {
    ACTIVE_UI_SLOTS.contains(&slot)
}
```

Keep Rust field names camelCase through serde attributes to preserve Specta frontend shape.

- [ ] **Step 6: Run focused manifest verification**

Run:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
cd src-tauri && cargo test validates_extension_host_provider_manifest --lib
cd src-tauri && cargo test extension_host_manifest_rejects_unknown_slot --lib
```

Expected: all pass.

- [ ] **Step 7: Commit Task 1**

Run:

```bash
git add packages/plugin-sdk/src/index.ts packages/plugin-sdk/src/index.test.ts packages/plugin-sdk/src/index.typecheck.ts docs/plugins/plugin-api-v1-contract.json src-tauri/src/domain/plugins.rs src-tauri/src/domain/plugin_contributions.rs src-tauri/src/domain/mod.rs
git commit -m "feat(plugins): define extension host manifest contract"
```

## Task 2: Active Contribution Registry and Host Query API

**Files:**
- Create: `src-tauri/src/app/plugins/contribution_registry.rs`
- Modify: `src-tauri/src/app/plugins/mod.rs`
- Modify: `src-tauri/src/app/plugin_service.rs`
- Modify: `src-tauri/src/commands/plugins.rs`
- Modify: `src-tauri/src/commands/registry.rs`
- Create: `src/services/pluginContributions.ts`
- Modify: `src/query/keys.ts`
- Modify: `src/query/plugins.ts`
- Modify: `src/generated/bindings.ts`

- [ ] **Step 1: Write failing Rust registry tests**

Create tests in `src-tauri/src/app/plugins/contribution_registry.rs`:

```rust
#[test]
fn contribution_registry_filters_enabled_plugins_and_orders_ui_slots() {
    let enabled = plugin_detail_with_ui(
        "acme.settings",
        crate::plugins::PluginStatus::Enabled,
        "settings.sections",
        "settings-a",
        20,
    );
    let disabled = plugin_detail_with_ui(
        "acme.disabled",
        crate::plugins::PluginStatus::Disabled,
        "settings.sections",
        "settings-hidden",
        10,
    );
    let earlier = plugin_detail_with_ui(
        "acme.settings-earlier",
        crate::plugins::PluginStatus::Enabled,
        "settings.sections",
        "settings-b",
        5,
    );

    let snapshot = ActiveContributionSnapshot::from_plugin_details(&[enabled, disabled, earlier])
        .expect("snapshot");

    let ids: Vec<_> = snapshot
        .ui_for_slot("settings.sections")
        .iter()
        .map(|item| item.contribution_id.as_str())
        .collect();
    assert_eq!(ids, vec!["settings-b", "settings-a"]);
}

#[test]
fn contribution_registry_rejects_unknown_slots() {
    let plugin = plugin_detail_with_raw_ui_slot("acme.bad", "settings.unknown");
    let err = ActiveContributionSnapshot::from_plugin_details(&[plugin]).unwrap_err();
    assert_eq!(err.code(), "PLUGIN_UNKNOWN_UI_SLOT");
}
```

- [ ] **Step 2: Run failing registry tests**

Run:

```bash
cd src-tauri && cargo test contribution_registry_ --lib
```

Expected: fail because no active contribution registry exists.

- [ ] **Step 3: Implement contribution snapshot**

Create `ActiveContributionSnapshot` with these public behaviors:

```rust
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActiveUiContribution {
    pub plugin_id: String,
    pub contribution_id: String,
    pub slot_id: String,
    pub title: Option<String>,
    pub order: i32,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActiveContributionSnapshot {
    pub ui: Vec<ActiveUiContribution>,
    pub providers: Vec<ActiveProviderContribution>,
    pub protocols: Vec<ActiveProtocolContribution>,
    pub protocol_bridges: Vec<ActiveProtocolBridgeContribution>,
    pub commands: Vec<ActiveCommandContribution>,
    pub gateway_hooks: Vec<ActiveGatewayHookContribution>,
    pub gateway_rules: Vec<ActiveGatewayRuleContribution>,
}
```

Rules:

- only `PluginStatus::Enabled` contributes active entries;
- sort UI by `order`, then `plugin_id`, then `contribution_id`;
- every contribution ID must be stable and non-empty;
- duplicate command IDs from different plugins return `PLUGIN_DUPLICATE_COMMAND`;
- unknown UI slots return `PLUGIN_UNKNOWN_UI_SLOT`;
- provider and bridge identifiers must use plugin namespace or be rejected with `PLUGIN_CONTRIBUTION_NAMESPACE_MISMATCH`.

- [ ] **Step 4: Add Tauri command and frontend query**

Add `plugin_active_contributions` returning `ActiveContributionSnapshot`.

Add `src/services/pluginContributions.ts`:

```ts
import { commands, type ActiveContributionSnapshot } from "../generated/bindings";
import { invokeGeneratedIpc } from "./generatedIpc";

export type { ActiveContributionSnapshot };

export async function pluginActiveContributions(): Promise<ActiveContributionSnapshot> {
  return invokeGeneratedIpc<ActiveContributionSnapshot>({
    title: "读取插件扩展点失败",
    cmd: "plugin_active_contributions",
    invoke: async () => commands.pluginActiveContributions(),
  });
}
```

Add query key:

```ts
const pluginContributionsAllKey = ["pluginContributions"] as const;
export const pluginContributionKeys = {
  all: pluginContributionsAllKey,
  active: () => [...pluginContributionsAllKey, "active"] as const,
};
```

Add hook in `src/query/plugins.ts`:

```ts
export function usePluginActiveContributionsQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: pluginContributionKeys.active(),
    queryFn: () => pluginActiveContributions(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}
```

Invalidate `pluginContributionKeys.active()` in every install, update, enable, disable, uninstall, rollback, and quarantine mutation.

- [ ] **Step 5: Regenerate bindings**

Run:

```bash
pnpm tauri:gen-types
```

Expected: `src/generated/bindings.ts` contains `ActiveContributionSnapshot` and `pluginActiveContributions`.

- [ ] **Step 6: Verify registry and query code**

Run:

```bash
cd src-tauri && cargo test contribution_registry_ --lib
pnpm typecheck
pnpm test:unit -- src/query/__tests__/plugins.test.tsx
```

Expected: all pass.

- [ ] **Step 7: Commit Task 2**

Run:

```bash
git add src-tauri/src/app/plugins/contribution_registry.rs src-tauri/src/app/plugins/mod.rs src-tauri/src/app/plugin_service.rs src-tauri/src/commands/plugins.rs src-tauri/src/commands/registry.rs src/services/pluginContributions.ts src/query/keys.ts src/query/plugins.ts src/generated/bindings.ts
git commit -m "feat(plugins): expose active contribution registry"
```

## Task 3: Install Preview and Update Diff Contribution Impact

**Files:**
- Modify: `src-tauri/src/domain/plugins.rs`
- Modify: `src-tauri/src/app/plugin_service.rs`
- Modify: `src-tauri/src/infra/plugins/package.rs`
- Modify: `src/pages/plugins/PluginInstallPreviewDialog.tsx`
- Modify: `src/pages/plugins/PluginUpdatePreviewDialog.tsx`
- Modify: `src/pages/__tests__/PluginsPage.test.tsx`
- Modify: `src/generated/bindings.ts`

- [ ] **Step 1: Write failing preview/diff tests**

Add to `src-tauri/src/app/plugin_service.rs` tests:

```rust
#[test]
fn install_preview_describes_extension_contribution_impact() {
    let ctx = plugin_test_context();
    let package = write_extension_package(
        &ctx,
        "acme.openrouter",
        serde_json::json!({
            "providers": [{
                "providerType": "openrouter",
                "displayName": "OpenRouter",
                "targetCliKeys": ["claude"],
                "extensionNamespace": "openrouter"
            }],
            "ui": {
                "providers.editor.sections": [{
                    "id": "openrouter-routing",
                    "title": "OpenRouter 路由",
                    "order": 10,
                    "schema": { "type": "section", "fields": [] }
                }]
            },
            "commands": [{ "command": "acme.openrouter.refreshModels", "title": "刷新模型" }]
        }),
    );

    let preview = preview_plugin_from_local_package_with_policy(
        &ctx.db,
        &package,
        &ctx.cache_dir,
        "0.62.0",
        LocalPackageInstallPolicy { allow_unsigned: true, developer_mode: true, ..Default::default() },
    )
    .unwrap();

    assert!(preview.contribution_impact.providers.iter().any(|p| p.id == "openrouter"));
    assert!(preview.contribution_impact.ui_slots.iter().any(|s| s.slot_id == "providers.editor.sections"));
    assert!(preview.contribution_impact.commands.iter().any(|c| c.command == "acme.openrouter.refreshModels"));
}

#[test]
fn update_diff_reports_removed_and_added_contributions() {
    let ctx = plugin_test_context();
    install_extension_manifest(&ctx.db, "acme.debug", vec!["logs.detail.tabs"]);
    let package = write_extension_package_with_slots(&ctx, "acme.debug", vec!["settings.sections"]);

    let diff = preview_plugin_update_from_local_package(
        &ctx.db,
        &package,
        &ctx.cache_dir,
        "0.62.0",
        LocalPackageInstallPolicy { allow_unsigned: true, developer_mode: true, ..Default::default() },
    )
    .unwrap();

    assert!(diff.contribution_changes.iter().any(|c| c.name == "logs.detail.tabs" && c.change == "removed"));
    assert!(diff.contribution_changes.iter().any(|c| c.name == "settings.sections" && c.change == "added"));
}
```

- [ ] **Step 2: Run failing preview tests**

Run:

```bash
cd src-tauri && cargo test contribution_impact --lib
```

Expected: fail because `contribution_impact` and `contribution_changes` do not exist.

- [ ] **Step 3: Add preview/update DTOs and impact builders**

Add DTOs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributionImpact {
    pub providers: Vec<PluginContributionImpactItem>,
    pub protocols: Vec<PluginContributionImpactItem>,
    pub protocol_bridges: Vec<PluginContributionImpactItem>,
    pub ui_slots: Vec<PluginUiSlotImpact>,
    pub commands: Vec<PluginCommandImpact>,
    pub gateway: Vec<PluginContributionImpactItem>,
    pub capabilities: Vec<String>,
}
```

Add `contribution_impact` to `PluginInstallPreview` and `contribution_changes` to `PluginUpdateDiff`. The preview builder must calculate impact from normalized manifest contributions even when the package is not installed.

- [ ] **Step 4: Validate package `main` path**

In `src-tauri/src/infra/plugins/package.rs`, when `runtime.kind == extensionHost`, validate:

- `main` is a relative path;
- `main` does not escape the package root;
- extracted file exists;
- file size is at most `1 MiB` for M1;
- file extension is `.js` or `.cjs`.

Return `PLUGIN_EXTENSION_MAIN_MISSING`, `PLUGIN_EXTENSION_MAIN_INVALID`, or `PLUGIN_EXTENSION_MAIN_TOO_LARGE` with clear messages.

- [ ] **Step 5: Update preview dialogs**

In `PluginInstallPreviewDialog.tsx`, add a compact section titled `扩展范围` showing:

- Provider;
- 页面区域;
- 协议/转译;
- 命令;
- 网关;
- 能力.

In `PluginUpdatePreviewDialog.tsx`, render added/removed/changed contribution entries beside permission changes.

- [ ] **Step 6: Verify preview/update behavior**

Run:

```bash
cd src-tauri && cargo test contribution_impact --lib
pnpm test:unit -- src/pages/__tests__/PluginsPage.test.tsx
pnpm typecheck
pnpm tauri:gen-types
```

Expected: all pass and generated bindings are stable after committed Rust types.

- [ ] **Step 7: Commit Task 3**

Run:

```bash
git add src-tauri/src/domain/plugins.rs src-tauri/src/app/plugin_service.rs src-tauri/src/infra/plugins/package.rs src/pages/plugins/PluginInstallPreviewDialog.tsx src/pages/plugins/PluginUpdatePreviewDialog.tsx src/pages/__tests__/PluginsPage.test.tsx src/generated/bindings.ts
git commit -m "feat(plugins): show extension contribution impact"
```

## Task 4: Provider Extension Storage and Gateway Visibility

**Files:**
- Create: `src-tauri/src/infra/db/migrations/v34_to_v35.rs`
- Modify: `src-tauri/src/infra/db/migrations/mod.rs`
- Modify: `src-tauri/src/infra/db/migrations/ensure.rs`
- Modify: `src-tauri/src/infra/db/migrations/baseline_v25.rs`
- Modify: `src-tauri/src/infra/db/migrations/tests.rs`
- Modify: `src-tauri/src/domain/providers/types.rs`
- Modify: `src-tauri/src/domain/providers/queries.rs`
- Modify: `src-tauri/src/domain/providers/tests.rs`
- Modify: `src-tauri/src/app/provider_service.rs`
- Modify: `src-tauri/src/commands/providers/crud.rs`
- Modify: `src/services/providers/providers.ts`
- Modify: `src/generated/bindings.ts`

- [ ] **Step 1: Write failing migration and provider tests**

Add migration test in `src-tauri/src/infra/db/migrations/tests.rs`:

```rust
#[test]
fn migrations_create_provider_extension_values_table() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    apply_migrations(&mut conn).expect("apply migrations");

    assert!(test_has_table(&conn, "provider_extension_values"));
    assert!(test_has_column(&conn, "provider_extension_values", "provider_id"));
    assert!(test_has_column(&conn, "provider_extension_values", "plugin_id"));
    assert!(test_has_column(&conn, "provider_extension_values", "namespace"));
    assert!(test_has_column(&conn, "provider_extension_values", "values_json"));
}
```

Add provider test in `src-tauri/src/domain/providers/tests.rs`:

```rust
#[test]
fn provider_upsert_saves_and_preserves_extension_values_by_namespace() {
    let db = test_db();
    let mut params = default_provider_params("openrouter-provider");
    params.extension_values = vec![ProviderExtensionValuesInput {
        plugin_id: "acme.openrouter".to_string(),
        namespace: "openrouter".to_string(),
        values: serde_json::json!({ "route": "auto", "fallbackEnabled": true }),
    }];
    let saved = upsert(&db, params).expect("save provider");
    assert_eq!(
        saved.extension_values[0].values["route"],
        serde_json::json!("auto")
    );

    let mut edit = default_provider_params("openrouter-provider-renamed");
    edit.provider_id = Some(saved.id);
    edit.extension_values = Vec::new();
    let edited = upsert(&db, edit).expect("edit provider");

    assert_eq!(edited.extension_values.len(), 1);
    assert_eq!(
        edited.extension_values[0].values["fallbackEnabled"],
        serde_json::json!(true)
    );
}

#[test]
fn provider_duplicate_copies_extension_values() {
    let db = test_db();
    let mut params = default_provider_params("provider-with-extension");
    params.extension_values = vec![ProviderExtensionValuesInput {
        plugin_id: "acme.openrouter".to_string(),
        namespace: "openrouter".to_string(),
        values: serde_json::json!({ "headersPolicy": "strip" }),
    }];
    let source = upsert(&db, params).expect("save provider");

    let duplicate = duplicate_for_tests(&db, source.id).expect("duplicate provider");

    assert_eq!(duplicate.extension_values.len(), 1);
    assert_eq!(
        duplicate.extension_values[0].values["headersPolicy"],
        serde_json::json!("strip")
    );
}
```

- [ ] **Step 2: Run failing provider storage tests**

Run:

```bash
cd src-tauri && cargo test provider_extension_values --lib
```

Expected: fail because table and provider DTO fields do not exist.

- [ ] **Step 3: Add migration and ensure patches**

Create `v34_to_v35.rs` with:

```rust
pub(super) fn migrate_v34_to_v35(conn: &mut rusqlite::Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| format!("failed to start v34->v35: {e}"))?;
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS provider_extension_values (
  provider_id INTEGER NOT NULL,
  plugin_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  values_json TEXT NOT NULL DEFAULT '{}',
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(provider_id, plugin_id, namespace),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE,
  FOREIGN KEY(plugin_id) REFERENCES plugins(plugin_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_extension_values_plugin_namespace
  ON provider_extension_values(plugin_id, namespace);
"#,
    )
    .map_err(|e| format!("failed to migrate v34->v35: {e}"))?;
    super::set_user_version(&tx, 35)?;
    tx.commit().map_err(|e| format!("failed to commit v34->v35: {e}"))?;
    Ok(())
}
```

Update `LATEST_SCHEMA_VERSION` to `35`, wire the match arm, and add the same table to `ensure.rs` and fresh schema.

- [ ] **Step 4: Add provider DTOs and repository helpers**

Add to `src-tauri/src/domain/providers/types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderExtensionValues {
    pub plugin_id: String,
    pub namespace: String,
    pub values: serde_json::Value,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderExtensionValuesInput {
    pub plugin_id: String,
    pub namespace: String,
    pub values: serde_json::Value,
}
```

Add `extension_values: Vec<ProviderExtensionValues>` to `ProviderSummary` and `ProviderForGateway`, and `extension_values: Vec<ProviderExtensionValuesInput>` to `ProviderUpsertParams`.

In `queries.rs`, add helpers:

```rust
fn list_extension_values(conn: &Connection, provider_id: i64) -> AppResult<Vec<ProviderExtensionValues>>;
fn save_extension_values(conn: &Connection, provider_id: i64, values: &[ProviderExtensionValuesInput]) -> AppResult<bool>;
fn copy_extension_values(conn: &Connection, from_provider_id: i64, to_provider_id: i64) -> AppResult<()>;
```

`save_extension_values` must upsert only submitted `(plugin_id, namespace)` rows and leave unrelated rows intact.

- [ ] **Step 5: Wire app/frontend payloads**

Add `extension_values` to `ProviderUpsertInput` as `extension_values: Option<Vec<ProviderExtensionValuesInput>>` with camelCase Specta output. In `src/services/providers/providers.ts`, extend `ProviderUpsertInput` and `toProviderUpsertPayload()`:

```ts
extensionValues: input.extensionValues ?? [],
```

Ensure `toProviderSummary()` preserves generated `extension_values` and still narrows only `cli_key` and `auth_mode`.

- [ ] **Step 6: Verify provider storage**

Run:

```bash
cd src-tauri && cargo test provider_extension_values --lib
cd src-tauri && cargo test migrations_create_provider_extension_values_table --lib
pnpm tauri:gen-types
pnpm test:unit -- src/services/providers/__tests__/providers.service.test.ts
pnpm typecheck
```

Expected: all pass.

- [ ] **Step 7: Commit Task 4**

Run:

```bash
git add src-tauri/src/infra/db/migrations/v34_to_v35.rs src-tauri/src/infra/db/migrations/mod.rs src-tauri/src/infra/db/migrations/ensure.rs src-tauri/src/infra/db/migrations/baseline_v25.rs src-tauri/src/infra/db/migrations/tests.rs src-tauri/src/domain/providers/types.rs src-tauri/src/domain/providers/queries.rs src-tauri/src/domain/providers/tests.rs src-tauri/src/app/provider_service.rs src-tauri/src/commands/providers/crud.rs src/services/providers/providers.ts src/generated/bindings.ts
git commit -m "feat(providers): add plugin extension value storage"
```

## Task 5: Host-Rendered UI Contribution Framework

**Files:**
- Create: `src/plugins/contributions/types.ts`
- Create: `src/plugins/contributions/useActiveContributions.ts`
- Create: `src/plugins/contributions/HostRenderedContribution.tsx`
- Create: `src/plugins/contributions/ContributionSlot.tsx`
- Create: `src/plugins/contributions/__tests__/HostRenderedContribution.test.tsx`
- Modify: `src/pages/providers/ProviderEditorDialog.tsx`
- Modify: `src/pages/providers/useProviderEditorForm.ts`
- Modify: `src/pages/providers/providerEditorActionContext.ts`
- Modify: `src/pages/providers/providerEditorSubmitModel.ts`
- Modify: `src/pages/providers/__tests__/ProviderEditorDialog.test.tsx`
- Modify: `src/pages/settings/SettingsMainColumn.tsx`
- Modify: `src/pages/settings/__tests__/SettingsMainColumn.test.tsx`
- Modify: `src/components/home/RequestLogDetailDialog.tsx`
- Modify: `src/components/home/__tests__/RequestLogDetailDialog.test.tsx`

- [ ] **Step 1: Write failing renderer tests**

Create `src/plugins/contributions/__tests__/HostRenderedContribution.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";
import { HostRenderedContribution } from "../HostRenderedContribution";

test("renders host-owned fields and emits value changes", async () => {
  const onChange = vi.fn();
  render(
    <HostRenderedContribution
      contribution={{
        pluginId: "acme.openrouter",
        contributionId: "openrouter-routing",
        slotId: "providers.editor.sections",
        order: 10,
        title: "OpenRouter 路由",
        schema: {
          type: "section",
          fields: [
            { type: "text", key: "route", label: "Route" },
            { type: "boolean", key: "fallbackEnabled", label: "启用兜底" },
          ],
        },
      }}
      values={{ route: "auto", fallbackEnabled: false }}
      disabled={false}
      onChange={onChange}
      onCommand={vi.fn()}
    />
  );

  await userEvent.clear(screen.getByLabelText("Route"));
  await userEvent.type(screen.getByLabelText("Route"), "quality");
  await userEvent.click(screen.getByLabelText("启用兜底"));

  expect(onChange).toHaveBeenCalledWith("route", "quality");
  expect(onChange).toHaveBeenCalledWith("fallbackEnabled", true);
});

test("renders invalid schema as non-crashing diagnostic", () => {
  render(
    <HostRenderedContribution
      contribution={{
        pluginId: "acme.bad",
        contributionId: "bad",
        slotId: "settings.sections",
        order: 1,
        schema: { type: "unknown" },
      }}
      values={{}}
      disabled={false}
      onChange={vi.fn()}
      onCommand={vi.fn()}
    />
  );

  expect(screen.getByText("插件界面无法渲染")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run failing UI tests**

Run:

```bash
pnpm test:unit -- src/plugins/contributions/__tests__/HostRenderedContribution.test.tsx
```

Expected: fail because renderer files do not exist.

- [ ] **Step 3: Implement contribution renderer and slot helper**

Implement `HostRenderedContribution` with existing primitives:

- `FormField`
- `Input`
- `Switch`
- `Select`
- `Textarea`
- `Button`
- `Card`
- `Badge`

Rules:

- unknown schema returns a small warning panel, not an exception;
- field keys must be passed back as `(key, value)`;
- `button` fields call `onCommand(command, { pluginId, contributionId })`;
- `disabled` disables inputs and buttons;
- renderer does not use `dangerouslySetInnerHTML`.

Implement `ContributionSlot`:

```tsx
export function ContributionSlot({
  slotId,
  valuesByContribution,
  disabled,
  onValueChange,
  onCommand,
}: ContributionSlotProps) {
  const contributions = useContributionsForSlot(slotId);
  if (contributions.length === 0) return null;
  return (
    <>
      {contributions.map((contribution) => (
        <HostRenderedContribution
          key={`${contribution.pluginId}:${contribution.contributionId}`}
          contribution={contribution}
          values={valuesByContribution[contribution.contributionId] ?? {}}
          disabled={disabled}
          onChange={(fieldKey, value) => onValueChange(contribution, fieldKey, value)}
          onCommand={(command, args) => onCommand(command, args)}
        />
      ))}
    </>
  );
}
```

- [ ] **Step 4: Add Provider editor slots and save path**

In `ProviderEditorDialog.tsx`, render `providers.editor.sections` between auth-specific fields and stream timeout. In `useProviderEditorForm.ts`, derive plugin extension field values from `provider.extension_values`, update local state on contribution value changes, and pass `extensionValues` into `buildProviderEditorUpsertInput()`.

In `providerEditorSubmitModel.ts`, include:

```ts
extensionValues: ctx.extensionValues,
```

Add test in `ProviderEditorDialog.test.tsx` that renders an OpenRouter field, edits it, saves, and expects:

```ts
expect(lastCall.extensionValues).toEqual([
  {
    pluginId: "acme.openrouter",
    namespace: "openrouter",
    values: { route: "quality" },
  },
]);
```

- [ ] **Step 5: Add Settings and Request Log slots**

In `SettingsMainColumn.tsx`, render `settings.sections` as a full-width section below core settings and above advanced/system sections.

In `RequestLogDetailDialog.tsx`, render `logs.detail.tabs` as extra tabs after built-in summary/chain/raw tabs. A contributed tab receives trace metadata through command args, not through direct database access.

- [ ] **Step 6: Verify UI framework**

Run:

```bash
pnpm test:unit -- src/plugins/contributions/__tests__/HostRenderedContribution.test.tsx
pnpm test:unit -- src/pages/providers/__tests__/ProviderEditorDialog.test.tsx
pnpm test:unit -- src/pages/settings/__tests__/SettingsMainColumn.test.tsx
pnpm test:unit -- src/components/home/__tests__/RequestLogDetailDialog.test.tsx
pnpm typecheck
```

Expected: all pass.

- [ ] **Step 7: Commit Task 5**

Run:

```bash
git add src/plugins/contributions src/pages/providers/ProviderEditorDialog.tsx src/pages/providers/useProviderEditorForm.ts src/pages/providers/providerEditorActionContext.ts src/pages/providers/providerEditorSubmitModel.ts src/pages/providers/__tests__/ProviderEditorDialog.test.tsx src/pages/settings/SettingsMainColumn.tsx src/pages/settings/__tests__/SettingsMainColumn.test.tsx src/components/home/RequestLogDetailDialog.tsx src/components/home/__tests__/RequestLogDetailDialog.test.tsx
git commit -m "feat(plugins): render host-owned UI contributions"
```

## Task 6: Extension Host Worker Process and Lifecycle

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/app/plugins/extension_host_worker.rs`
- Create: `src-tauri/src/app/plugins/extension_host.rs`
- Modify: `src-tauri/src/app/plugins/process_runtime.rs`
- Modify: `src-tauri/src/app/plugins/runtime_lifecycle.rs`
- Modify: `src-tauri/src/app/plugins/runtime_manager.rs`
- Modify: `src-tauri/src/app/plugins/mod.rs`

- [ ] **Step 1: Write failing worker protocol tests**

Create tests in `src-tauri/src/app/plugins/extension_host.rs`:

```rust
#[tokio::test]
async fn extension_host_activates_and_dispatches_command() {
    let dir = tempfile::tempdir().unwrap();
    write_extension_main(
        dir.path(),
        r#"
module.exports.activate = function(api) {
  api.commands.registerCommand("acme.echo", function(args) {
    return { ok: true, echo: args.text };
  });
};
"#,
    );
    let manifest = extension_manifest("acme.echo", "dist/extension.js");
    let mut host = ExtensionHostInstance::start_for_tests(manifest, dir.path().to_path_buf())
        .await
        .expect("start extension host");

    host.activate().await.expect("activate");
    let result = host
        .execute_command("acme.echo", serde_json::json!({ "text": "hello" }))
        .await
        .expect("execute command");

    assert_eq!(result["echo"], serde_json::json!("hello"));
    host.dispose().await;
}

#[tokio::test]
async fn extension_host_timeout_kills_worker() {
    let dir = tempfile::tempdir().unwrap();
    write_extension_main(
        dir.path(),
        r#"
module.exports.activate = function(api) {
  api.commands.registerCommand("acme.never", function() {
    while (true) {}
  });
};
"#,
    );
    let manifest = extension_manifest("acme.never", "dist/extension.js");
    let mut host = ExtensionHostInstance::start_for_tests_with_timeout(
        manifest,
        dir.path().to_path_buf(),
        std::time::Duration::from_millis(10),
    )
    .await
    .expect("start extension host");

    host.activate().await.expect("activate");
    let err = host.execute_command("acme.never", serde_json::json!({})).await.unwrap_err();
    assert_eq!(err.code(), "PLUGIN_EXTENSION_CALL_TIMEOUT");
    assert!(!host.is_running());
}
```

- [ ] **Step 2: Run failing runtime tests**

Run:

```bash
cd src-tauri && cargo test extension_host_ --lib
```

Expected: fail because Extension Host worker does not exist.

- [ ] **Step 3: Add worker entry before Tauri/WebView startup**

In `src-tauri/src/main.rs`, route worker mode before Windows WebView2 check:

```rust
fn main() {
    if std::env::args().any(|arg| arg == "--extension-host-worker") {
        aio_coding_hub_lib::run_extension_host_worker();
        return;
    }

    #[cfg(windows)]
    ensure_webview2_or_exit();

    aio_coding_hub_lib::run()
}
```

In `src-tauri/src/lib.rs`, export:

```rust
pub fn run_extension_host_worker() {
    crate::app::plugins::extension_host_worker::run_stdio_worker();
}
```

- [ ] **Step 4: Implement worker process protocol**

`extension_host_worker.rs` must:

- read config JSON from a `--extension-host-config <path>` arg;
- load `plugin.json` metadata and `main` JavaScript from installed dir;
- emit `{"method":"extension.ready","params":{"workerVersion":1}}` as first stdout line;
- accept JSON-RPC methods:
  - `extension.handshake`;
  - `extension.activate`;
  - `extension.deactivate`;
  - `commands.execute`;
- enforce max input/output JSON line bytes in the worker as well as the parent;
- return JSON-RPC errors with stable codes.

For M1 JavaScript execution:

- use an embedded JS engine in the worker process;
- provide a CommonJS-style `module.exports`;
- expose an `api` object with `commands.registerCommand(command, handler)`;
- require registered command IDs to match manifest-declared commands;
- serialize returned JS values through JSON.

- [ ] **Step 5: Implement parent-side instance manager**

`extension_host.rs` must:

- spawn the current executable with `--extension-host-worker --extension-host-config <file>`;
- clear environment except safe `PATH`;
- set `stdin`, `stdout`, and `stderr` piped;
- wait for `extension.ready` within startup timeout;
- send handshake with plugin id, version, api version, and contribution hash;
- activate on demand;
- dispatch command calls;
- kill/reap on protocol error, timeout, crash, dispose, update, uninstall, app shutdown;
- expose `is_running()` for tests.

Refactor `process_runtime.rs` into a generic JSON-RPC line client with:

```rust
pub(crate) async fn call_method(&mut self, method: &str, params: serde_json::Value) -> AppResult<serde_json::Value>;
```

Keep `call_hook()` as a thin legacy wrapper calling `plugin.handleHook`.

- [ ] **Step 6: Upgrade lifecycle registry**

Extend `RuntimeLifecycleRegistry` with extension instance lifecycle methods:

```rust
pub(crate) trait PluginRuntimeInstanceRegistry: Send + Sync {
    fn retain_for_plugins(&self, plugins: &[PluginDetail]);
    fn dispose_plugin(&self, plugin_id: &str);
    fn dispose_all(&self);
}
```

Keep `PluginRuntimeCache` registered and called so legacy runtime cache tests continue to pass.

- [ ] **Step 7: Verify runtime lifecycle**

Run:

```bash
cd src-tauri && cargo test extension_host_ --lib
cd src-tauri && cargo test process_runtime --lib
cd src-tauri && cargo test lifecycle_registry --lib
pnpm tauri:check
```

Expected: all pass.

- [ ] **Step 8: Commit Task 6**

Run:

```bash
git add src-tauri/Cargo.toml src-tauri/src/main.rs src-tauri/src/lib.rs src-tauri/src/app/plugins/extension_host_worker.rs src-tauri/src/app/plugins/extension_host.rs src-tauri/src/app/plugins/process_runtime.rs src-tauri/src/app/plugins/runtime_lifecycle.rs src-tauri/src/app/plugins/runtime_manager.rs src-tauri/src/app/plugins/mod.rs
git commit -m "feat(plugins): add managed extension host worker"
```

## Task 7: Commands, Storage, Diagnostics, and Generalized Execution Reports

**Files:**
- Modify: `src-tauri/src/domain/plugins.rs`
- Modify: `src-tauri/src/infra/plugins/runtime_reports.rs`
- Modify: `src-tauri/src/infra/plugins/replay_export.rs`
- Modify: `src-tauri/src/app/plugins/extension_host.rs`
- Modify: `src-tauri/src/app/plugin_service.rs`
- Modify: `src-tauri/src/commands/plugins.rs`
- Modify: `src/services/plugins.ts`
- Modify: `src/query/plugins.ts`
- Modify: `src/pages/plugins/PluginRuntimeReportsPanel.tsx`
- Modify: `src/generated/bindings.ts`

- [ ] **Step 1: Write failing command/report tests**

Add Rust test in `src-tauri/src/app/plugin_service.rs`:

```rust
#[tokio::test]
async fn plugin_command_execution_records_extension_report() {
    let ctx = extension_service_test_context().await;
    install_enabled_extension_with_command(&ctx, "acme.debug", "acme.debug.exportTrace").await;

    let value = execute_plugin_command(
        &ctx.db,
        &ctx.extension_hosts,
        "acme.debug.exportTrace",
        serde_json::json!({ "traceId": "trace-1" }),
    )
    .await
    .expect("execute command");

    assert_eq!(value["ok"], serde_json::json!(true));
    let reports = crate::infra::plugins::runtime_reports::list_extension_execution_reports(
        &ctx.db,
        Some("acme.debug"),
        Some("command"),
        Some("acme.debug.exportTrace"),
        None,
        20,
    )
    .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].contribution_type, "command");
}
```

- [ ] **Step 2: Run failing command/report tests**

Run:

```bash
cd src-tauri && cargo test plugin_command_execution_records_extension_report --lib
```

Expected: fail because command dispatch and generalized reports are not wired.

- [ ] **Step 3: Generalize execution reports**

Add `PluginExtensionExecutionReport`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginExtensionExecutionReport {
    pub id: i64,
    pub plugin_id: String,
    pub contribution_type: String,
    pub contribution_id: String,
    pub command_or_hook: Option<String>,
    pub trace_id: Option<String>,
    pub status: String,
    pub started_at_ms: i64,
    pub duration_ms: i64,
    pub failure_kind: Option<String>,
    pub error_code: Option<String>,
    pub input_budget: serde_json::Value,
    pub output_budget: serde_json::Value,
    pub mutation_summary: serde_json::Value,
    pub replayable: bool,
    pub created_at: i64,
}
```

Keep current `PluginHookExecutionReport` by mapping hook rows into the generalized report or by maintaining compatibility fields until the UI is migrated.

- [ ] **Step 4: Add command execution command**

Add Tauri command:

```rust
#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginExecuteCommandInput {
    pub command: String,
    pub args: serde_json::Value,
}
```

`plugin_execute_command` must:

- find command contribution in active snapshot;
- start/activate the owning extension host if needed;
- pass args through byte-limited JSON-RPC;
- record success/failure execution report;
- return `PLUGIN_COMMAND_NOT_FOUND` if not declared;
- return `PLUGIN_COMMAND_PLUGIN_DISABLED` if plugin not active.

- [ ] **Step 5: Add host-mediated storage and diagnostics M1**

Add worker-to-host API methods:

- `storage.get(pluginId, key)`;
- `storage.set(pluginId, key, value)`;
- `diagnostics.getRuntimeReports(pluginId, limit)`.

Store plugin config/storage in existing `plugin_configs` for M1 under a reserved object key:

```json
{
  "config": {},
  "storage": {
    "key": "value"
  }
}
```

Reject storage payloads over `64 KiB` per plugin with `PLUGIN_STORAGE_LIMIT_EXCEEDED`.

- [ ] **Step 6: Update frontend command wrappers**

Add in `src/services/plugins.ts`:

```ts
export async function pluginExecuteCommand(command: string, args: JsonValue) {
  const normalizedCommand = normalizeRequiredText("command", command);
  return invokeGeneratedIpc<JsonValue>({
    title: "执行插件命令失败",
    cmd: "plugin_execute_command",
    args: { command: normalizedCommand, args },
    invoke: async () => commands.pluginExecuteCommand({ command: normalizedCommand, args }),
  });
}
```

Use this in `ContributionSlot` `onCommand` wiring from Task 5.

- [ ] **Step 7: Verify command/report APIs**

Run:

```bash
cd src-tauri && cargo test plugin_command_execution_records_extension_report --lib
cd src-tauri && cargo test runtime_reports --lib
pnpm tauri:gen-types
pnpm test:unit -- src/pages/plugins/__tests__/PluginRuntimeReportsPanel.test.tsx
pnpm typecheck
```

Expected: all pass.

- [ ] **Step 8: Commit Task 7**

Run:

```bash
git add src-tauri/src/domain/plugins.rs src-tauri/src/infra/plugins/runtime_reports.rs src-tauri/src/infra/plugins/replay_export.rs src-tauri/src/app/plugins/extension_host.rs src-tauri/src/app/plugin_service.rs src-tauri/src/commands/plugins.rs src/services/plugins.ts src/query/plugins.ts src/pages/plugins/PluginRuntimeReportsPanel.tsx src/generated/bindings.ts
git commit -m "feat(plugins): execute extension commands with reports"
```

## Task 8: Protocol Bridge Contributions M1

**Files:**
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/registry.rs`
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/traits.rs`
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/bridge.rs`
- Modify: `src-tauri/src/gateway/proxy/protocol_bridge/e2e_tests.rs`
- Modify: `src-tauri/src/domain/providers/types.rs`
- Modify: `src-tauri/src/domain/providers/queries.rs`
- Modify: `src-tauri/src/domain/providers/tests.rs`
- Modify: `src-tauri/src/app/plugins/extension_host.rs`

- [ ] **Step 1: Write failing bridge registry tests**

Add to `registry.rs` tests:

```rust
#[test]
fn bridge_registry_lists_plugin_declared_bridge_types() {
    let builtins = BuiltinBridgeRegistry::default();
    let contributions = vec![PluginBridgeContribution {
        plugin_id: "acme.bridge".to_string(),
        bridge_type: "acme.bridge.openai-gemini".to_string(),
        inbound_protocol: "openai.chat".to_string(),
        outbound_protocol: "gemini.generateContent".to_string(),
        supports_streaming: true,
    }];

    let registry = CombinedBridgeRegistry::new(builtins, contributions);

    assert!(registry.contains("cx2cc"));
    assert!(registry.contains("acme.bridge.openai-gemini"));
    assert_eq!(
        registry.describe("acme.bridge.openai-gemini").unwrap().plugin_id.as_deref(),
        Some("acme.bridge")
    );
}

#[test]
fn provider_upsert_accepts_declared_plugin_bridge_type() {
    let db = test_db();
    install_enabled_bridge_plugin(&db, "acme.bridge", "acme.bridge.openai-gemini");
    let mut params = default_provider_params("bridge-provider");
    params.bridge_type = Some("acme.bridge.openai-gemini".to_string());

    let saved = upsert(&db, params).expect("save provider");

    assert_eq!(saved.bridge_type.as_deref(), Some("acme.bridge.openai-gemini"));
}
```

- [ ] **Step 2: Run failing bridge tests**

Run:

```bash
cd src-tauri && cargo test bridge_registry_lists_plugin_declared_bridge_types --lib
cd src-tauri && cargo test provider_upsert_accepts_declared_plugin_bridge_type --lib
```

Expected: fail because only `cx2cc` is accepted.

- [ ] **Step 3: Implement combined bridge registry**

Replace the static-only lookup with:

```rust
pub(crate) enum BridgeRegistration {
    Builtin { bridge_type: &'static str, factory: BridgeFactory },
    Plugin { plugin_id: String, bridge_type: String, inbound_protocol: String, outbound_protocol: String, supports_streaming: bool },
}
```

`get_bridge("cx2cc")` keeps returning the built-in bridge. Plugin bridge lookup returns a descriptor used by gateway dispatch to call the extension host.

- [ ] **Step 4: Add plugin bridge JSON-RPC envelopes**

Define call DTOs in `traits.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginBridgeRequest {
    pub bridge_type: String,
    pub phase: String,
    pub body: serde_json::Value,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginBridgeResponse {
    pub body: serde_json::Value,
    pub diagnostics: serde_json::Value,
}
```

For this task, support non-stream request translation through:

- `bridge.requestToIr`;
- `bridge.irToRequest`;
- `bridge.responseToIr`;
- `bridge.irToResponse`.

Streaming declarations are validated and listed, but streaming execution can return `PLUGIN_BRIDGE_STREAMING_UNAVAILABLE` until stream chunk envelopes are fully wired.

- [ ] **Step 5: Preserve Rust host ownership**

Gateway bridge execution must keep Rust in control of:

- upstream URL;
- API key/OAuth token injection;
- HTTP transport;
- byte limits;
- stream framing;
- final error classification;
- trace logging.

Plugin bridge calls only transform JSON/IR payloads.

- [ ] **Step 6: Verify bridge contributions**

Run:

```bash
cd src-tauri && cargo test bridge_registry_lists_plugin_declared_bridge_types --lib
cd src-tauri && cargo test provider_upsert_accepts_declared_plugin_bridge_type --lib
cd src-tauri && cargo test protocol_bridge --lib
pnpm tauri:check
```

Expected: all pass.

- [ ] **Step 7: Commit Task 8**

Run:

```bash
git add src-tauri/src/gateway/proxy/protocol_bridge/registry.rs src-tauri/src/gateway/proxy/protocol_bridge/traits.rs src-tauri/src/gateway/proxy/protocol_bridge/bridge.rs src-tauri/src/gateway/proxy/protocol_bridge/e2e_tests.rs src-tauri/src/domain/providers/types.rs src-tauri/src/domain/providers/queries.rs src-tauri/src/domain/providers/tests.rs src-tauri/src/app/plugins/extension_host.rs
git commit -m "feat(plugins): register protocol bridge contributions"
```

## Task 9: Gateway Contribution Migration and Privacy Filter Equivalence

**Files:**
- Modify: `src-tauri/src/gateway/plugins/registry.rs`
- Modify: `src-tauri/src/gateway/plugins/pipeline.rs`
- Modify: `src-tauri/src/gateway/plugins/contract.rs`
- Modify: `src-tauri/src/app/plugins/official.rs`
- Modify: `src-tauri/src/app/plugins/official_assets.rs`
- Modify: `src-tauri/src/app/plugins/official_privacy_filter_runtime.rs`
- Modify: `src-tauri/resources/plugins/official/privacy-filter/plugin.json`
- Modify: `docs/plugins/reference/declarative-rules.md`
- Modify: `docs/plugin-manifest-v1.md`

- [ ] **Step 1: Write failing gateway migration tests**

Add to `src-tauri/src/gateway/plugins/registry.rs`:

```rust
#[test]
fn legacy_gateway_hooks_normalize_to_gateway_contributions() {
    let manifest: crate::plugins::PluginManifest = serde_json::from_value(serde_json::json!({
        "id": "acme.legacy-redactor",
        "name": "Legacy Redactor",
        "version": "0.1.0",
        "apiVersion": "1.0.0",
        "runtime": { "kind": "declarativeRules", "rules": ["rules/main.json"] },
        "hooks": [{ "name": "gateway.request.afterBodyRead", "priority": 50 }],
        "permissions": ["request.body.read", "request.body.write"],
        "hostCompatibility": { "app": ">=0.62.0 <1.0.0", "pluginApi": "^1.0.0" }
    }))
    .unwrap();

    let normalized = normalize_gateway_contributions(&manifest).expect("normalize");

    assert_eq!(normalized.gateway_rules.len(), 1);
    assert_eq!(normalized.gateway_hooks[0].name, "gateway.request.afterBodyRead");
}
```

Add privacy filter equivalence test:

```rust
#[tokio::test]
async fn official_privacy_filter_behavior_matches_before_contribution_migration() {
    let plugin = official_privacy_filter_detail_for_tests();
    let pipeline = GatewayPluginPipeline::for_tests(
        vec![plugin],
        Arc::new(RuntimeGatewayPluginExecutor::for_tests()),
        GatewayPluginPipelineConfig::default(),
    );

    let output = pipeline
        .run_request_hook(request_with_authorization_header("Bearer secret-token"))
        .await
        .expect("privacy filter");

    assert!(!output.headers_debug_string().contains("secret-token"));
}
```

- [ ] **Step 2: Run failing gateway migration tests**

Run:

```bash
cd src-tauri && cargo test legacy_gateway_hooks_normalize_to_gateway_contributions --lib
cd src-tauri && cargo test official_privacy_filter_behavior_matches_before_contribution_migration --lib
```

Expected: first fails until normalization exists; second must pass after migration work.

- [ ] **Step 3: Normalize gateway contributions**

Implement `normalize_gateway_contributions()`:

- If manifest has `contributes.gatewayHooks`, use those.
- If manifest has `contributes.gatewayRules`, use those.
- If legacy top-level `hooks` and `runtime.kind == declarativeRules`, convert to `gatewayHooks` and `gatewayRules`.
- If `official.privacy-filter` uses native runtime, expose it as a gateway hook contribution internally.

- [ ] **Step 4: Update official privacy filter manifest**

Update `src-tauri/resources/plugins/official/privacy-filter/plugin.json` to include `contributes.gatewayHooks` while keeping the legacy shape only if current installer paths still require it.

- [ ] **Step 5: Update docs**

In `docs/plugin-manifest-v1.md`, lead with Extension Host manifest. Move old `runtime + hooks` under `Legacy gateway compatibility`.

In `docs/plugins/reference/declarative-rules.md`, state that declarative rules are a gateway contribution family for constrained transformations and not the target for flexible UI/provider/protocol plugins.

- [ ] **Step 6: Verify gateway behavior**

Run:

```bash
cd src-tauri && cargo test gateway_plugin_pipeline --lib
cd src-tauri && cargo test privacy_filter --lib
pnpm check:plugin-system-docs
```

Expected: all pass.

- [ ] **Step 7: Commit Task 9**

Run:

```bash
git add src-tauri/src/gateway/plugins/registry.rs src-tauri/src/gateway/plugins/pipeline.rs src-tauri/src/gateway/plugins/contract.rs src-tauri/src/app/plugins/official.rs src-tauri/src/app/plugins/official_assets.rs src-tauri/src/app/plugins/official_privacy_filter_runtime.rs src-tauri/resources/plugins/official/privacy-filter/plugin.json docs/plugins/reference/declarative-rules.md docs/plugin-manifest-v1.md
git commit -m "feat(plugins): migrate gateway hooks to contributions"
```

## Task 10: Developer Tooling, Examples, and Documentation

**Files:**
- Modify: `packages/create-aio-plugin/src/scaffold.ts`
- Modify: `packages/create-aio-plugin/src/scaffold.test.ts`
- Modify: `packages/create-aio-plugin/src/devtools.ts`
- Modify: `packages/create-aio-plugin/src/cli.ts`
- Create: `docs/plugins/extension-host.md`
- Modify: `docs/plugins/README.md`
- Modify: `docs/plugins/reference/manifest.md`
- Modify: `docs/plugins/reference/sdk.md`
- Modify: `docs/plugins/reference/compatibility.md`
- Modify: `docs/plugins/runtime/README.md`
- Modify: `docs/plugins/examples/README.md`

- [ ] **Step 1: Write failing scaffolder tests**

Add to `packages/create-aio-plugin/src/scaffold.test.ts`:

```ts
test("scaffolds extension host plugin", async () => {
  const dir = await scaffoldPlugin(tempDir, {
    pluginId: "acme.openrouter",
    template: "extension",
  });

  expect(readJson(path.join(dir, "plugin.json"))).toMatchObject({
    id: "acme.openrouter",
    runtime: { kind: "extensionHost", language: "typescript" },
    main: "dist/extension.js",
  });
  expect(await fileExists(path.join(dir, "src", "extension.ts"))).toBe(true);
  expect(await fileExists(path.join(dir, "tsconfig.json"))).toBe(true);
});

test("publish-check prints extension contribution impact", async () => {
  const dir = await scaffoldPlugin(tempDir, {
    pluginId: "acme.openrouter",
    template: "extension",
  });
  const result = await runPublishCheck(dir);

  expect(result.stdout).toContain("runtime: extensionHost");
  expect(result.stdout).toContain("contributions:");
  expect(result.stdout).toContain("providers");
  expect(result.stdout).toContain("ui");
  expect(result.exitCode).toBe(0);
});
```

- [ ] **Step 2: Run failing tooling tests**

Run:

```bash
pnpm --filter create-aio-plugin test
```

Expected: fail because `extension` template and contribution impact output do not exist.

- [ ] **Step 3: Implement extension template**

Template files:

```text
plugin.json
src/extension.ts
tsconfig.json
package.json
README.md
fixtures/provider-editor.json
```

Generated `src/extension.ts`:

```ts
import type { AioExtensionContext } from "@aio-coding-hub/plugin-sdk";

export function activate(ctx: AioExtensionContext) {
  ctx.commands.registerCommand("PLUGIN_ID.hello", (args) => ({
    ok: true,
    message: `hello ${String(args?.name ?? "AIO")}`,
  }));
}

export function deactivate() {}
```

Generated `plugin.json` must use the new manifest shape and one `settings.sections` contribution by default so users immediately see a visible extension point.

- [ ] **Step 4: Update validate and publish-check**

`validate --strict` must:

- validate manifest through SDK;
- check `main` exists after build;
- check command actions are declared;
- check contribution slots are known;
- print a deterministic contribution summary.

`publish-check` must print:

```text
runtime: extensionHost
main: dist/extension.js
contributions:
  ui:
    settings.sections: 1
  commands:
    acme.plugin.hello
capabilities:
  commands.execute
```

- [ ] **Step 5: Write docs**

Create `docs/plugins/extension-host.md` with sections:

- 插件最终形态;
- TypeScript only;
- package layout;
- manifest;
- activation;
- UI schema;
- provider extension values;
- commands;
- protocol bridge declarations;
- diagnostics/reports;
- lifecycle and disposal;
- why React/WebView/native injection is not supported.

Update existing docs so their first path points to Extension Host. Keep declarative rules visible as a constrained gateway rule option.

- [ ] **Step 6: Verify tooling/docs**

Run:

```bash
pnpm --filter create-aio-plugin test
pnpm --filter create-aio-plugin typecheck
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm check:plugin-system-docs
pnpm check:plugin-api-contract
```

Expected: all pass.

- [ ] **Step 7: Commit Task 10**

Run:

```bash
git add packages/create-aio-plugin/src/scaffold.ts packages/create-aio-plugin/src/scaffold.test.ts packages/create-aio-plugin/src/devtools.ts packages/create-aio-plugin/src/cli.ts docs/plugins/extension-host.md docs/plugins/README.md docs/plugins/reference/manifest.md docs/plugins/reference/sdk.md docs/plugins/reference/compatibility.md docs/plugins/runtime/README.md docs/plugins/examples/README.md
git commit -m "feat(plugins): scaffold extension host plugins"
```

## Task 11: Full Verification and Integration Review

**Files:**
- Modify only files touched by failures discovered in this task.

- [ ] **Step 1: Run generated binding check**

Run:

```bash
pnpm tauri:gen-types
git diff -- src/generated/bindings.ts
```

Expected: no unexpected uncommitted generated binding diff after all committed type changes.

- [ ] **Step 2: Run frontend verification**

Run:

```bash
pnpm typecheck
pnpm lint
pnpm test:unit -- src/plugins/contributions src/pages/providers src/pages/plugins src/pages/settings src/components/home
```

Expected: all pass.

- [ ] **Step 3: Run SDK/tooling verification**

Run:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk test
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
pnpm --filter create-aio-plugin test
pnpm --filter create-aio-plugin typecheck
```

Expected: all pass.

- [ ] **Step 4: Run Rust verification**

Run:

```bash
cd src-tauri && cargo fmt -- --check
cd src-tauri && cargo test --lib plugin
cd src-tauri && cargo test --lib provider_extension_values
cd src-tauri && cargo test --lib extension_host
cd src-tauri && cargo test --lib protocol_bridge
cd src-tauri && cargo check --locked
```

Expected: all pass.

- [ ] **Step 5: Run docs and contract checks**

Run:

```bash
pnpm check:plugin-api-contract
pnpm check:plugin-system-docs
pnpm check:spec-links
```

Expected: all pass.

- [ ] **Step 6: Manual desktop smoke**

Run:

```bash
pnpm tauri:dev
```

Smoke steps:

1. Open Plugins page.
2. Import a generated `extension` plugin package.
3. Preview shows `extensionHost`, UI slot, command, and capabilities.
4. Enable plugin.
5. Open Settings and confirm contributed section appears.
6. Disable plugin and confirm contributed section disappears without app restart.
7. Create or edit a Provider and confirm provider extension fields persist.
8. Run a contributed command from a host-rendered button and confirm a runtime report appears.

Expected: desktop app remains stable and no internal browser/WebView plugin container appears.

- [ ] **Step 7: Commit final fixes**

If Step 1-6 require fixes, commit the minimal corrections:

```bash
git add <changed-files>
git commit -m "fix(plugins): stabilize extension host integration"
```

If no fixes are needed, do not create an empty commit.

## Self-Review

- Spec coverage:
  - Extension Manifest v1: Task 1.
  - Contribution registry and lifecycle invalidation: Task 2 and Task 6.
  - Package preview/update diff contribution impact: Task 3.
  - Provider extension storage: Task 4 and Task 5.
  - Host-rendered UI schema across multiple pages: Task 5.
  - TypeScript Extension Host runtime: Task 6.
  - Commands/storage/diagnostics/reports: Task 7.
  - Protocol bridge contributions: Task 8.
  - Gateway legacy migration and privacy filter consistency: Task 9.
  - Developer tooling/examples/docs: Task 10.
  - Verification and acceptance: Task 11.
- Placeholder scan:
  - The plan avoids open placeholder words and defines concrete files, tests, commands, expected results, and commit points.
- Type consistency:
  - `runtime.kind = "extensionHost"`, `main`, `activationEvents`, `contributes`, `capabilities`, `ProviderExtensionValues`, `ActiveContributionSnapshot`, `PluginExtensionExecutionReport`, and `plugin_execute_command` are introduced once and reused consistently.
- Architecture risk check:
  - UI remains host-rendered, worker process is disposable, provider extension values are namespaced, bridge plugins transform data but do not own transport, and legacy gateway behavior remains test-covered.
