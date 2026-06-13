# Plugin Manifest v1

`plugin.json` is the stable package contract between a plugin and aio coding hub. Manifest v1 supports declarative rule plugins first, WASM code plugins when host policy enables them, and a small set of official-only native engines.

## 1. Required Fields

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | Globally unique plugin ID. |
| `name` | string | User-visible name. |
| `version` | string | Plugin version, using SemVer. |
| `apiVersion` | string | Plugin API version, for example `1.0.0`. |
| `runtime` | object | Runtime declaration. |
| `hooks` | array | Hook registrations. |
| `permissions` | array | Requested permissions. |
| `hostCompatibility` | object | Supported aio coding hub host range. |

## 2. Optional Fields

| Field | Type | Description |
| --- | --- | --- |
| `entry` | string | Runtime artifact path, such as `plugin.wasm`; not required for declarative rules. |
| `configSchema` | object | JSON Schema subset for user configuration. |
| `configVersion` | integer | Configuration schema version. |
| `description` | string | Short user-visible summary. |
| `author` | string or object | Author metadata. |
| `homepage` | string | Project homepage URL. |
| `repository` | string or object | Source repository metadata. |
| `license` | string | SPDX license expression when possible. |
| `checksum` | string | Package checksum. |
| `signature` | string | Package signature. |
| `category` | string | `security`, `productivity`, `redaction`, or `utility`. |

## 3. ID And Version Rules

Plugin IDs use the format `publisher.plugin-name`.

- The publisher and name segments must be lowercase ASCII.
- Each segment may contain letters, digits, and hyphens.
- Dots separate namespace segments.
- Path separators, `..`, spaces, shell metacharacters, and empty segments are invalid.
- `official.privacy-filter` is the only bundled official plugin ID.
- The `official.*` namespace can only be installed through the built-in official plugin source; local, marketplace, and GitHub packages must use their own publisher namespace.

Versions must follow SemVer. Pre-release versions are allowed for local development and unsigned packages but marketplace stable releases should use release versions.

`apiVersion` is independent from the app version. The host may add backward-compatible fields within the same major API. Breaking changes require a new major API.

## 4. Runtime

Runtime v1 supports community declarative rules:

```json
{
  "kind": "declarativeRules",
  "rules": ["rules/main.json"]
}
```

WASM runtime:

```json
{
  "kind": "wasm",
  "abiVersion": "1.0.0",
  "memoryLimitBytes": 16777216
}
```

WASM packages are installable only when host policy enables execution. A host that has not enabled WASM execution must reject or disable WASM plugins instead of routing them through another runtime.

Short-term validation must reject arbitrary JavaScript/TypeScript, Node.js, Deno, native dynamic libraries, and WebView code.

Official-only native runtime:

```json
{
  "kind": "native",
  "engine": "privacyFilter"
}
```

`native` is reserved for built-in official plugins installed from the built-in official source. Third-party packages cannot declare host-native engines.

## 5. Host Compatibility

`hostCompatibility` constrains plugin installation and enablement:

```json
{
  "app": ">=0.56.0 <1.0.0",
  "pluginApi": "^1.0.0",
  "platforms": ["macos", "windows", "linux"]
}
```

Incompatible plugins are marked `incompatible` and never enter the hook pipeline.

## 6. Hook v1

Active hooks in plugin API v1 are the hooks currently wired into the gateway or log pipeline. Reserved hooks for future host integration are documented so the names can stay stable, but manifest validation rejects them with `PLUGIN_RESERVED_HOOK` until the host implements their call sites.

| Hook | Trigger | Modification | Default timeout | Default failure policy | Matching permissions |
| --- | --- | --- | --- | --- | --- |
| `gateway.request.afterBodyRead` | Body reader finished buffering allowed body | JSON body, raw body metadata | 200 ms | fail-open | `request.body.read`, `request.body.write` |
| `gateway.request.beforeSend` | Before reqwest sends upstream request | headers and body | 300 ms | fail-open or security fail-closed | `request.header.write`, `request.body.write` |
| `gateway.response.chunk` | Stream chunk before CLI output | chunk pass, replace, block, warn | 20 ms | security fail-closed, non-security fail-open | `stream.inspect`, `stream.modify` |
| `gateway.response.after` | Complete non-stream response below size budget | body pass, replace, block, warn | 300 ms | security fail-closed, non-security fail-open | `response.body.read`, `response.body.write` |
| `gateway.error` | Host or upstream error observed | no host-error hiding | 100 ms | fail-open | `request.meta.read` |
| `log.beforePersist` | Request or audit log before persistence | redacted log fields | 100 ms | fail-closed-to-host-redaction | `log.redact` |

Streaming hooks receive bounded chunks plus a fixed-size sliding window. They do not receive an unlimited full response.

Reserved hooks:

- `gateway.request.received`
- `gateway.request.beforeProviderResolution`
- `gateway.response.headers`

## 7. Permission v1

Reserved permissions for future host-mediated APIs are documented for naming stability, but manifest validation rejects them with `PLUGIN_RESERVED_PERMISSION` until those APIs exist.

