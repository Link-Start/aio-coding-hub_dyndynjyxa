import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../generated/bindings";
import {
  type PluginDetail,
  pluginDisable,
  pluginEnable,
  pluginGet,
  pluginGrantPermissions,
  pluginInstallFromFile,
  pluginInstallRemote,
  pluginInstallOfficial,
  pluginList,
  pluginListAuditLogs,
  pluginParseMarketIndex,
  pluginQuarantineRevoked,
  pluginRevokePermission,
  pluginRollback,
  pluginSaveConfig,
  pluginUninstall,
  pluginUpdateFromFile,
} from "../plugins";

vi.mock("../../generated/bindings", () => ({
  commands: {
    pluginList: vi.fn(),
    pluginGet: vi.fn(),
    pluginInstallFromFile: vi.fn(),
    pluginInstallRemote: vi.fn(),
    pluginUpdateFromFile: vi.fn(),
    pluginRollback: vi.fn(),
    pluginParseMarketIndex: vi.fn(),
    pluginQuarantineRevoked: vi.fn(),
    pluginInstallOfficial: vi.fn(),
    pluginEnable: vi.fn(),
    pluginDisable: vi.fn(),
    pluginUninstall: vi.fn(),
    pluginSaveConfig: vi.fn(),
    pluginGrantPermissions: vi.fn(),
    pluginRevokePermission: vi.fn(),
    pluginListAuditLogs: vi.fn(),
  },
}));

vi.mock("../consoleLog", () => ({
  logToConsole: vi.fn(),
}));

function pluginSummary() {
  return {
    id: 1,
    plugin_id: "official.prompt-optimizer",
    name: "Prompt Optimizer",
    current_version: "1.0.0",
    status: "disabled" as const,
    runtime: "declarativeRules",
    permission_risk: "high" as const,
    update_available: false,
    last_error: null,
    created_at: 10,
    updated_at: 20,
  };
}

function pluginDetail(install_source: PluginDetail["install_source"] = "local"): PluginDetail {
  return {
    summary: pluginSummary(),
    manifest: {
      id: "official.prompt-optimizer",
      name: "Prompt Optimizer",
      version: "1.0.0",
      apiVersion: "1.0.0",
      runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
      hooks: [{ name: "gateway.request.afterBodyRead", priority: 100 }],
      permissions: ["request.body.read"],
      hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
    },
    install_source,
    installed_dir: null,
    config: {},
    granted_permissions: [],
    pending_permissions: [],
    audit_logs: [],
    runtime_failures: [],
  };
}

