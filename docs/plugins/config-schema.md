# Plugin Config Schema

Plugins may declare a `configSchema` JSON Schema subset in `plugin.json`.

Supported scalar types:

- `string`
- `number`
- `integer`
- `boolean`

Supported structured types:

- `array`
- `object`
- `enum`

Sensitive fields may use `password` style metadata. The GUI must not echo saved secret values as plain text. The backend validates config before persisting it; the frontend validation is only a convenience layer.
