import {
  createHash,
  createPrivateKey,
  createPublicKey,
  generateKeyPairSync,
  sign,
  verify,
} from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { GatewayHookName, PluginManifest, ValidationResult } from "@aio-coding-hub/plugin-sdk";
import { validateManifest } from "@aio-coding-hub/plugin-sdk";
import { createPluginScaffold, type ScaffoldFiles, type ScaffoldTemplate } from "./scaffold";

export type PackedPlugin = {
  bytes: Uint8Array;
  checksum: string;
};

export type SigningKeyPair = {
  privateKey: string;
  publicKey: string;
};

export type CliIo = {
  log: (line: string) => void;
  error: (line: string) => void;
};

const USAGE = "Usage: create-aio-plugin <publisher.plugin-name> [rule|wasm]";

export function runCreateAioPluginCli(args: string[], cwd: string, io: CliIo = console): number {
  const [commandOrId, idOrTemplate, maybeTemplate, maybePublicKey] = args;

  if (!commandOrId) {
    io.error(USAGE);
    return 1;
  }

  if (commandOrId === "validate" || commandOrId === "replay" || commandOrId === "pack") {
    const sample = createPluginScaffold({
      id: idOrTemplate ?? "acme.sample",
      name: titleFromId(idOrTemplate ?? "acme.sample"),
      template: "rule",
    });
    if (commandOrId === "validate") {
      io.log(JSON.stringify(validatePluginFiles(sample)));
    } else if (commandOrId === "replay") {
      io.log(JSON.stringify(replayHook(sample, "gateway.request.afterBodyRead", {})));
    } else {
      const packed = packPlugin(sample);
      const outputPath = join(cwd, `${idOrTemplate ?? "acme.sample"}.aio-plugin`);
      writeFileSync(outputPath, packed.bytes);
      io.log(
        JSON.stringify({
          path: outputPath,
          checksum: packed.checksum,
          sizeBytes: packed.bytes.length,
        })
      );
    }
    return 0;
  }

  if (commandOrId === "sign") {
    const bytes = new TextEncoder().encode(idOrTemplate ?? "");
    const keyPair = maybeTemplate
      ? { privateKey: maybeTemplate, publicKey: createPublicKeyFromPrivateKey(maybeTemplate) }
      : generateSigningKeyPair();
    io.log(JSON.stringify(signPackage(bytes, keyPair.privateKey, keyPair.publicKey)));
    return 0;
  }

  if (commandOrId === "verify") {
    if (!maybeTemplate || !maybePublicKey) {
      io.error("Usage: create-aio-plugin verify <bytes> <signature> <publicKey>");
      return 1;
    }
    const bytes = new TextEncoder().encode(idOrTemplate ?? "");
    io.log(JSON.stringify(verifyPackage(bytes, maybeTemplate, maybePublicKey)));
    return 0;
  }

  const idArg = commandOrId;
  const template = (idOrTemplate ?? "rule") as ScaffoldTemplate;
  const files = createPluginScaffold({
    id: idArg,
    name: titleFromId(idArg),
    template,
  });

  for (const [path, content] of Object.entries(files)) {
    const fullPath = join(cwd, idArg, path);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, content);
  }
  return 0;
}

export function validatePluginFiles(files: ScaffoldFiles): ValidationResult {
  const manifestText = files["plugin.json"];
  if (!manifestText) {
    return {
      ok: false,
      error: { code: "PLUGIN_MISSING_MANIFEST", message: "missing plugin.json" },
    };
  }
  try {
    return validateManifest(JSON.parse(manifestText) as PluginManifest);
  } catch (error) {
    return {
      ok: false,
      error: {
        code: "PLUGIN_INVALID_MANIFEST",
        message: error instanceof Error ? error.message : "invalid manifest",
      },
    };
  }
}

export function replayHook(files: ScaffoldFiles, hook: GatewayHookName, context: unknown): unknown {
  const validation = validatePluginFiles(files);
  if (!validation.ok) {
    throw new Error(`${validation.error.code}: ${validation.error.message}`);
  }
  return {
    action: "pass",
    hook,
    contextPreview: typeof context === "string" ? context.slice(0, 64) : undefined,
  };
}

export function packPlugin(files: ScaffoldFiles): PackedPlugin {
  const bytes = createStoredZipBytes(
    Object.entries(files)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([path, content]) => [path, new TextEncoder().encode(content)] as const)
  );
  return {
    bytes,
    checksum: sha256(bytes),
  };
}

