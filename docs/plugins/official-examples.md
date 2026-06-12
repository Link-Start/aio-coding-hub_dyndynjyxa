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

Runtime: `native:privacyFilter`

Aligned with the core redaction behavior of [packyme/privacy-filter](https://github.com/packyme/privacy-filter).

Demonstrates pre-upstream privacy filtering for prompts and request logs.

Hooks:

- `gateway.request.afterBodyRead`
- `log.beforePersist`

Permissions:

- `request.body.read`
- `request.body.write`
- `log.redact`

Behavior:

- Redacts emails, Chinese mobile phone numbers, Chinese ID card patterns, Luhn-valid bank cards, and IPv4 addresses.
- Loads the upstream gitleaks-style rule set from `rules/gitleaks.toml`.
- Redacts known vendor secrets, contextual passwords/API keys, and high-entropy secret candidates.
- Uses span merging and false-positive mitigation for SSH command targets, paths, URLs, hashes, UUIDs, template variables, common placeholders, and business ID assignments.

Important limitation:

Like upstream, Privacy Filter is irreversible redaction. It does not restore original sensitive values into model responses after upstream processing.

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
