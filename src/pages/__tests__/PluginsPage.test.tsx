import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { PluginsPage } from "../PluginsPage";
import type { PluginDetail, PluginSummary } from "../../services/plugins";
import { openDesktopSinglePath } from "../../services/desktop/dialog";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import {
  usePluginDisableMutation,
  usePluginEnableMutation,
  usePluginInstallFromFileMutation,
  usePluginInstallOfficialMutation,
  usePluginQuery,
  usePluginRollbackMutation,
  usePluginUpdateFromFileMutation,
  usePluginsListQuery,
  usePluginUninstallMutation,
} from "../../query/plugins";

vi.mock("sonner", () => {
  const toast = Object.assign(vi.fn(), {
    loading: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  });
  return { toast };
});

vi.mock("../../services/desktop/dialog", async () => {
  const actual = await vi.importActual<typeof import("../../services/desktop/dialog")>(
    "../../services/desktop/dialog"
  );
  return { ...actual, openDesktopSinglePath: vi.fn() };
});

vi.mock("../../query/plugins", async () => {
  const actual = await vi.importActual<typeof import("../../query/plugins")>("../../query/plugins");
  return {
    ...actual,
    usePluginsListQuery: vi.fn(),
    usePluginQuery: vi.fn(),
    usePluginInstallFromFileMutation: vi.fn(),
    usePluginInstallOfficialMutation: vi.fn(),
    usePluginUpdateFromFileMutation: vi.fn(),
    usePluginRollbackMutation: vi.fn(),
    usePluginEnableMutation: vi.fn(),
    usePluginDisableMutation: vi.fn(),
    usePluginUninstallMutation: vi.fn(),
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
      hooks: [{ name: "gateway.request.afterBodyRead", priority: 100, failurePolicy: "fail-open" }],
      permissions: ["request.body.read", "request.body.write"],
      hostCompatibility: {
        app: ">=0.56.0 <1.0.0",
        pluginApi: "^1.0.0",
        platforms: ["macos", "windows", "linux"],
      },
      configSchema: {
        type: "object",
        required: ["mode"],
        properties: {
          mode: { type: "string", enum: ["append_instruction", "rewrite_system_message"] },
        },
      },
    },
    install_source: "official",
    installed_dir: null,
    config: { mode: "append_instruction" },
    granted_permissions: ["request.body.read"],
    pending_permissions: ["request.body.write"],
    audit_logs: [
      {
        id: 1,
        plugin_id: baseSummary.plugin_id,
        trace_id: "trace-1",
        event_type: "plugin.installed",
        risk_level: "low",
        message: "Plugin installed",
        details: {},
        created_at: 30,
      },
    ],
    runtime_failures: [],
    ...overrides,
  };
}

function mutation(overrides: Record<string, unknown> = {}) {
  return {
    mutateAsync: vi.fn().mockResolvedValue(detail()),
    isPending: false,
    ...overrides,
  };
}

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

