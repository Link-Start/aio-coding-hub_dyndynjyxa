# Plugin Permissions

Permissions are explicit and risk-ranked. The host trims hook context before invoking plugins and rejects unauthorized writes.

Common permissions:

- `request.meta.read`: low risk.
- `request.header.read`: medium risk.
- `request.header.readSensitive`: high risk.
- `request.header.write`: high risk.
- `request.body.read`: high risk.
- `request.body.write`: high risk.
- `response.header.read`: low risk.
- `response.header.write`: medium risk.
- `response.body.read`: high risk.
- `response.body.write`: high risk.
- `stream.inspect`: high risk.
- `stream.modify`: high risk.
- `log.redact`: medium risk.

Reserved permissions for future host-mediated APIs:

- `plugin.storage`: medium risk.
- `network.fetch`: high risk.
- `file.read`: high risk.
- `file.write`: high risk.
- `secret.read`: critical risk.

Reserved permissions are rejected during manifest validation until the host implements those APIs.

High and critical permissions require clear user authorization. Plugin upgrades that add permissions require 重新授权 before the plugin can be enabled with those new capabilities.
