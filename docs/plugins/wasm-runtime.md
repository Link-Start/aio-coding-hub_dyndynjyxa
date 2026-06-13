# WASM Plugin Runtime Design

## Goal

WASM runtime is the policy-gated community code-plugin runtime for aio coding hub. It exists to run plugin logic outside the Rust main-process trust boundary while preserving deterministic resource limits, permission trimming, auditability, and cross-platform behavior. `declarativeRules` is the default community runtime; WASM is for plugins that truly need code execution once host policy enables it.

WASM packages are installable only when host policy enables execution. This runtime is not enabled for arbitrary marketplace execution until the compatibility tests, signing policy, and host allowlist are all in place. `plugin.wasm` artifacts are packaged as binary files by `create-aio-plugin pack`.

## vNext Host Policy

In vNext, WASM manifests are part of the compatibility contract, but gateway execution is policy-gated. A plugin with `runtime.kind = "wasm"` must not be enabled unless host policy explicitly sets `wasm_enabled = true`; otherwise the gateway returns `PLUGIN_RUNTIME_DISABLED`.

WASM enablement is rejected while host policy disables execution. The plugin can still be packaged and validated as an ABI artifact, but users cannot enable it in the gateway until the host policy explicitly allows WASM execution.

## WASM ABI v1

The WASM ABI v1 contract is intentionally narrow:

- The guest module exports one guest entrypoint named `aio_plugin_handle`.
- The host writes one UTF-8 JSON request into guest memory.
- The guest returns one UTF-8 JSON response pointer/length pair encoded as `u64`.
- The response must be a hook result compatible with the existing gateway plugin pipeline.
- The host only passes permission-trimmed JSON, never internal Rust references, database handles, provider secrets, or WebView state.

Rust plugin authors should use `aio-plugin-wasm-sdk` from `packages/plugin-wasm-sdk` for these ABI shapes and the `aio_plugin_entrypoint!` macro.

The initial JSON envelope is:

```json
{
  "abiVersion": "1.0.0",
  "pluginId": "publisher.plugin-name",
  "hook": "gateway.request.afterBodyRead",
  "traceId": "optional-trace-id",
  "config": {},
  "context": {}
}
```

The guest response envelope is:

```json
{
  "action": "replace",
  "requestBody": "{\"messages\":[]}",
  "headers": {
    "x-plugin-redacted": "1"
  },
  "audit": []
}
```

`action` may be `pass`, `replace`, `block`, or `warn` only when the hook and granted permissions allow that action. Replacement fields use the same active gateway envelope as the host: `requestBody`, `responseBody`, `streamChunk`, `logMessage`, and `headers`. Legacy `contextPatch` output is rejected in vNext.

## Guest Entrypoint

The guest entrypoint signature is:

```wat
(func (export "aio_plugin_handle") (param i32 i32) (result i64))
```

The two parameters are pointer and byte length for the request JSON. The return value packs response pointer and byte length:

```text
return = (ptr << 32) | len
```

The host requires an exported linear memory named `memory`. The host does not pass host functions for filesystem, network, environment variables, wall-clock access, process spawning, or random data in ABI v1.

## memory/time/filesystem/network restrictions

M5 enforces these default limits:

- Maximum input JSON bytes: 256 KiB.
- Maximum output JSON bytes: 256 KiB.
- Default guest memory limit: 16 MiB unless a lower manifest limit is provided.
- Default hook timeout: inherited from the gateway hook timeout, capped by the runtime.
- Fuel is consumed per Wasmtime instruction and exhausted modules are terminated.
- no WASI filesystem imports are provided.
- no network imports are provided.
- no environment variable imports are provided.
- no host clock import is provided.

The host never mounts app data, plugin data, logs, cache, or user directories into WASM. Any future storage API must be a dedicated, permission-gated host function with size limits and audit logs.

## Execution Model

The host creates a fresh Wasmtime store for each hook call in M5. This is slower than pooling but simpler and safer for the foundation phase. Pooling can be added later after deterministic reset semantics are tested.

Each execution:

1. Validates the manifest runtime kind is `wasm`.
2. Reads the module from the installed plugin directory.
3. Compiles and instantiates the module with no WASI imports.
4. Writes the permission-trimmed JSON envelope into exported memory.
5. Executes `aio_plugin_handle`.
6. Reads and bounds-checks the response JSON.
7. Converts timeout, trap, bad pointer, malformed JSON, and missing export into structured runtime failures.

## Security Requirements

- host only passes permission-trimmed JSON.
- The plugin cannot read sensitive headers unless `request.header.readSensitive` was granted and the hook allows it.
- The plugin cannot write body, headers, or stream chunks unless the matching write/modify permission was granted.
- The plugin cannot access files because no WASI filesystem imports are available.
- The plugin cannot access the network because no network imports are available.
- fuel-based termination is mandatory for dead-loop protection.
- All runtime failures must be auditable with English diagnostic messages.

## Failure Policy

WASM runtime failures are isolated to the current hook invocation:

- Missing export: runtime failure, plugin result is treated as hook error.
- Trap or fuel exhaustion: runtime failure, plugin result is treated as hook error.
- Oversized input or output: runtime failure, plugin result is treated as hook error.
- Malformed output JSON: runtime failure, plugin result is treated as hook error.

The gateway pipeline still decides fail-open or fail-closed from the hook policy. The runtime itself never silently ignores errors.

## M5 Acceptance Tests

M5 backend tests cover:

- A valid WASM module can echo a small hook response.
- A module importing WASI filesystem APIs is denied at instantiation.
- A dead-loop module terminates by fuel exhaustion instead of blocking the host.

SDK and example checks run through:

```bash
pnpm plugin-wasm-sdk:test
```
