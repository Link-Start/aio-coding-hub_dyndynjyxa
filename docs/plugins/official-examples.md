# Official Example Plugins

Official example plugins are bundled fixtures used to validate the plugin API and demonstrate safe plugin patterns.

Current official IDs:

- `official.prompt-optimizer`
- `official.safety-detector`
- `official.redactor`
- `official.privacy-filter`

They are available from the Plugins page through the official plugin install buttons.

## Prompt Optimizer

ID: `official.prompt-optimizer`

Runtime: `declarativeRules`

Demonstrates request-body prompt transformation.

Hooks:

- `gateway.request.afterBodyRead`

Permissions:

- `request.body.read`
- `request.body.write`

Behavior:

- Adds a system/developer-style instruction for chat request bodies.
- Prepends an instruction for `input` and `prompt` string request bodies.

## Safety Detector

ID: `official.safety-detector`

Runtime: `declarativeRules`

Demonstrates response and stream blocking.

Hooks:

- `gateway.response.after`
- `gateway.response.chunk`

Permissions:

- `response.body.read`
- `stream.inspect`
- `stream.modify`

Behavior:

- Blocks known dangerous shell patterns in non-stream responses.
- Blocks dangerous stream chunks before they are forwarded.
- Uses fail-closed hook policy for safety-sensitive behavior.

## Sensitive Data Redactor

ID: `official.redactor`

Runtime: `declarativeRules`

Demonstrates log and response redaction with broad redaction hooks.

Hooks:

- `gateway.request.beforeSend`
- `gateway.response.chunk`
- `gateway.response.after`
- `log.beforePersist`

Permissions:

- `request.body.read`
- `response.body.read`
- `log.redact`

Behavior:

- Redacts bearer tokens.
- Redacts GitHub tokens.
- Redacts URL query token values.
- Redacts database connection credentials in logs.

## Privacy Filter

ID: `official.privacy-filter`

Runtime: `declarativeRules`

Inspired by [packyme/privacy-filter](https://github.com/packyme/privacy-filter).

Demonstrates pre-upstream privacy filtering for prompts and request logs.

Hooks:

- `gateway.request.afterBodyRead`
- `log.beforePersist`

Permissions:

- `request.body.read`
- `request.body.write`
- `log.redact`

Behavior:

- Redacts emails.
- Redacts Chinese mobile phone numbers.
- Redacts Chinese ID card patterns.
- Redacts bank card candidates.
- Redacts IPv4 addresses.
- Redacts OpenAI, AWS, GitHub, Google, Slack, JWT, private key, bearer token, URL query token, and contextual password/API key patterns.

Important limitation:

The upstream `packyme/privacy-filter` Go project includes richer algorithmic behavior such as Luhn checks, entropy scoring, gitleaks-style rule loading, span merging, and false-positive mitigation. The official `declarativeRules` example intentionally preserves only the high-value regex subset that can run safely without code execution.

## Where They Live

Official plugin fixtures are currently stored in the host repository:

```text
src-tauri/tests/fixtures/plugins/official/
```

The host registers them in:

```text
src-tauri/src/app/plugins/official.rs
```

They remain in this repository while plugin API v1 is stabilizing. After the API is stable, the SDK, scaffolder, official plugins, and community examples can be split into dedicated repositories.
