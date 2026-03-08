import { describe, expect, it } from "vitest";
import {
  computeOutputTokensPerSecond,
  formatBytes,
  formatCountdownSeconds,
  formatDurationMs,
  formatDurationMsShort,
  formatInteger,
  formatIsoDateTime,
  formatPercent,
  formatRelativeTimeFromMs,
  formatRelativeTimeFromUnixSeconds,
  formatTokensPerSecond,
  formatTokensPerSecondShort,
  formatUnixSeconds,
  formatUsd,
  formatUsdCompact,
  formatUsdRaw,
  formatUsdShort,
  sanitizeTtfbMs,
} from "../formatters";

describe("utils/formatters", () => {
  it("formatDurationMs variants", () => {
    expect(formatDurationMs(null)).toBe("—");
    expect(formatDurationMs(12.2)).toBe("12ms");
    expect(formatDurationMs(1200)).toBe("1.20s");
    expect(formatDurationMs(61_000)).toBe("1m1.0s");

    expect(formatDurationMsShort(999)).toBe("999ms");
    expect(formatDurationMsShort(1200)).toBe("1.2s");
    expect(formatDurationMsShort(61_000)).toBe("1m");
    expect(formatDurationMsShort(3_660_000)).toBe("1h1m");
  });

  it("sanitizeTtfbMs", () => {
    expect(sanitizeTtfbMs(null, 1)).toBeNull();
    expect(sanitizeTtfbMs(10, null)).toBeNull();
    expect(sanitizeTtfbMs(10, 10)).toBe(10);
    expect(sanitizeTtfbMs(9, 10)).toBe(9);
    expect(sanitizeTtfbMs(11, 10)).toBeNull();
  });

  it("formatInteger / percent", () => {
    expect(formatInteger(undefined)).toBe("—");
    expect(formatInteger(12.7)).toBe("13");
    expect(formatPercent(0.1234, 2)).toBe("12.34%");
    expect(formatPercent(0.1234, Number.NaN)).toBe("12%");
  });

  it("tokens per second", () => {
    expect(computeOutputTokensPerSecond(null, 1000, 100)).toBeNull();
    expect(computeOutputTokensPerSecond(10, 0, 1)).toBeNull();
    expect(computeOutputTokensPerSecond(10, 1000, 1000)).toBeCloseTo(10 / 1.0);
    expect(computeOutputTokensPerSecond(0, 1000, 1000)).toBeNull();
    expect(computeOutputTokensPerSecond(10, 1100, 100)).toBeCloseTo(10 / 1.0);
    expect(formatTokensPerSecond(1.23)).toContain("Token/秒");
  });

  it("USD formatting", () => {
    expect(formatUsd(null)).toBe("—");
    expect(formatUsd(0)).toBe("$0.000000");
    expect(formatUsdRaw(0.12)).toBe("$0.12");
    expect(formatUsdRaw(null)).toBe("—");
    expect(formatUsdShort(1.2)).toBe("$1.20");
  });

  it("time formatters", () => {
    expect(formatUnixSeconds(null)).toBe("—");
    expect(formatCountdownSeconds(61)).toBe("01:01");
    expect(formatCountdownSeconds(3661)).toBe("1:01:01");
    expect(formatRelativeTimeFromMs(null)).toBe("—");
    expect(formatRelativeTimeFromMs(0, Number.NaN)).toBe("—");
    expect(formatRelativeTimeFromMs(0, 0)).toBe("<1分钟");
    expect(formatRelativeTimeFromMs(0, 2 * 3_600_000)).toBe("2小时");
    expect(formatRelativeTimeFromMs(0, 2 * 86_400_000)).toBe("2天");
    expect(formatRelativeTimeFromUnixSeconds(0, 60_000)).toBe("1分钟");
  });

  it("bytes and ISO datetime", () => {
    expect(formatBytes(-1)).toBe("—");
    expect(formatBytes(10)).toBe("10 B");
    expect(formatBytes(1024)).toContain("KB");
    expect(formatBytes(1_500_000)).toContain("MB");
    expect(formatBytes(2_000_000_000)).toContain("GB");
    expect(formatIsoDateTime("")).toBe("—");
    expect(formatIsoDateTime("not-a-date")).toBe("not-a-date");
    expect(formatIsoDateTime("2020-01-02T12:00:00Z")).toContain("2020-01-02");
  });

  it("compact formatters", () => {
    expect(formatTokensPerSecondShort(null)).toBe("—");
    expect(formatTokensPerSecondShort(999.94)).toBe("999.9 t/s");
    expect(formatTokensPerSecondShort(1500)).toBe("1.5k t/s");

    expect(formatUsdCompact(null)).toBe("—");
    expect(formatUsdCompact(0)).toBe("$0");
    expect(formatUsdCompact(0.0012)).toBe("$0.0012");
    expect(formatUsdCompact(1.234)).toBe("$1.23");
  });
});
