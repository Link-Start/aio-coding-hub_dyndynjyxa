import {
  type GatewayHookName,
  type PluginManifest,
  type PluginPermission,
  type PluginRuntime,
  permissionRisk,
  validateManifest,
} from "./index";

const manifest: PluginManifest = {
  id: "acme.redactor",
  name: "Redactor",
  version: "1.0.0",
  apiVersion: "1.0.0",
  runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
  hooks: [{ name: "gateway.request.afterBodyRead", priority: 10 }],
  permissions: ["request.body.read", "log.redact"],
  hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
};

const runtime: PluginRuntime = manifest.runtime;
const hook: GatewayHookName = manifest.hooks[0].name;
const permission: PluginPermission = "request.body.read";

if (runtime.kind !== "declarativeRules") {
  throw new Error("unexpected runtime");
}

if (hook !== "gateway.request.afterBodyRead") {
  throw new Error("unexpected hook");
}

if (permissionRisk(permission) !== "high") {
  throw new Error("unexpected risk");
}

const result = validateManifest(manifest);
if (!result.ok) {
  throw new Error(result.error.message);
}
