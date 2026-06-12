import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PluginDetail, PluginSummary } from "../../services/plugins";
import {
  pluginDisable,
  pluginEnable,
  pluginGet,
  pluginInstallRemote,
  pluginInstallOfficial,
  pluginList,
  pluginQuarantineRevoked,
  pluginRevokePermission,
  pluginRollback,
  pluginSaveConfig,
  pluginUninstall,
  pluginUpdateFromFile,
} from "../../services/plugins";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { pluginKeys } from "../keys";
import {
  usePluginDisableMutation,
  usePluginEnableMutation,
  usePluginInstallOfficialMutation,
  usePluginInstallRemoteMutation,
  usePluginQuery,
  usePluginQuarantineRevokedMutation,
  usePluginRevokePermissionMutation,
  usePluginRollbackMutation,
  usePluginsListQuery,
  usePluginSaveConfigMutation,
  usePluginUninstallMutation,
  usePluginUpdateFromFileMutation,
} from "../plugins";

vi.mock("../../services/plugins", async () => {
  const actual =
    await vi.importActual<typeof import("../../services/plugins")>("../../services/plugins");
  return {
    ...actual,
    pluginList: vi.fn(),
    pluginGet: vi.fn(),
    pluginEnable: vi.fn(),
    pluginInstallRemote: vi.fn(),
    pluginInstallOfficial: vi.fn(),
    pluginQuarantineRevoked: vi.fn(),
    pluginUpdateFromFile: vi.fn(),
    pluginRollback: vi.fn(),
    pluginDisable: vi.fn(),
    pluginUninstall: vi.fn(),
    pluginSaveConfig: vi.fn(),
    pluginRevokePermission: vi.fn(),
  };
});

function summary(overrides: Partial<PluginSummary> = {}): PluginSummary {
  return {
    id: 1,
    plugin_id: "official.prompt-optimizer",
    name: "Prompt Optimizer",
    current_version: "1.0.0",
    status: "disabled",
    runtime: "declarativeRules",
    permission_risk: "high",
    update_available: false,
    last_error: null,
    created_at: 10,
    updated_at: 20,
    ...overrides,
  };
}

function detail(overrides: Partial<PluginDetail> = {}): PluginDetail {
  const baseSummary = summary();
  return {
    summary: baseSummary,
    manifest: {
      id: baseSummary.plugin_id,
      name: baseSummary.name,
      version: "1.0.0",
      apiVersion: "1.0.0",
      runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
      hooks: [{ name: "gateway.request.afterBodyRead", priority: 100 }],
      permissions: ["request.body.read"],
      hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
    },
    install_source: "official",
    installed_dir: null,
    config: {},
    granted_permissions: [],
    pending_permissions: [],
    audit_logs: [],
    runtime_failures: [],
    ...overrides,
  };
}

describe("query/plugins", () => {
  it("uses stable list and detail query keys", async () => {
    vi.mocked(pluginList).mockResolvedValue([summary()]);
    vi.mocked(pluginGet).mockResolvedValue(detail());
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => usePluginsListQuery(), { wrapper });
    renderHook(() => usePluginQuery(" official.prompt-optimizer "), { wrapper });

    await waitFor(() => {
      expect(pluginList).toHaveBeenCalled();
      expect(pluginGet).toHaveBeenCalledWith("official.prompt-optimizer");
    });

    expect(client.getQueryState(pluginKeys.list())).toBeTruthy();
    expect(client.getQueryState(pluginKeys.detail("official.prompt-optimizer"))).toBeTruthy();
  });

  it("invalidates list and detail queries after mutations", async () => {
    const next = detail({ summary: summary({ status: "enabled" }) });
    vi.mocked(pluginEnable).mockResolvedValue(next);
    vi.mocked(pluginInstallRemote).mockResolvedValue(next);
    vi.mocked(pluginInstallOfficial).mockResolvedValue(next);
    vi.mocked(pluginQuarantineRevoked).mockResolvedValue(next);
    vi.mocked(pluginUpdateFromFile).mockResolvedValue(next);
    vi.mocked(pluginRollback).mockResolvedValue(next);
    vi.mocked(pluginDisable).mockResolvedValue(next);
    vi.mocked(pluginUninstall).mockResolvedValue(next);
    vi.mocked(pluginSaveConfig).mockResolvedValue(next);
    vi.mocked(pluginRevokePermission).mockResolvedValue(next);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result: enableResult } = renderHook(() => usePluginEnableMutation(), { wrapper });
    const { result: installOfficialResult } = renderHook(() => usePluginInstallOfficialMutation(), {
      wrapper,
    });
    const { result: installRemoteResult } = renderHook(() => usePluginInstallRemoteMutation(), {
      wrapper,
    });
    const { result: quarantineRevokedResult } = renderHook(
      () => usePluginQuarantineRevokedMutation(),
      {
        wrapper,
      }
    );
    const { result: disableResult } = renderHook(() => usePluginDisableMutation(), { wrapper });
    const { result: uninstallResult } = renderHook(() => usePluginUninstallMutation(), { wrapper });
    const { result: updateResult } = renderHook(() => usePluginUpdateFromFileMutation(), {
      wrapper,
    });
    const { result: rollbackResult } = renderHook(() => usePluginRollbackMutation(), { wrapper });
    const { result: saveConfigResult } = renderHook(() => usePluginSaveConfigMutation(), {
      wrapper,
    });
    const { result: revokePermissionResult } = renderHook(
      () => usePluginRevokePermissionMutation(),
      {
        wrapper,
      }
    );

    await act(async () => {
      await enableResult.current.mutateAsync("official.prompt-optimizer");
      await installRemoteResult.current.mutateAsync({
        pluginId: "official.prompt-optimizer",
        downloadUrl: "https://github.com/acme/plugin/releases/download/v1/plugin.aio-plugin",
        checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      });
      await installOfficialResult.current.mutateAsync("official.prompt-optimizer");
      await quarantineRevokedResult.current.mutateAsync("official.prompt-optimizer");
      await disableResult.current.mutateAsync("official.prompt-optimizer");
      await uninstallResult.current.mutateAsync("official.prompt-optimizer");
      await updateResult.current.mutateAsync("/tmp/plugin-update.aio-plugin");
      await rollbackResult.current.mutateAsync({
        pluginId: "official.prompt-optimizer",
        version: "1.0.0",
      });
      await saveConfigResult.current.mutateAsync({
        pluginId: "official.prompt-optimizer",
        config: { mode: "append_instruction" },
      });
      await revokePermissionResult.current.mutateAsync({
        pluginId: "official.prompt-optimizer",
        permission: "request.body.write",
      });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: pluginKeys.list() });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: pluginKeys.detail("official.prompt-optimizer"),
    });
  });
});
