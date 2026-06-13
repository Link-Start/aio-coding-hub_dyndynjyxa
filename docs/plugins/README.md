# AIO Coding Hub Plugin Development

This directory is the developer guide for the AIO Coding Hub plugin system.

Plugins extend the local gateway, request and response hooks, log redaction, and GUI-managed configuration. Short-term community plugins should prefer `declarativeRules`; code plugins must use isolated runtimes such as WASM when host policy enables them.

## Start Here

- [Getting Started](./getting-started.md): first local plugin, validation, replay, packaging, and import flow.
- [Plugin SDK](./sdk.md): TypeScript types and validation helpers for plugin authors and tooling.
- [Declarative Rules](./declarative-rules.md): no-code rule runtime for request/log redaction, safety checks, and prompt edits.
- [Official Examples](./official-examples.md): the built-in Privacy Filter and what it demonstrates.

## Core Contracts

- [Manifest](./manifest.md): `plugin.json` required fields and runtime declarations.
- [Full Manifest v1](../plugin-manifest-v1.md): canonical manifest specification with examples.
- [Hooks](./hooks.md): gateway and log hook names, timing, and use cases.
- [Permissions](./permissions.md): permission names, risk levels, and authorization behavior.
- [Config Schema](./config-schema.md): supported config schema subset for GUI rendering and backend validation.
- [Compatibility](./compatibility.md): app, plugin API, platform, and ABI version rules.

## Runtime And Distribution

- [Security](./security.md): least privilege, isolation, signing, and failure policies.
- [Streaming](./streaming.md): bounded stream chunk processing.
- [WASM Runtime](./wasm-runtime.md): ABI v1 design and execution limits.
- [Process Runtime PoC](./process-runtime-poc.md): disabled-by-default process isolation design.
- [Publishing](./publishing.md): `.aio-plugin` packaging, checksum, signatures, updates, and rollback.
- [Architecture Audit](./architecture-audit.md): trust boundaries, runtime choices, performance, and stability guidance.

## Recommended Development Order

1. Choose `declarativeRules` unless the plugin truly needs code execution.
2. Write `plugin.json` with the narrowest hooks and permissions.
3. Add focused fixture rules or WASM entrypoint code.
4. Validate with `create-aio-plugin`.
5. Replay test fixtures before importing into the desktop app.
6. Package and sign release artifacts only after local behavior is stable.

## Current Stability Notes

- Arbitrary JavaScript and TypeScript plugins are not supported.
- WASM and process runtime docs describe the isolation contract; marketplace enablement is still guarded by host policy.
- Active hooks and permissions are the only capabilities accepted by manifest validation; reserved hooks and permissions are documented for future host integration.
- Only `official.privacy-filter` is bundled as an official native plugin. Community extensions should use `declarativeRules`, WASM, or a future isolated process runtime rather than `native`.
