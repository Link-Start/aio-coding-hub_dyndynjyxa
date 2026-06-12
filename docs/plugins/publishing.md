# Plugin Publishing

The package format is `.aio-plugin`, a zip archive with `plugin.json` at the root or inside one top-level directory.

Publishing checklist:

- Validate `plugin.json`.
- Keep package size and entry count small.
- Compute `sha256` over the package bytes.
- Sign release metadata with Ed25519 when publishing through a trusted index.
- Include rollback notes for breaking updates.

The current implementation supports local/offline package import, constrained remote `.aio-plugin` download, checksum/signature verification, update permission deltas, revoked-plugin quarantine, and rollback snapshots.

Remote package installation is intentionally narrow:

- download URLs must be `https://` or `file://` without credentials;
- artifact paths must end in `.aio-plugin`;
- packages are size-limited before extraction;
- checksum is mandatory for remote and GitHub release installs;
- Ed25519 signatures are verified when a signature and trusted public key are provided.

Developer tooling emits Ed25519 signatures as base64. Public keys are raw 32-byte Ed25519 public keys encoded as base64, matching the host verifier input.
