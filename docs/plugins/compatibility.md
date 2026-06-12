# Plugin Compatibility

Compatibility uses SemVer.

Manifest fields:

- `version`: the plugin release version.
- `apiVersion`: the plugin API version used by this manifest.
- `hostCompatibility.app`: compatible aio coding hub app versions.
- `hostCompatibility.pluginApi`: compatible plugin API versions.
- `hostCompatibility.platforms`: optional platform allowlist.

WASM plugins also declare a WASM ABI version:

```json
{ "kind": "wasm", "abiVersion": "1.0.0" }
```

The host rejects unsupported major versions. Future plugin API changes must preserve backward compatibility or require a major version bump.
