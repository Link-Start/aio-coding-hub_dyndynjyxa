# Streaming Plugins

Streaming plugins use `gateway.response.chunk`.

The runtime provides:

- current chunk bytes or text;
- a bounded sliding window for cross-chunk detection;
- trace metadata;
- permission-trimmed context.

Without `stream.inspect`, the plugin cannot inspect stream contents. Without `stream.modify`, the plugin cannot replace or block chunks.

Stream plugins must not assume access to the full response. They should detect bounded patterns and return pass, warn, replace, or block according to the granted permissions.
