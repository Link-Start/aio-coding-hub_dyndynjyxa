// Usage:
// - Used by `src/components/home/HomeProviderLimitPanel.tsx` to load provider limit usage data.

import {
  commands,
  type ProviderLimitUsageRow as GeneratedProviderLimitUsageRow,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";
import type { CliKey } from "./providers";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export type ProviderLimitUsageRow = Override<
  GeneratedProviderLimitUsageRow,
  {
    cli_key: CliKey;
  }
>;

function toProviderLimitUsageRow(value: GeneratedProviderLimitUsageRow): ProviderLimitUsageRow {
  return {
    ...value,
    cli_key: narrowGeneratedStringUnion(
      value.cli_key,
      CLI_KEY_VALUES,
      "provider_limit_usage_v1.cli_key"
    ),
  };
}

export async function providerLimitUsageV1(cliKey?: CliKey | null) {
  return invokeGeneratedIpc<ProviderLimitUsageRow[]>({
    title: "读取 Provider 限额用量失败",
    cmd: "provider_limit_usage_v1",
    args: {
      cliKey: cliKey ?? null,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.providerLimitUsageV1(cliKey ?? null), (rows) =>
        rows.map(toProviderLimitUsageRow)
      ),
  });
}