export function generateSigningKeyPair(): SigningKeyPair {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  return {
    privateKey: privateKey.export({ format: "der", type: "pkcs8" }).toString("base64"),
    publicKey: rawPublicKeyFromSpki(
      publicKey.export({ format: "der", type: "spki" }) as Buffer
    ).toString("base64"),
  };
}

export function signPackage(
  bytes: Uint8Array,
  privateKey: string,
  publicKey?: string
): { checksum: string; signature: string; publicKey: string } {
  const checksum = sha256(bytes);
  const key = createPrivateKey({
    key: Buffer.from(privateKey, "base64"),
    format: "der",
    type: "pkcs8",
  });
  const signature = sign(null, Buffer.from(bytes), key).toString("base64");
  return {
    checksum,
    signature,
    publicKey:
      publicKey ??
      rawPublicKeyFromSpki(
        createPublicKey(key).export({ format: "der", type: "spki" }) as Buffer
      ).toString("base64"),
  };
}

export function verifyPackage(
  bytes: Uint8Array,
  signature: string,
  publicKey: string
): { ok: boolean; checksum: string } {
  const key = createPublicKey({
    key: spkiFromRawPublicKey(Buffer.from(publicKey, "base64")),
    format: "der",
    type: "spki",
  });
  return {
    ok: verify(null, Buffer.from(bytes), key, Buffer.from(signature, "base64")),
    checksum: sha256(bytes),
  };
}

function createPublicKeyFromPrivateKey(privateKey: string): string {
  return signPackage(new Uint8Array(), privateKey).publicKey;
}

function titleFromId(id: string): string {
  const segments = id.split(".");
  const slug = segments[segments.length - 1] ?? id;
  return slug
    .split("-")
    .map((part: string) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(" ");
}

function sha256(bytes: Uint8Array): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function createStoredZipBytes(entries: readonly (readonly [string, Uint8Array])[]): Uint8Array {
  const chunks: Uint8Array[] = [];
  const centralDirectory: Uint8Array[] = [];
  let offset = 0;

  for (const [path, data] of entries) {
    const name = new TextEncoder().encode(path.replace(/\\/g, "/"));
    const crc = crc32(data);
    const localHeader = concatBytes([
      u32(0x04034b50),
      u16(20),
      u16(0x0800),
      u16(0),
      u16(0),
      u16(0),
      u32(crc),
      u32(data.length),
      u32(data.length),
      u16(name.length),
      u16(0),
      name,
    ]);
    chunks.push(localHeader, data);

    centralDirectory.push(
      concatBytes([
        u32(0x02014b50),
        u16(20),
        u16(20),
        u16(0x0800),
        u16(0),
        u16(0),
        u16(0),
        u32(crc),
        u32(data.length),
        u32(data.length),
        u16(name.length),
        u16(0),
        u16(0),
        u16(0),
        u16(0),
        u32(0),
        u32(offset),
        name,
      ])
    );
    offset += localHeader.length + data.length;
  }

  const centralDirectoryOffset = offset;
  const centralDirectoryBytes = concatBytes(centralDirectory);
  const endOfCentralDirectory = concatBytes([
    u32(0x06054b50),
    u16(0),
    u16(0),
    u16(entries.length),
    u16(entries.length),
    u32(centralDirectoryBytes.length),
    u32(centralDirectoryOffset),
    u16(0),
  ]);

  return concatBytes([...chunks, centralDirectoryBytes, endOfCentralDirectory]);
}

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function concatBytes(chunks: readonly Uint8Array[]): Uint8Array {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const output = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.length;
  }
  return output;
}

function u16(value: number): Uint8Array {
  return new Uint8Array([value & 0xff, (value >>> 8) & 0xff]);
}

function u32(value: number): Uint8Array {
  return new Uint8Array([
    value & 0xff,
    (value >>> 8) & 0xff,
    (value >>> 16) & 0xff,
    (value >>> 24) & 0xff,
  ]);
}

function rawPublicKeyFromSpki(spki: Buffer): Buffer {
  const prefix = Buffer.from("302a300506032b6570032100", "hex");
  if (spki.length !== prefix.length + 32 || !spki.subarray(0, prefix.length).equals(prefix)) {
    throw new Error("Unsupported Ed25519 SPKI public key format");
  }
  return spki.subarray(prefix.length);
}

function spkiFromRawPublicKey(raw: Buffer): Buffer {
  if (raw.length !== 32) {
    throw new Error("Ed25519 public key must be 32 bytes");
  }
  return Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), raw]);
}
