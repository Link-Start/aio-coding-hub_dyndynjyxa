import { createPublicKey, verify } from "node:crypto";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { createPluginScaffold } from "./scaffold";
import {
  generateSigningKeyPair,
  packPlugin,
  replayHook,
  runCreateAioPluginCli,
  signPackage,
  validatePluginFiles,
  verifyPackage,
} from "./devtools";

describe("create-aio-plugin scaffold", () => {
  it("creates a declarative rule plugin template", () => {
    const files = createPluginScaffold({
      id: "acme.redactor",
      name: "Redactor",
      template: "rule",
    });

    expect(files["plugin.json"]).toContain('"id": "acme.redactor"');
    expect(files["plugin.json"]).toContain('"kind": "declarativeRules"');
    expect(files["rules/main.json"]).toContain('"kind": "replace"');
    expect(files["README.md"]).toContain("acme.redactor");
  });

  it("creates a WASM plugin template without enabling marketplace execution", () => {
    const files = createPluginScaffold({
      id: "acme.policy",
      name: "Policy",
      template: "wasm",
    });

    expect(files["plugin.json"]).toContain('"kind": "wasm"');
    expect(files["src/lib.rs"]).toContain("aio_plugin_handle");
    expect(files["README.md"]).toContain("disabled by default");
  });

  it("validates manifests, replays hook fixtures, and verifies package signatures", () => {
    const files = createPluginScaffold({
      id: "acme.redactor",
      name: "Redactor",
      template: "rule",
    });

    expect(validatePluginFiles(files).ok).toBe(true);
    expect(replayHook(files, "gateway.request.afterBodyRead", { body: "SECRET_TOKEN" })).toEqual({
      action: "pass",
      hook: "gateway.request.afterBodyRead",
    });

    const packed = packPlugin(files);
    const entries = unpackStoredZipEntries(packed.bytes);

    expect(entries.get("plugin.json")).toContain('"id": "acme.redactor"');
    expect(entries.get("rules/main.json")).toContain('"kind": "replace"');

    const keyPair = generateSigningKeyPair();
    const signed = signPackage(packed.bytes, keyPair.privateKey);

    expect(signed.publicKey).toBe(keyPair.publicKey);
    expect(Buffer.from(signed.publicKey, "base64")).toHaveLength(32);
    expect(
      verify(
        null,
        Buffer.from(packed.bytes),
        createPublicKey({
          key: Buffer.concat([
            Buffer.from("302a300506032b6570032100", "hex"),
            Buffer.from(keyPair.publicKey, "base64"),
          ]),
          format: "der",
          type: "spki",
        }),
        Buffer.from(signed.signature, "base64")
      )
    ).toBe(true);
    expect(verifyPackage(packed.bytes, signed.signature, keyPair.publicKey)).toMatchObject({
      ok: true,
      checksum: packed.checksum,
    });
    expect(
      verifyPackage(new TextEncoder().encode("tampered"), signed.signature, keyPair.publicKey)
    ).toMatchObject({
      ok: false,
    });
  });

  it("signs and verifies package bytes through the CLI helper", () => {
    const keyPair = generateSigningKeyPair();
    const signedOutput: string[] = [];
    const verifyOutput: string[] = [];

    expect(
      runCreateAioPluginCli(["sign", "package-bytes", keyPair.privateKey], process.cwd(), {
        log: (line) => signedOutput.push(line),
        error: () => undefined,
      })
    ).toBe(0);
    const signed = JSON.parse(signedOutput[0] ?? "{}") as {
      checksum: string;
      signature: string;
      publicKey: string;
    };

    expect(
      runCreateAioPluginCli(
        ["verify", "package-bytes", signed.signature, signed.publicKey],
        process.cwd(),
        {
          log: (line) => verifyOutput.push(line),
          error: () => undefined,
        }
      )
    ).toBe(0);

    expect(JSON.parse(verifyOutput[0] ?? "{}")).toMatchObject({
      ok: true,
      checksum: signed.checksum,
    });
  });

  it("packs a scaffold into an .aio-plugin file through the CLI helper", () => {
    const cwd = mkdtempSync(join(tmpdir(), "aio-plugin-pack-"));
    const output: string[] = [];

    expect(
      runCreateAioPluginCli(["pack", "acme.redactor"], cwd, {
        log: (line) => output.push(line),
        error: () => undefined,
      })
    ).toBe(0);

    const result = JSON.parse(output[0] ?? "{}") as {
      path: string;
      checksum: string;
      sizeBytes: number;
    };

    expect(result.path).toBe(join(cwd, "acme.redactor.aio-plugin"));
    expect(result.checksum).toMatch(/^sha256:[a-f0-9]{64}$/);
    expect(result.sizeBytes).toBeGreaterThan(0);
    expect(existsSync(result.path)).toBe(true);
  });
});

function unpackStoredZipEntries(bytes: Uint8Array): Map<string, string> {
  const entries = new Map<string, string>();
  let offset = 0;
  while (offset + 30 <= bytes.length) {
    const signature = readU32(bytes, offset);
    if (signature !== 0x04034b50) break;
    const compression = readU16(bytes, offset + 8);
    const compressedSize = readU32(bytes, offset + 18);
    const nameLength = readU16(bytes, offset + 26);
    const extraLength = readU16(bytes, offset + 28);
    expect(compression).toBe(0);
    const nameStart = offset + 30;
    const dataStart = nameStart + nameLength + extraLength;
    const name = new TextDecoder().decode(bytes.subarray(nameStart, nameStart + nameLength));
    const data = bytes.subarray(dataStart, dataStart + compressedSize);
    entries.set(name, new TextDecoder().decode(data));
    offset = dataStart + compressedSize;
  }
  return entries;
}

function readU16(bytes: Uint8Array, offset: number): number {
  return bytes[offset] | ((bytes[offset + 1] ?? 0) << 8);
}

function readU32(bytes: Uint8Array, offset: number): number {
  return (
    ((bytes[offset] ?? 0) |
      ((bytes[offset + 1] ?? 0) << 8) |
      ((bytes[offset + 2] ?? 0) << 16) |
      ((bytes[offset + 3] ?? 0) << 24)) >>>
    0
  );
}
