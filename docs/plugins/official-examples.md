# Official Example Plugin

The official catalog intentionally stays small. `official.privacy-filter` is the only bundled official plugin.

This keeps the trusted host surface narrow while the open extension surface remains available through `declarativeRules`, WASM, and the disabled-by-default process runtime proof of concept.

## Current Official ID

- `official.privacy-filter`

It is available from the Plugins page through the official plugin install button.

## Privacy Filter

ID: `official.privacy-filter`

Runtime: `native:privacyFilter`

Aligned with the core redaction behavior of [packyme/privacy-filter](https://github.com/packyme/privacy-filter).

Demonstrates pre-upstream privacy filtering for prompts and request logs.

It also demonstrates schema-driven configuration UI. The host renders its switches, select fields, and sensitive-type checkbox group from `configSchema` plus `x-aio-ui` metadata. It does not require a host-side plugin-specific page component.

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

Provider request shapes:

`official.privacy-filter` redacts matching string values anywhere inside JSON request bodies. It also supports raw text bodies. For Codex/OpenAI Responses payloads, `input[].content[].text` and `input_text` content are covered because the engine walks every JSON string value before upstream send. Claude-style `messages[].content[].text` and OpenAI-compatible chat `messages[].content` strings are covered by the same recursive JSON-string walk.

Important limitation:

Like upstream, Privacy Filter is irreversible redaction. It does not restore original sensitive values into model responses after upstream processing.

## Official-Style Example Checklist

An official-style example must include:

- a minimal manifest;
- a fixture for Claude messages;
- a fixture for Codex/OpenAI Responses input;
- a local replay command;
- a package command;
- the exact permissions it requests;
- a short explanation of what is intentionally unsupported.

For community examples, prefer `declarativeRules` unless the behavior requires deterministic code execution that cannot fit the rule runtime. WASM examples may demonstrate ABI packaging, but gateway execution remains policy-gated until the host enables it.

## Retired Built-In Examples

Earlier drafts included built-in prompt optimizer, safety detector, and generic redactor examples. They are no longer bundled official plugins.

Similar behavior should be implemented as community plugins:

- Prompt rewriting: `declarativeRules` on `gateway.request.afterBodyRead`.
- Response safety checks: `declarativeRules` on `gateway.response.after` or `gateway.response.chunk`.
- Generic log redaction: `declarativeRules` on `log.beforePersist`, or WASM when the rule runtime is not expressive enough.

## Where It Lives

The official plugin fixture is stored in the host repository:

```text
src-tauri/resources/plugins/official/privacy-filter/
```

The host registers it in:

```text
src-tauri/src/app/plugins/official.rs
```

The fixture remains in this repository while plugin API v1 is stabilizing. After the API is stable, the SDK, scaffolder, and community examples can move to dedicated repositories.