describe("services/plugins", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("wraps plugin list and detail IPC commands", async () => {
    vi.mocked(commands.pluginList).mockResolvedValue({ status: "ok", data: [pluginSummary()] });
    vi.mocked(commands.pluginGet).mockResolvedValue({
      status: "ok",
      data: pluginDetail("official"),
    });

    await expect(pluginList()).resolves.toHaveLength(1);
    await expect(pluginGet(" official.prompt-optimizer ")).resolves.toMatchObject({
      summary: { plugin_id: "official.prompt-optimizer" },
    });

    expect(commands.pluginGet).toHaveBeenCalledWith({ pluginId: "official.prompt-optimizer" });
  });

  it("normalizes mutation inputs before invoking generated commands", async () => {
    const detail = pluginDetail();
    vi.mocked(commands.pluginInstallFromFile).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginInstallRemote).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginUpdateFromFile).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginRollback).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginParseMarketIndex).mockResolvedValue({ status: "ok", data: [] });
    vi.mocked(commands.pluginInstallOfficial).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginQuarantineRevoked).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginEnable).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginDisable).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginUninstall).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginSaveConfig).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginGrantPermissions).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginRevokePermission).mockResolvedValue({ status: "ok", data: detail });
    vi.mocked(commands.pluginListAuditLogs).mockResolvedValue({ status: "ok", data: [] });

    await pluginInstallFromFile(" /tmp/plugin.json ");
    await pluginInstallRemote({
      pluginId: " community.remote ",
      downloadUrl: " https://github.com/acme/plugin/releases/download/v1/plugin.aio-plugin ",
      checksum: " sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ",
      signature: " signature ",
      publicKey: " public-key ",
      source: "github_release",
    });
    await pluginUpdateFromFile(" /tmp/plugin-update.aio-plugin ");
    await pluginRollback(" official.prompt-optimizer ", " 1.0.0 ");
    await pluginParseMarketIndex(
      ' {"plugins":[]} ',
      " https://plugins.example.test/index.json ",
      " sig "
    );
    await pluginInstallOfficial(" official.redactor ");
    await pluginQuarantineRevoked(" community.revoked ");
    await pluginEnable(" official.prompt-optimizer ");
    await pluginDisable(" official.prompt-optimizer ");
    await pluginUninstall(" official.prompt-optimizer ");
    await pluginSaveConfig(" official.prompt-optimizer ", { mode: "append_instruction" });
    await pluginGrantPermissions(" official.prompt-optimizer ", [
      " request.body.read ",
      "",
      "request.body.read",
      "request.body.write",
    ]);
    await pluginRevokePermission(" official.prompt-optimizer ", " request.body.write ");
    await pluginListAuditLogs({ pluginId: " official.prompt-optimizer ", limit: 9999 });

    expect(commands.pluginInstallFromFile).toHaveBeenCalledWith({ filePath: "/tmp/plugin.json" });
    expect(commands.pluginInstallRemote).toHaveBeenCalledWith({
      pluginId: "community.remote",
      downloadUrl: "https://github.com/acme/plugin/releases/download/v1/plugin.aio-plugin",
      checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      signature: "signature",
      publicKey: "public-key",
      source: "github_release",
    });
    expect(commands.pluginUpdateFromFile).toHaveBeenCalledWith({
      filePath: "/tmp/plugin-update.aio-plugin",
    });
    expect(commands.pluginRollback).toHaveBeenCalledWith({
      pluginId: "official.prompt-optimizer",
      version: "1.0.0",
    });
    expect(commands.pluginParseMarketIndex).toHaveBeenCalledWith({
      indexJson: '{"plugins":[]}',
      indexUrl: "https://plugins.example.test/index.json",
      signature: "sig",
    });
    expect(commands.pluginInstallOfficial).toHaveBeenCalledWith({ pluginId: "official.redactor" });
    expect(commands.pluginQuarantineRevoked).toHaveBeenCalledWith({
      pluginId: "community.revoked",
    });
    expect(commands.pluginEnable).toHaveBeenCalledWith({ pluginId: "official.prompt-optimizer" });
    expect(commands.pluginDisable).toHaveBeenCalledWith({ pluginId: "official.prompt-optimizer" });
    expect(commands.pluginUninstall).toHaveBeenCalledWith({
      pluginId: "official.prompt-optimizer",
    });
    expect(commands.pluginSaveConfig).toHaveBeenCalledWith({
      pluginId: "official.prompt-optimizer",
      config: { mode: "append_instruction" },
    });
    expect(commands.pluginGrantPermissions).toHaveBeenCalledWith({
      pluginId: "official.prompt-optimizer",
      permissions: ["request.body.read", "request.body.write"],
    });
    expect(commands.pluginRevokePermission).toHaveBeenCalledWith({
      pluginId: "official.prompt-optimizer",
      permission: "request.body.write",
    });
    expect(commands.pluginListAuditLogs).toHaveBeenCalledWith({
      pluginId: "official.prompt-optimizer",
      limit: 500,
    });
  });

  it("rejects empty plugin ids and file paths before IPC", async () => {
    await expect(pluginGet(" ")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(pluginInstallFromFile(" ")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(pluginInstallOfficial(" ")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      pluginInstallRemote({
        pluginId: "community.remote",
        downloadUrl: " ",
        checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    expect(commands.pluginGet).not.toHaveBeenCalled();
    expect(commands.pluginInstallFromFile).not.toHaveBeenCalled();
    expect(commands.pluginInstallOfficial).not.toHaveBeenCalled();
    expect(commands.pluginInstallRemote).not.toHaveBeenCalled();
  });
});
