import { describe, expect, it } from "vitest";
import { hasFailoverFromSegments } from "../traceRoute";

describe("services/gateway/traceRoute hasFailoverFromSegments", () => {
  it("multi-provider switch counts as failover", () => {
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "failed" },
        { provider: "B", status: "success" },
      ])
    ).toBe(true);
  });

  it("single provider with failed segment (retry then success) is not failover", () => {
    // 回归锚点：旧内联判定因 some(failed) 返回 true，导致落库后徽章跳变。
    expect(hasFailoverFromSegments([{ provider: "A", status: "failed" }])).toBe(false);
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "failed" },
        { provider: "A", status: "success" },
      ])
    ).toBe(false);
  });

  it("skipped segments do not count as hops", () => {
    // 防御分支行为：真实输入（RealtimeTraceCards segments）从不产出 "skipped"
    //（skipped outcome 被映射为 "failed" 计 hop，与 Rust 一致），见 traceRoute.ts 头注释。
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "skipped" },
        { provider: "B", status: "success" },
      ])
    ).toBe(false);
    // skipped 夹在同 provider 中间：仍视为同一 hop
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "failed" },
        { provider: "B", status: "skipped" },
        { provider: "A", status: "success" },
      ])
    ).toBe(false);
  });

  it("empty or all-skipped segments are not failover and do not throw", () => {
    expect(hasFailoverFromSegments([])).toBe(false);
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "skipped" },
        { provider: "B", status: "skipped" },
      ])
    ).toBe(false);
  });
});
