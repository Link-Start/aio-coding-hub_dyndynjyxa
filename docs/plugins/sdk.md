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
