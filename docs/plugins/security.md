# Plugin Security

The plugin system is designed around least privilege and isolation.

Core rules:

- no arbitrary JavaScript executes inside the Rust main process.
- no arbitrary JavaScript executes inside the Tauri WebView.
- WASM runs without WASI filesystem or network imports.
- Process runtime PoC is disabled by default.
- Hook failures are audited.
- High-risk hooks may use fail-closed behavior.
- Repeated runtime failures may move a plugin to `quarantined`.

Unsigned offline packages are restricted. High and critical permissions are rejected unless a future explicit trusted policy allows them.
