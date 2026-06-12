export type PluginPermissionRisk = "low" | "medium" | "high" | "critical";

export type GatewayHookName =
  | "gateway.request.received"
  | "gateway.request.afterBodyRead"
  | "gateway.request.beforeProviderResolution"
  | "gateway.request.beforeSend"
  | "gateway.response.headers"
  | "gateway.response.chunk"
  | "gateway.response.after"
  | "gateway.error"
  | "log.beforePersist";

export type PluginPermission =
  | "request.meta.read"
  | "request.header.read"
  | "request.header.readSensitive"
  | "request.header.write"
  | "request.body.read"
  | "request.body.write"
  | "response.header.read"
  | "response.header.write"
  | "response.body.read"
  | "response.body.write"
  | "stream.inspect"
  | "stream.modify"
  | "log.redact"
  | "plugin.storage"
  | "network.fetch"
  | "file.read"
  | "file.write"
  | "secret.read";

export type PluginRuntime =
  | { kind: "declarativeRules"; rules: string[] }
  | { kind: "wasm"; abiVersion: string; memoryLimitBytes?: number };

export type PluginHook = {
  name: GatewayHookName;
  priority?: number;
  failurePolicy?: "fail-open" | "fail-closed";
};

export type PluginHostCompatibility = {
  app: string;
  pluginApi: string;
  platforms?: string[];
};

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue | undefined };

export type PluginManifest = {
  id: string;
  name: string;
  version: string;
  apiVersion: string;
  runtime: PluginRuntime;
  hooks: PluginHook[];
  permissions: PluginPermission[];
  hostCompatibility: PluginHostCompatibility;
  entry?: string;
  configSchema?: JsonValue;
  configVersion?: number;
  description?: string;
  author?: JsonValue;
  homepage?: string;
  repository?: JsonValue;
  license?: string;
  checksum?: string;
  signature?: string;
  category?: string;
};

export type PluginHookContext = {
  hook: GatewayHookName;
  traceId?: string;
  config: JsonValue;
  context: JsonValue;
};

export type PluginHookResult =
  | { action: "pass"; audit?: JsonValue[] }
  | { action: "warn"; message: string; audit?: JsonValue[] }
  | { action: "block"; reason: string; audit?: JsonValue[] }
  | { action: "replace"; contextPatch: JsonValue; audit?: JsonValue[] };

export type ValidationResult =
  | { ok: true }
  | { ok: false; error: { code: string; message: string } };

const PERMISSION_RISKS: Record<PluginPermission, PluginPermissionRisk> = {
  "request.meta.read": "low",
  "request.header.read": "medium",
  "request.header.readSensitive": "high",
  "request.header.write": "high",
  "request.body.read": "high",
  "request.body.write": "high",
  "response.header.read": "medium",
  "response.header.write": "high",
  "response.body.read": "high",
  "response.body.write": "high",
  "stream.inspect": "high",
  "stream.modify": "high",
  "log.redact": "medium",
  "plugin.storage": "medium",
  "network.fetch": "high",
  "file.read": "critical",
  "file.write": "critical",
  "secret.read": "critical",
};

const KNOWN_HOOKS = new Set<GatewayHookName>([
  "gateway.request.received",
  "gateway.request.afterBodyRead",
  "gateway.request.beforeProviderResolution",
  "gateway.request.beforeSend",
  "gateway.response.headers",
  "gateway.response.chunk",
  "gateway.response.after",
  "gateway.error",
  "log.beforePersist",
]);

const KNOWN_PERMISSIONS = new Set<PluginPermission>(
  Object.keys(PERMISSION_RISKS) as PluginPermission[]
);

export function permissionRisk(permission: PluginPermission): PluginPermissionRisk {
  return PERMISSION_RISKS[permission];
}

export function validateManifest(manifest: PluginManifest): ValidationResult {
  if (!/^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*)+$/.test(manifest.id)) {
    return invalid("PLUGIN_INVALID_ID", "plugin id must look like publisher.plugin-name");
  }
  if (!isSemver(manifest.version) || !isSemver(manifest.apiVersion)) {
    return invalid("PLUGIN_INVALID_VERSION", "version and apiVersion must be SemVer");
  }
  if (manifest.runtime.kind === "declarativeRules" && manifest.runtime.rules.length === 0) {
    return invalid("PLUGIN_INVALID_RUNTIME", "declarativeRules runtime requires rules");
  }
  if (manifest.runtime.kind === "wasm" && !isSemver(manifest.runtime.abiVersion)) {
    return invalid("PLUGIN_INVALID_RUNTIME", "wasm runtime requires SemVer abiVersion");
  }
  if (manifest.hooks.length === 0) {
    return invalid("PLUGIN_MISSING_HOOKS", "plugin must declare at least one hook");
  }
  for (const hook of manifest.hooks) {
    if (!KNOWN_HOOKS.has(hook.name)) {
      return invalid("PLUGIN_UNKNOWN_HOOK", `unknown hook: ${hook.name}`);
    }
  }
  for (const permission of manifest.permissions) {
    if (!KNOWN_PERMISSIONS.has(permission)) {
      return invalid("PLUGIN_UNKNOWN_PERMISSION", `unknown permission: ${permission}`);
    }
  }
  return { ok: true };
}

function invalid(code: string, message: string): ValidationResult {
  return { ok: false, error: { code, message } };
}

function isSemver(value: string): boolean {
  return /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/.test(value);
}
