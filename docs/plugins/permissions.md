# Plugin Permissions

Permissions are explicit and risk-ranked. The host trims hook context before invoking plugins and rejects unauthorized writes.

Common permissions:

- `request.meta.read`: low risk.
- `request.body.read`: high risk.
- `request.body.write`: high risk.
- `response.body.read`: high risk.
- `stream.inspect`: high risk.
- `stream.modify`: high risk.
- `log.redact`: medium risk.
- `secret.read`: critical risk.

High and critical permissions require clear user authorization. Plugin upgrades that add permissions require 重新授权 before the plugin can be enabled with those new capabilities.
