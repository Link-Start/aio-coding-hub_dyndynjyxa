# Process Plugin Runtime PoC

## Goal

The process runtime PoC explores plugin isolation through a child process using JSON-RPC over stdio. It is a design and lifecycle foundation only: it is disabled by default and has no marketplace enablement by default.

This runtime is not a replacement for the WASM runtime. It exists for future plugins that cannot fit the WASM ABI but still need isolation from the Rust main process and Tauri WebView.

## Boundary

The process runtime runs a plugin executable as a child process with:

- stdin and stdout used only for JSON-RPC over stdio.
- stderr captured as bounded diagnostics.
- no inherited stdin from the app.
- no direct access to Tauri WebView.
- no direct access to app SQLite connections.
- no implicit network or filesystem grant from the host.

Any future filesystem, network, or secret access must go through explicit host-mediated APIs. M5 does not expose those APIs.

## JSON-RPC over stdio

Each request is a single newline-delimited JSON-RPC 2.0 object:

```json
{"jsonrpc":"2.0","id":1,"method":"plugin.handleHook","params":{"hook":"gateway.request.afterBodyRead","context":{}}}
```

Each response is one newline-delimited JSON-RPC 2.0 object:

```json
{"jsonrpc":"2.0","id":1,"result":{"action":"pass"}}
```

The host rejects:

- malformed JSON;
- mismatched IDs;
- responses over the configured byte limit;
- plugin-side JSON-RPC errors;
- output after the hook timeout.

## Lifecycle

The process lifecycle has four bounded phases:

1. Spawn the child with a start timeout.
2. Send a hook request and wait for one response with a hook timeout.
3. Keep the process warm only until idle recycle expires.
4. Kill and reap the child on timeout, crash, protocol error, or idle recycle.

The initial PoC starts one process per test session and reuses it only while it remains healthy and idle time is below the configured threshold.

## Required Limits

- start timeout defaults to 500 ms.
- hook timeout defaults to 300 ms.
- idle recycle defaults to 30 seconds.
- request and response lines are each capped at 256 KiB.
- stderr diagnostics are bounded and never streamed into the UI unbounded.

## Safety Policy

- The runtime is disabled by default.
- There is no marketplace enablement by default.
- Process plugins must be explicitly marked experimental by host policy before use.
- crash isolation must prevent a child exit from crashing the app.
- Timeouts must kill the child process and record an English diagnostic message.
- The host must treat every protocol error as a runtime failure.

## M5 Acceptance Tests

M5 backend tests cover:

- A valid child process starts and returns a JSON-RPC hook result.
- A child that sleeps during startup hits start timeout.
- A child that sleeps during hook handling hits hook timeout and is killed.
- A child that exits early is reported as crash isolation, not a host crash.
- A healthy idle child is recycled after idle recycle.
