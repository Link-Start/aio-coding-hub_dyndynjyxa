# Declarative Rules Runtime

`declarativeRules` is the preferred short-term runtime for community plugins. It lets plugins inspect and transform request bodies, response bodies, stream chunks, and log messages without executing arbitrary code in the host.

## Manifest Runtime

Declare rule files in `plugin.json`:

```json
{
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  }
}
```

Rule paths are relative to the plugin root. They must not contain `..` or absolute path prefixes.

## Rule File Shape

```json
{
  "rules": [
    {
      "id": "redact-api-key",
      "hook": "gateway.request.afterBodyRead",
      "target": {
        "field": "request.body",
        "jsonPath": "$.messages[*].content"
      },
      "match": {
        "regex": "sk-[A-Za-z0-9_-]{20,}",
        "caseSensitive": true
      },
      "action": {
        "kind": "replace",
        "replacement": "[REDACTED]"
      }
    }
  ]
}
```

Each rule has:

- `id`: stable rule identifier for diagnostics.
- `hook`: one hook name from [Hooks](./hooks.md).
- `target`: where the rule scans.
- `match.regex`: Rust `regex` pattern.
- `match.caseSensitive`: optional, defaults to `true`.
- `action`: what to do after a match.
- `when`: optional runtime filter.

## Targets

Supported `target.field` values:

- `request.body`
- `response.body`
- `stream.chunk`
- `log.message`

`request.body` and `response.body` may include `jsonPath` for string fields inside JSON payloads.

Supported JSONPath subset:

- `$`
- `.key`
- `[*]`

Examples:

- `$.input`
- `$.prompt`
- `$.messages[*].content`
- `$.choices[*].message.content`

Quoted keys, filters, recursive descent, numeric indexes, and arbitrary JSONPath expressions are not supported.

## Actions

### replace

Replaces all regex matches in the selected text.

```json
{
  "kind": "replace",
  "replacement": "[REDACTED]"
}
```

Capture groups are supported by the Rust regex replacement syntax:

```json
{
  "kind": "replace",
  "replacement": "$1[SECRET]"
}
```

### block

Stops the current request, response, or stream processing when the pipeline and hook allow blocking.

```json
{
  "kind": "block",
  "reason": "Dangerous output blocked by plugin."
}
```

### warn

Records a warning reason without mutating the target.

```json
{
  "kind": "warn",
  "message": "Suspicious content detected."
}
```

### appendMessage

Appends a `system` or `developer` message to chat-style request bodies.

```json
{
  "kind": "appendMessage",
  "role": "system",
  "content": "Clarify intent and preserve user constraints."
}
```

## Conditional Rules

Use `when` to limit a rule by CLI, model, or config value:

```json
{
  "when": {
    "cliKeys": ["codex", "claude"],
    "models": ["gpt-4.1"],
    "configEquals": {
      "redactBeforeUpstream": true
    }
  }
}
```

All provided conditions must match.

## Permissions

Rules still need matching manifest permissions:

- Reading request body: `request.body.read`
- Mutating request body: `request.body.write`
- Reading response body: `response.body.read`
- Mutating response body: `response.body.write`
- Reading stream chunks: `stream.inspect`
- Mutating stream chunks: `stream.modify`
- Redacting logs: `log.redact`

The host trims hook context before rule execution and rejects unauthorized mutations after execution.

## Runtime Limits

- Maximum regex pattern length: 4 KiB.
- Maximum compiled regex size: 2 MiB.
- Maximum rules per runtime: 256.
- Hook execution is bounded by the gateway plugin timeout.
- Invalid JSON targets are skipped when the target cannot be parsed as JSON syntax.

## Local Replay Compatibility

`create-aio-plugin replay` implements the host-supported v1.1 declarative rule subset for local fixtures. It is intentionally deterministic and does not execute WASM, process plugins, network calls, or host-only native engines.

Replay supports the same v1.1 rule actions for the community rule runtime: `replace`, `block`, `warn`, and `appendMessage`. For request body rewrites, it supports raw text targets and the documented JSONPath subset such as `$.messages[*].content`, `$.input[*].content[*].text`, and `$.input`.

## Good Uses

- Prompt optimization by appending instructions.
- API key, token, email, and log redaction.
- Safety checks that block known dangerous command patterns.
- Lightweight response warnings.

## Poor Fits

Use WASM or a future isolated process runtime instead when a plugin requires:

- entropy scoring;
- Luhn validation;
- external API calls;
- model-based classification;
- filesystem access;
- complex stateful analysis.