| Permission | Risk | Description |
| --- | --- | --- |
| `request.meta.read` | low | Read method, path, CLI key, trace ID, provider hints. |
| `request.header.read` | medium | Read non-sensitive request headers. |
| `request.header.readSensitive` | high | Read sensitive request headers such as `Authorization` and `Cookie`. |
| `request.header.write` | high | Modify request headers. |
| `request.body.read` | high | Read request body. |
| `request.body.write` | high | Modify request body. |
| `response.header.read` | low | Read response headers. |
| `response.header.write` | medium | Modify safe response headers returned to CLI. |
| `response.body.read` | high | Read complete non-stream response body when below budget. |
| `response.body.write` | high | Modify non-stream response body. |
| `stream.inspect` | high | Inspect streamed chunks and sliding window. |
| `stream.modify` | high | Replace or block streamed chunks. |
| `log.redact` | medium | Redact log fields before persistence. |

Reserved permissions:

| Permission | Risk | Future host-mediated API |
| --- | --- | --- |
| `plugin.storage` | medium | Use isolated plugin storage. |
| `network.fetch` | high | Make host-mediated network requests. |
| `file.read` | high | Read host-mediated files. |
| `file.write` | high | Write host-mediated files. |
| `secret.read` | critical | Read host-managed secrets. |

高危权限需要二次授权. Critical permissions require second confirmation and stronger UI copy.

插件升级新增权限必须重新授权. The host must keep the plugin disabled or partially disabled until the new permissions are approved.

## 8. Hook And Permission Compatibility

Validation rejects:

- Unknown hook names.
- Reserved hook names.
- Unknown permissions.
- Reserved permissions.
- Write permissions requested for hooks that cannot modify.
- Sensitive header reads without `request.header.readSensitive`.
- Body writes without matching body read/write permission.
- `stream.modify` actions without `stream.modify`.
- `network.fetch`, `file.read`, `file.write`, or `secret.read` until the host provides those APIs.

## 9. Config Schema Subset

The supported `configSchema` subset includes:

- `string`
- `number`
- `integer`
- `boolean`
- `enum`
- `array`
- `object`
- `password`

Plugins cannot provide custom GUI code. The host renders the form, validates before saving, and validates again before enabling. Sensitive values are not returned to the frontend in plaintext.

## 10. State Machine

States:

- `available`
- `installed`
- `enabled`
- `disabled`
- `update_available`
- `incompatible`
- `quarantined`
- `uninstalled`

Allowed transitions:

| From | To | Trigger |
| --- | --- | --- |
| `available` | `installed` | User installs package or market plugin. |
| `installed` | `enabled` | User grants required permissions and valid config. |
| `installed` | `disabled` | User installs but does not enable. |
| `enabled` | `disabled` | User disables plugin. |
| `disabled` | `enabled` | User enables plugin after validation. |
| `enabled` | `update_available` | Market detects newer compatible version. |
| `disabled` | `update_available` | Market detects newer compatible version. |
| `update_available` | `enabled` | Update succeeds and permissions remain valid. |
| `update_available` | `disabled` | Update succeeds but needs new permission approval. |
| `installed` | `incompatible` | Host/API/platform version is incompatible. |
| `enabled` | `quarantined` | Repeated crash, timeout, signature failure, or revoked market status. |
| `disabled` | `quarantined` | Signature failure or revoked market status. |
| `quarantined` | `disabled` | User acknowledges and restores after validation. |
| any active state | `uninstalled` | User uninstalls plugin. |

Upgrade failure restores the previous version, config snapshot, permissions, and enabled state. Signature failure moves the plugin to `quarantined`. Runtime crash and repeated timeout can move an enabled plugin to `quarantined`.

## 11. Example Manifest: Community Prompt Helper

```json
{
  "id": "acme.prompt-helper",
  "name": "Prompt Helper",
  "version": "1.0.0",
  "apiVersion": "1.0.0",
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 100,
      "failurePolicy": "fail-open"
    }
  ],
  "permissions": ["request.body.read", "request.body.write"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  },
  "configSchema": {
    "type": "object",
    "required": ["mode"],
    "properties": {
      "mode": {
        "type": "string",
        "enum": ["append_instruction", "prepend_context"]
      },
      "onlyModels": {
        "type": "array",
        "items": { "type": "string" }
      },
      "onlyClis": {
        "type": "array",
        "items": { "type": "string", "enum": ["claude", "codex", "gemini"] }
      }
    }
  }
}
```

## 12. Example Manifest: Privacy Filter

```json
{
  "id": "official.privacy-filter",
  "name": "Privacy Filter",
  "version": "1.0.0",
  "apiVersion": "1.0.0",
  "category": "privacy",
  "description": "Official native privacy filter aligned with packyme/privacy-filter for pre-upstream prompt and log redaction.",
  "homepage": "https://github.com/packyme/privacy-filter",
  "repository": {
    "type": "git",
    "url": "https://github.com/packyme/privacy-filter.git"
  },
  "license": "MIT",
  "runtime": {
    "kind": "native",
    "engine": "privacyFilter"
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 5,
      "failurePolicy": "fail-closed"
    },
    {
      "name": "log.beforePersist",
      "priority": 1,
      "failurePolicy": "fail-closed"
    }
  ],
  "permissions": ["request.body.read", "request.body.write", "log.redact"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  },
  "configSchema": {
    "type": "object",
    "required": ["redactBeforeUpstream", "redactLogs", "profile"],
    "properties": {
      "redactBeforeUpstream": {
        "type": "boolean"
      },
      "redactLogs": {
        "type": "boolean"
      },
      "profile": {
        "type": "string",
        "enum": ["balanced"]
      }
    }
  }
}
```
