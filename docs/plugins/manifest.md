# Plugin Manifest

The manifest file is `plugin.json`. It follows manifest v1 from `docs/plugin-manifest-v1.md`.

Required fields:

- `id`: publisher-scoped ID such as `publisher.plugin-name`.
- `name`: display name.
- `version`: SemVer plugin version.
- `apiVersion`: SemVer plugin API version.
- `runtime`: `declarativeRules` or `wasm` for community plugins. `native` is reserved for built-in official plugins.
- `hooks`: hook declarations.
- `permissions`: requested permission names.
- `hostCompatibility`: app and plugin API compatibility constraints.

The `official.*` namespace is reserved for built-in official plugins. Local, marketplace, and GitHub packages must use their own publisher namespace.

Runtime examples:

```json
{ "kind": "declarativeRules", "rules": ["rules/main.json"] }
```

```json
{ "kind": "wasm", "abiVersion": "1.0.0", "memoryLimitBytes": 16777216 }
```

Official-only native runtime example:

```json
{ "kind": "native", "engine": "privacyFilter" }
```

Only built-in official plugins installed from the official source may use `native`.

`hostCompatibility` must include `app` and `pluginApi`; `platforms` may restrict OS support.