describe("pages/PluginsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(usePluginInstallFromFileMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginInstallOfficialMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginUpdateFromFileMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginRollbackMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginEnableMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginDisableMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginUninstallMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginQuery).mockReturnValue({
      data: detail(),
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
  });

  it("renders list fields and plugin detail permissions", () => {
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary({ update_available: true, last_error: "Last failure" })],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getAllByText("Prompt Optimizer").length).toBeGreaterThan(0);
    expect(screen.getAllByText("official.prompt-optimizer").length).toBeGreaterThan(0);
    expect(screen.getByText("declarativeRules")).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
    expect(screen.getByText("可更新")).toBeInTheDocument();
    expect(screen.getByText("Last failure")).toBeInTheDocument();
    expect(screen.getByText("gateway.request.afterBodyRead")).toBeInTheDocument();
    expect(screen.getByText("request.body.write")).toBeInTheDocument();
    expect(screen.getByText("未授权")).toBeInTheDocument();
    expect(screen.getByText("Plugin installed")).toBeInTheDocument();
  });

  it("shows empty and error states", () => {
    vi.mocked(usePluginsListQuery).mockReturnValueOnce({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    const { rerender } = renderWithProviders(<PluginsPage />);
    expect(screen.getByText("还没有安装插件")).toBeInTheDocument();

    vi.mocked(usePluginsListQuery).mockReturnValueOnce({
      data: null,
      isLoading: false,
      isFetching: false,
      error: new Error("boom"),
    } as any);
    rerender(
      <QueryClientProvider client={createTestQueryClient()}>
        <MemoryRouter>
          <PluginsPage />
        </MemoryRouter>
      </QueryClientProvider>
    );
    expect(screen.getByText(/插件列表加载失败/)).toBeInTheDocument();
  });

  it("wires import and enable actions", async () => {
    const importMutation = mutation();
    const installOfficialMutation = mutation();
    const enableMutation = mutation();
    vi.mocked(usePluginInstallFromFileMutation).mockReturnValue(importMutation as any);
    vi.mocked(usePluginInstallOfficialMutation).mockReturnValue(installOfficialMutation as any);
    vi.mocked(usePluginEnableMutation).mockReturnValue(enableMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary()],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(openDesktopSinglePath).mockResolvedValue("/tmp/plugin.json");

    renderWithProviders(<PluginsPage />);
    fireEvent.click(screen.getByRole("button", { name: "本地导入" }));
    expect(screen.getByRole("button", { name: /Privacy Filter/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Safety Detector/ }));
    fireEvent.click(screen.getByRole("button", { name: "启用" }));

    await waitFor(() => {
      expect(importMutation.mutateAsync).toHaveBeenCalledWith("/tmp/plugin.json");
      expect(installOfficialMutation.mutateAsync).toHaveBeenCalledWith("official.safety-detector");
      expect(enableMutation.mutateAsync).toHaveBeenCalledWith("official.prompt-optimizer");
      expect(toast.success).toHaveBeenCalled();
    });
  });

  it("shows package risk labels and wires update/rollback actions", async () => {
    const updateMutation = mutation();
    const rollbackMutation = mutation();
    vi.mocked(usePluginUpdateFromFileMutation).mockReturnValue(updateMutation as any);
    vi.mocked(usePluginRollbackMutation).mockReturnValue(rollbackMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [
        summary({
          plugin_id: "community.redactor",
          name: "Community Redactor",
          status: "update_available",
          update_available: true,
          permission_risk: "critical",
        }),
        summary({
          plugin_id: "community.revoked",
          name: "Revoked Plugin",
          status: "quarantined",
          update_available: false,
          last_error: "revoked by market",
        }),
      ],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(usePluginQuery).mockReturnValue({
      data: detail({
        summary: summary({
          plugin_id: "community.redactor",
          name: "Community Redactor",
          current_version: "1.1.0",
          status: "update_available",
          permission_risk: "critical",
          update_available: true,
        }),
        install_source: "offline",
        audit_logs: [
          {
            id: 2,
            plugin_id: "community.redactor",
            trace_id: null,
            event_type: "plugin.installed",
            risk_level: "high",
            message: "Local plugin package installed",
            details: { unsigned: true, fromVersion: "1.0.0" },
            created_at: 40,
          },
        ],
      }),
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(openDesktopSinglePath).mockResolvedValue("/tmp/community-redactor-1.1.0.aio-plugin");

    renderWithProviders(<PluginsPage />);
    fireEvent.click(screen.getAllByText("Community Redactor")[0]);
    fireEvent.click(screen.getByRole("button", { name: "更新" }));
    fireEvent.click(screen.getByRole("button", { name: "回滚 1.0.0" }));

    await waitFor(() => {
      expect(screen.getAllByText("未签名").length).toBeGreaterThan(0);
      expect(screen.getByText("已隔离")).toBeInTheDocument();
      expect(screen.getByText("revoked by market")).toBeInTheDocument();
      expect(updateMutation.mutateAsync).toHaveBeenCalledWith(
        "/tmp/community-redactor-1.1.0.aio-plugin"
      );
      expect(rollbackMutation.mutateAsync).toHaveBeenCalledWith({
        pluginId: "community.redactor",
        version: "1.0.0",
      });
    });
  });

  it("does not offer enable action for quarantined or uninstalled plugins", () => {
    const enableMutation = mutation();
    vi.mocked(usePluginEnableMutation).mockReturnValue(enableMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [
        summary({
          plugin_id: "community.revoked",
          name: "Revoked Plugin",
          status: "quarantined",
          last_error: "revoked by market",
        }),
        summary({
          plugin_id: "community.removed",
          name: "Removed Plugin",
          status: "uninstalled",
        }),
      ],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getByText("已隔离")).toBeInTheDocument();
    expect(screen.getByText("已卸载")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "启用" })).not.toBeInTheDocument();
    expect(enableMutation.mutateAsync).not.toHaveBeenCalled();
  });
});
