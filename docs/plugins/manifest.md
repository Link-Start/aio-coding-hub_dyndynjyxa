# Plugin Manifest

The manifest file is `plugin.json`. It follows manifest v1 from `docs/plugin-manifest-v1.md`.

Required fields:

- `id`: publisher-scoped ID such as `publisher.plugin-name`.
- `name`: display name.
- `version`: SemVer plugin version.
- `apiVersion`: SemVer plugin API version.
- `runtime`: either `declarativeRules` or `wasm`.
- `hooks`: hook declarations.
- `permissions`: requested permission names.
- `hostCompatibility`: app and plugin API compatibility constraints.

Runtime examples:

```json
{ "kind": "declarativeRules", "rules": ["rules/main.json"] }
```

```json
{ "kind": "wasm", "abiVersion": "1.0.0", "memoryLimitBytes": 16777216 }
```

`hostCompatibility` must include `app` and `pluginApi`; `platforms` may restrict OS support.
