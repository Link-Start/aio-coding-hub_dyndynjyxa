# Plugin Getting Started

Use this guide to create, validate, package, and import a local AIO Coding Hub plugin.

For a full map of plugin documentation, see [Plugin Development](./README.md).

## Choose A Runtime

Start with `declarativeRules` when the plugin can be expressed as regex matching, replacement, warning, blocking, or appending messages.

Use WASM only when a plugin needs code execution and can fit the isolated WASM ABI. Arbitrary JavaScript and TypeScript plugins are not supported.

## Install The SDK

Community plugins should use `@aio-coding-hub/plugin-sdk` for shared manifest, hook, permission, and validation types.

```bash
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
```

SDK details: [Plugin SDK](./sdk.md).

WASM plugins should use the Rust `aio-plugin-wasm-sdk` contracts:

```bash
pnpm plugin-wasm-sdk:test
```

The minimal Rust example lives at `packages/plugin-wasm-sdk/examples/redactor`.

## Create A Plugin

Use `create-aio-plugin` to scaffold a local plugin:

```bash
pnpm --filter create-aio-plugin test
pnpm create-aio-plugin acme.redactor rule
pnpm create-aio-plugin acme.policy wasm
```

Each scaffold contains a `plugin.json`. Rule plugins also contain `rules/main.json`; WASM plugins contain a minimal Rust entrypoint skeleton.

## Minimal Declarative Rule Plugin

`plugin.json`:

```json
{
  "id": "acme.redactor",
  "name": "Acme Redactor",
  "version": "0.1.0",
  "apiVersion": "1.0.0",
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 50,
      "failurePolicy": "fail-open"
    }
  ],
  "permissions": ["request.body.read", "request.body.write"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  }
}
```

`rules/main.json`:

```json
{
  "rules": [
    {
      "id": "redact-openai-key",
      "hook": "gateway.request.afterBodyRead",
      "target": {
        "field": "request.body",
        "jsonPath": "$.messages[*].content"
      },
      "match": {
        "regex": "sk-(?:proj-)?[A-Za-z0-9_-]{20,}"
      },
      "action": {
        "kind": "replace",
        "replacement": "[REDACTED]"
      }
    }
  ]
}
```

Rule details: [Declarative Rules Runtime](./declarative-rules.md).

## Local Development Flow

1. Edit `plugin.json`.
2. Validate the manifest with `pnpm create-aio-plugin validate acme.redactor`.
3. Replay a fixture with `pnpm create-aio-plugin replay acme.redactor`.
4. Pack the plugin as `acme.redactor.aio-plugin` with `pnpm create-aio-plugin pack acme.redactor`.
5. Sign package bytes with `pnpm create-aio-plugin sign <bytes> [privateKey]`.
6. Verify package bytes with `pnpm create-aio-plugin verify <bytes> <signature> <publicKey>`.
7. Import the package from the Plugins page.

WASM marketplace execution remains disabled by default until host policy explicitly enables it.

## Next References

- [Manifest](./manifest.md)
- [Hooks](./hooks.md)
- [Permissions](./permissions.md)
- [Config Schema](./config-schema.md)
- [Official Examples](./official-examples.md)
- [Publishing](./publishing.md)
