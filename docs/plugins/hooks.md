# Plugin Hooks

Hooks are stable extension points in the gateway and logging pipeline. Plugin API v1 keeps the active surface small and explicit so community plugins can reason about timing, permissions, and mutation behavior.

Default vNext hook timeout: 150 ms.
Default vNext failure policy: `fail-open`.

Reserved hooks are rejected during manifest validation until the host implements their call sites:

- `gateway.request.received`
- `gateway.request.beforeProviderResolution`
- `gateway.response.headers`

## Hook Matrix

| Hook | Phase | Read permissions | Write permissions | Mutation fields | Context fields |
| --- | --- | --- | --- | --- | --- |
| `gateway.request.afterBodyRead` | After request body read and before upstream provider send. | `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read` | `request.header.write`, `request.body.write` | `headers`, `requestBody` | `traceId`, `request.cliKey`, `request.method`, `request.path`, `request.query`, `request.headers`, `request.body`, `request.requestedModel`, `request.normalizedMessages` |
| `gateway.request.beforeSend` | After provider resolution and before upstream provider send. | `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read` | `request.header.write`, `request.body.write` | `headers`, `requestBody` | `traceId`, `request.cliKey`, `request.method`, `request.path`, `request.query`, `request.headers`, `request.body`, `request.requestedModel`, `request.normalizedMessages` |
| `gateway.response.chunk` | For each bounded streaming response chunk. | `stream.inspect` | `stream.modify` | `streamChunk` | `traceId`, `stream.sequence`, `stream.chunk` |
| `gateway.response.after` | After a complete non-streaming upstream response body is available. | `response.header.read`, `response.body.read` | `response.header.write`, `response.body.write` | `headers`, `responseBody` | `traceId`, `response.status`, `response.headers`, `response.body` |
| `gateway.error` | After gateway error response materialization and before it is sent. | `response.header.read`, `response.body.read` | `response.header.write`, `response.body.write` | `headers`, `responseBody` | `traceId`, `response.status`, `response.headers`, `response.body` |
| `log.beforePersist` | Before gateway request log persistence. | `log.redact` | `log.redact` | `logMessage` | `traceId`, `log.message` |

## gateway.request.afterBodyRead

- Phase: after request body read and before upstream provider send.
- Default timeout: 150 ms.
- Default failure policy: `fail-open`.
- Read permissions: `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read`.
- Write permissions: `request.header.write`, `request.body.write`.
- Mutation fields: `headers`, `requestBody`.
- Provider-neutral field: `request.normalizedMessages`.

Use this hook for prompt optimization, privacy filtering, and request-body checks. The host only includes `request.body` and `request.normalizedMessages` when the plugin has `request.body.read`.

Claude-style fixture:

```json
{
  "messages": [
    {
      "role": "user",
      "content": [{ "type": "text", "text": "hello claude" }]
    }
  ]
}
```

Codex/OpenAI Responses-style fixture:

```json
{
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [{ "type": "input_text", "text": "hello codex" }]
    }
  ]
}
```

The normalized context for both shapes includes entries like:

```json
{
  "request": {
    "normalizedMessages": [
      { "role": "user", "text": "hello codex", "source": "openai.responses.input_text" }
    ]
  }
}
```

## gateway.request.beforeSend

- Phase: after provider resolution and before upstream provider send.
- Default timeout: 150 ms.
- Default failure policy: `fail-open`.
- Read permissions: `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read`.
- Write permissions: `request.header.write`, `request.body.write`.
- Mutation fields: `headers`, `requestBody`.
- Provider-neutral field: `request.normalizedMessages`.

Runs after provider selection, auth/header preparation, request body sanitizers, and protocol rectifiers for the current attempt, immediately before the gateway sends bytes to the upstream provider. Use this hook when the plugin must guarantee final upstream request-body or request-header mutation.

This hook sees semantic decoded request body content. If a plugin mutates the body, the gateway updates the final upstream body and removes or recalculates wire-level length/encoding semantics as needed. Unchanged requests keep the original passthrough body where possible.

## gateway.response.chunk

- Phase: for each bounded streaming response chunk.
- Default timeout: 150 ms.
- Default failure policy: `fail-open`.
- Read permissions: `stream.inspect`.
- Write permissions: `stream.modify`.
- Mutation fields: `streamChunk`.
- Context fields: `traceId`, `stream.sequence`, `stream.chunk`.

This hook receives bounded streaming chunks, not the complete response. Plugins that need complete response bodies should use `gateway.response.after` for non-streaming requests.

## gateway.response.after

- Phase: after a complete non-streaming upstream response body is available.
- Default timeout: 150 ms.
- Default failure policy: `fail-open`.
- Read permissions: `response.header.read`, `response.body.read`.
- Write permissions: `response.header.write`, `response.body.write`.
- Mutation fields: `headers`, `responseBody`.
- Context fields: `traceId`, `response.status`, `response.headers`, `response.body`.

Use this hook for non-streaming response redaction, warnings, or response blocking.

## gateway.error

- Phase: after gateway error response materialization and before it is sent.
- Default timeout: 150 ms.
- Default failure policy: `fail-open`.
- Read permissions: `response.header.read`, `response.body.read`.
- Write permissions: `response.header.write`, `response.body.write`.
- Mutation fields: `headers`, `responseBody`.
- Context fields: `traceId`, `response.status`, `response.headers`, `response.body`.

Use this hook to redact or reshape gateway-generated error responses. It should not be used for provider success responses.

## log.beforePersist

- Phase: before gateway request log persistence.
- Default timeout: 150 ms.
- Default failure policy: `fail-open`.
- Read permissions: `log.redact`.
- Write permissions: `log.redact`.
- Mutation fields: `logMessage`.
- Context fields: `traceId`, `log.message`.

Use this hook for irreversible redaction before request logs are enqueued or written.
