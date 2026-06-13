# Plugin SDK

`@aio-coding-hub/plugin-sdk` provides shared TypeScript contracts for plugin manifests, hooks, permissions, runtimes, and validation helpers.

`aio-plugin-wasm-sdk` provides the matching Rust/WASM ABI contracts for code plugins that compile to WebAssembly.

The SDK is intended for:

- plugin authors writing `plugin.json`;
- scaffold and packaging tools;
- marketplace/index validation;
- examples and compatibility tests.

## Package Location

In this repository the SDK lives at:

```text
packages/plugin-sdk
packages/plugin-wasm-sdk
```

Run the SDK checks:

```bash
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
pnpm plugin-wasm-sdk:test
```

## Main Types

The SDK exports:

- `PluginManifest`
- `PluginRuntime`
- `PluginHook`
- `GatewayHookName`
- `PluginPermission`
- `PluginPermissionRisk`
- `PluginHookContext`
- `PluginHookResult`

It also exports helpers:

- `permissionRisk(permission)`
- `validateManifest(manifest)`

`create-aio-plugin` uses the SDK for manifest validation and adds local development commands over real plugin directories:

```bash
pnpm create-aio-plugin validate ./acme.redactor
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/request.json gateway.request.afterBodyRead
pnpm create-aio-plugin pack ./acme.redactor
```

The Rust/WASM SDK exports:

- `PluginManifest`
- `PluginRuntime`
- `PluginHook`
- `PluginHostCompatibility`
- `HookRequest`
- `HookResult`
- `HookAction`
- `aio_plugin_entrypoint!`
- pointer/length helpers for the ABI return value

## Minimal Manifest In TypeScript

```ts
import type { PluginManifest } from "@aio-coding-hub/plugin-sdk";
import { validateManifest } from "@aio-coding-hub/plugin-sdk";

const manifest: PluginManifest = {
  id: "acme.redactor",
  name: "Acme Redactor",
  version: "0.1.0",
  apiVersion: "1.0.0",
  runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
  hooks: [{ name: "gateway.request.afterBodyRead", priority: 50 }],
  permissions: ["request.body.read", "request.body.write"],
  hostCompatibility: {
    app: ">=0.56.0 <1.0.0",
    pluginApi: "^1.0.0",
    platforms: ["macos", "windows", "linux"]
  },
  configSchema: {
    type: "object",
    required: ["enabled"],
    properties: {
      enabled: {
        type: "boolean",
        title: "启用处理",
        description: "关闭后插件不会修改请求内容。",
        default: true,
        "x-aio-ui": { widget: "switch", order: 10 }
      }
    }
  }
};

const result = validateManifest(manifest);
if (!result.ok) {
  throw new Error(`${result.error.code}: ${result.error.message}`);
}
```

## SDK Boundary

The SDK is a contract package. It does not execute plugin code and does not grant host capabilities.

The Rust/WASM SDK follows the same rule. It only serializes ABI-compatible JSON, defines hook result helpers, and provides the `aio_plugin_entrypoint!` macro for exporting `aio_plugin_handle`.

`PluginHookResult` uses the same active mutation envelope as the gateway host:

```ts
const result = {
  action: "replace",
  requestBody: "{\"messages\":[]}",
  headers: { "x-plugin-redacted": "1" }
} satisfies PluginHookResult;
```

Use `requestBody`, `responseBody`, `streamChunk`, `logMessage`, and `headers` for replacements. `contextPatch` is not an active vNext gateway mutation field.

Host enforcement still happens in the Rust application:

- manifest compatibility checks;
- permission grants;
- hook context trimming;
- mutation permission enforcement;
- runtime timeout and failure policy handling;
- package checksum/signature verification.

## Rust/WASM Example

The repository includes a minimal WASM redactor example:

```text
packages/plugin-wasm-sdk/examples/redactor
```

It can be tested with:

```bash
pnpm plugin-wasm-sdk:test
```

## Versioning Guidance

- Keep `apiVersion` aligned with the plugin API major version.
- Use SemVer for plugin package versions.
- If the SDK adds backward-compatible types, plugin API major version can stay the same.
- Breaking hook, permission, runtime, or manifest changes require a new plugin API major version.
