# Plugin Hooks

Hooks are stable extension points in the gateway and logging pipeline.

Request hooks:

- `gateway.request.received`
- `gateway.request.afterBodyRead`
- `gateway.request.beforeProviderResolution`
- `gateway.request.beforeSend`

Response hooks:

- `gateway.response.headers`
- `gateway.response.after`
- `gateway.response.chunk`

Other hooks:

- `gateway.error`
- `log.beforePersist`

`gateway.request.afterBodyRead` is the primary hook for prompt optimization because it runs after the request body is available. `gateway.response.chunk` receives bounded streaming chunks with a sliding window, not the complete response. `log.beforePersist` is for redaction before request logs are enqueued.
