// Usage:
// - Render in Home page "概览 / 使用记录" area to show up-to-date in-flight traces.
// - Accepts a list of `TraceSession` candidates; component applies its own visibility + exit animation logic.

import { memo, useEffect, useMemo, useState } from "react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import { GatewayErrorCodes } from "../../constants/gatewayErrorCodes";
import type { TraceSession } from "../../services/traceStore";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatInteger,
  formatTokensPerSecond,
  formatTokensPerSecondShort,
  formatUsdRaw,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import { Clock, Server, Loader2, Cpu, Terminal, CheckCircle2, XCircle } from "lucide-react";
import {
  computeEffectiveInputTokens,
  computeStatusBadge,
  getErrorCodeLabel,
  SessionReuseBadge,
} from "./HomeLogShared";

export type RealtimeTraceCardsProps = {
  traces: TraceSession[];
  formatUnixSeconds: (ts: number) => string;
  showCustomTooltip: boolean;
};

const REALTIME_TRACE_EXIT_START_MS = 2200;
const REALTIME_TRACE_EXIT_ANIM_MS = 700;
const REALTIME_TRACE_EXIT_TOTAL_MS =
  REALTIME_TRACE_EXIT_START_MS + REALTIME_TRACE_EXIT_ANIM_MS + 100;

/**
 * UI-level safety net: hide traces stuck in "in progress" beyond this threshold.
 * Works independently of traceStore pruning for defense-in-depth.
 */
const STALE_TRACE_TIMEOUT_MS = 5 * 60 * 1000;

export const RealtimeTraceCards = memo(function RealtimeTraceCards({
  traces,
  formatUnixSeconds,
  showCustomTooltip,
}: RealtimeTraceCardsProps) {
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    if (traces.length === 0) return;
    let timer: number | null = null;
    let active = true;

    const tick = () => {
      if (!active) return false;
      const now = Date.now();
      setNowMs(now);

      return traces.some((trace) => {
        if (!trace.summary) {
          return now - trace.last_seen_ms < STALE_TRACE_TIMEOUT_MS;
        }
        return Math.max(0, now - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
      });
    };

    const stillNeeded = tick();
    if (!stillNeeded) return;

    timer = window.setInterval(() => {
      const needed = tick();
      if (!needed && timer != null) {
        window.clearInterval(timer);
        timer = null;
      }
    }, 250);
    return () => {
      active = false;
      if (timer != null) window.clearInterval(timer);
    };
  }, [traces]);

  const visibleTraces = useMemo(() => {
    const kept = traces.filter((trace) => {
      if (!trace.summary) {
        return nowMs - trace.last_seen_ms < STALE_TRACE_TIMEOUT_MS;
      }
      return Math.max(0, nowMs - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
    });
    return kept.slice(0, 5);
  }, [traces, nowMs]);

  return (
    <>
      {visibleTraces.map((trace) => {
        const completedAgeMs = trace.summary ? Math.max(0, nowMs - trace.last_seen_ms) : 0;
        const isExiting = Boolean(trace.summary) && completedAgeMs >= REALTIME_TRACE_EXIT_START_MS;
        const runningMs = trace.summary
          ? trace.summary.duration_ms
          : Math.max(0, nowMs - trace.first_seen_ms);

        const summaryStatus = trace.summary?.status ?? null;
        const summaryErrorCode = trace.summary?.error_code ?? null;
        const isInProgress = !trace.summary;

        const attemptRoute = (() => {
          const sortedAttempts = (trace.attempts ?? [])
            .slice()
            .sort((a, b) => a.attempt_index - b.attempt_index);

          type RouteSeg = { provider: string; status: "success" | "started" | "failed" };
          const segs: RouteSeg[] = [];

          for (const attempt of sortedAttempts) {
            const raw = attempt.provider_name?.trim();
            if (!raw || raw === "Unknown") continue;

            const status: RouteSeg["status"] =
              attempt.outcome === "success"
                ? "success"
                : attempt.outcome === "started"
                  ? "started"
                  : "failed";

            const last = segs[segs.length - 1];
            if (last?.provider === raw) {
              if (last.status === status) continue;
              if (last.status === "success") continue;
              if (status === "success") {
                last.status = "success";
                continue;
              }
              if (last.status === "started") continue;
              if (status === "started") {
                last.status = "started";
                continue;
              }
              continue;
            }

            segs.push({ provider: raw, status });
          }

          const startProvider = segs[0]?.provider ?? null;
          const endProvider = segs[segs.length - 1]?.provider ?? null;
          const providerText = endProvider ?? "未知";

          const routeLabel = (() => {
            if (segs.length === 0) return null;

            const text = segs
              .map((seg) => {
                const badge =
                  seg.status === "success" ? "✅" : seg.status === "started" ? "⏳" : "❌";
                return `${seg.provider}[${badge}]`;
              })
              .join("->");

            const shouldShow = segs.length > 1 || segs.some((s) => s.status !== "success");
            return shouldShow ? `(${text})` : null;
          })();

          return { providerText, startProvider, endProvider, routeLabel, segments: segs };
        })();

        const hasFailover =
          attemptRoute.segments.length > 1 ||
          attemptRoute.segments.some((s) => s.status === "failed");

        const statusBadge = computeStatusBadge({
          status: summaryStatus,
          errorCode: summaryErrorCode,
          inProgress: isInProgress,
          hasFailover,
        });
        const isClientAbort =
          statusBadge.isClientAbort ||
          summaryStatus === 499 ||
          summaryErrorCode === GatewayErrorCodes.REQUEST_ABORTED ||
          summaryErrorCode === GatewayErrorCodes.STREAM_ABORTED;
        const hasSessionReuse = (trace.attempts ?? []).some(
          (attempt) => attempt.session_reuse === true
        );

        const providerText = attemptRoute.providerText;

        const routeSummary = (() => {
          if (!attemptRoute.startProvider && !attemptRoute.endProvider) return "—";
          if (!attemptRoute.startProvider) return attemptRoute.endProvider ?? "—";
          if (!attemptRoute.endProvider) return attemptRoute.startProvider;
          const routeSegCount = attemptRoute.segments.length;
          const extra = routeSegCount > 2 ? ` +${routeSegCount - 2}` : "";
          return attemptRoute.startProvider === attemptRoute.endProvider
            ? attemptRoute.startProvider
            : `${attemptRoute.startProvider} → ${attemptRoute.endProvider}${extra}`;
        })();

        const modelText =
          trace.requested_model && trace.requested_model.trim()
            ? trace.requested_model.trim()
            : "未知";

        const cacheWrite = (() => {
          const s = trace.summary;
          if (!s)
            return {
              tokens: null as number | null,
              ttl: null as "5m" | "1h" | null,
            };
          // 优先 5m，其次 1h，最后用 cache_creation_input_tokens 汇总
          if (s.cache_creation_5m_input_tokens != null && s.cache_creation_5m_input_tokens > 0) {
            return { tokens: s.cache_creation_5m_input_tokens, ttl: "5m" as const };
          }
          if (s.cache_creation_1h_input_tokens != null && s.cache_creation_1h_input_tokens > 0) {
            return { tokens: s.cache_creation_1h_input_tokens, ttl: "1h" as const };
          }
          if (s.cache_creation_input_tokens != null && s.cache_creation_input_tokens > 0) {
            return { tokens: s.cache_creation_input_tokens, ttl: null };
          }
          if (s.cache_creation_5m_input_tokens != null) {
            return { tokens: s.cache_creation_5m_input_tokens, ttl: "5m" as const };
          }
          if (s.cache_creation_1h_input_tokens != null) {
            return { tokens: s.cache_creation_1h_input_tokens, ttl: "1h" as const };
          }
          if (s.cache_creation_input_tokens != null) {
            return { tokens: s.cache_creation_input_tokens, ttl: null };
          }
          return { tokens: null as number | null, ttl: null as "5m" | "1h" | null };
        })();

        const ttfbMs = trace.summary
          ? sanitizeTtfbMs(trace.summary.ttfb_ms ?? null, trace.summary.duration_ms)
          : null;

        const effectiveInputTokens = computeEffectiveInputTokens(
          trace.cli_key,
          trace.summary?.input_tokens ?? null,
          trace.summary?.cache_read_input_tokens ?? null
        );
        const displayInputTokens = effectiveInputTokens ?? (isClientAbort ? 0 : null);
        const displayOutputTokens = trace.summary?.output_tokens ?? (isClientAbort ? 0 : null);
        const displayCacheReadTokens =
          trace.summary?.cache_read_input_tokens ?? (isClientAbort ? 0 : null);
        const displayCacheWriteTokens = cacheWrite.tokens ?? (isClientAbort ? 0 : null);
        const displayCostUsd = trace.summary?.cost_usd ?? (isClientAbort ? 0 : null);
        const displayCostText = displayCostUsd == null ? "—" : formatUsdRaw(displayCostUsd);

        const outputTokensPerSecond = trace.summary
          ? computeOutputTokensPerSecond(displayOutputTokens, trace.summary.duration_ms, ttfbMs)
          : null;
        const displayOutputTokensPerSecond =
          outputTokensPerSecond ?? (isClientAbort && displayOutputTokens === 0 ? 0 : null);

        return (
          <div
            key={trace.trace_id}
            className={cn(
              "transform overflow-hidden transition-all ease-out motion-reduce:transition-none motion-reduce:transform-none",
              isExiting
                ? "max-h-0 opacity-0 translate-y-1 !mt-0 duration-700"
                : "max-h-[120px] opacity-100 translate-y-0 duration-700 my-1.5 mx-2"
            )}
          >
            <div
              className={cn(
                "relative rounded-lg border",
                isInProgress
                  ? "bg-gradient-to-r from-indigo-50/60 to-white dark:from-indigo-900/30 dark:to-slate-800 border-indigo-200/80 dark:border-indigo-700/60 animate-[glow-pulse_2.5s_ease-in-out_infinite] glow-pulse-active"
                  : hasFailover && !statusBadge.isError
                    ? "bg-gradient-to-r from-amber-50/60 to-white dark:from-amber-900/30 dark:to-slate-800 border-amber-200 dark:border-amber-700 shadow-sm"
                    : "bg-gradient-to-r from-emerald-50/60 to-white dark:from-emerald-900/30 dark:to-slate-800 border-emerald-200 dark:border-emerald-700 shadow-sm"
              )}
            >
              {/* Left accent bar */}
              <div
                className={cn(
                  "absolute left-0 top-2 bottom-2 w-1 rounded-r-full",
                  isInProgress
                    ? "bg-indigo-500"
                    : hasFailover && !statusBadge.isError
                      ? "bg-amber-400"
                      : "bg-emerald-500"
                )}
              />

              <div className="flex flex-col gap-1.5 px-3 py-2.5">
                {/* Row 1: Status + CLI + Model + Time + Badges */}
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className={cn(
                      "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                      statusBadge.tone
                    )}
                    title={statusBadge.title}
                  >
                    {isInProgress ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : statusBadge.isError ? (
                      <XCircle className="h-3 w-3" />
                    ) : (
                      <CheckCircle2 className="h-3 w-3" />
                    )}
                    {statusBadge.text}
                  </span>

                  <span
                    className={cn(
                      "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                      cliBadgeTone(trace.cli_key)
                    )}
                  >
                    {trace.cli_key === "claude" ? (
                      <Terminal className="h-3 w-3" />
                    ) : (
                      <Cpu className="h-3 w-3" />
                    )}
                    {cliShortLabel(trace.cli_key)}
                  </span>

                  <span
                    className="text-xs font-medium text-slate-800 dark:text-slate-200 truncate"
                    title={modelText}
                  >
                    {modelText}
                  </span>

                  {summaryErrorCode && (
                    <span className="rounded bg-amber-50 dark:bg-amber-900/30 px-1 py-0.5 text-[10px] font-medium text-amber-700 dark:text-amber-400 shrink-0">
                      {getErrorCodeLabel(summaryErrorCode)}
                    </span>
                  )}

                  <span className="flex items-center gap-1.5 text-[11px] text-slate-400 dark:text-slate-500 ml-auto shrink-0">
                    {hasSessionReuse && <SessionReuseBadge showCustomTooltip={showCustomTooltip} />}
                    <Clock className="h-3 w-3" />
                    {formatUnixSeconds(Math.floor(trace.first_seen_ms / 1000))}
                  </span>
                </div>

                {/* Row 2: Provider + Stats Grid (2 rows x 4 cols for alignment) */}
                <div className="flex items-start gap-3 text-[11px]">
                  {/* Provider - left side (2 rows: name + placeholder) */}
                  <div className="flex flex-col gap-y-0.5 w-[110px] shrink-0" title={routeSummary}>
                    <div className="flex items-center gap-1 h-4">
                      <Server className="h-3 w-3 text-slate-400 dark:text-slate-500 shrink-0" />
                      <span className="truncate font-medium text-slate-600 dark:text-slate-400">
                        {providerText}
                      </span>
                    </div>
                    <div className="flex items-center h-4">
                      {attemptRoute.routeLabel && !isInProgress ? (
                        <span className="inline-flex items-center gap-0.5 text-[10px] truncate">
                          {attemptRoute.segments.map((seg, idx) => (
                            <span key={idx} className="inline-flex items-center gap-0.5">
                              {idx > 0 && (
                                <span className="text-slate-300 dark:text-slate-600">→</span>
                              )}
                              <span
                                className={
                                  seg.status === "success"
                                    ? "text-emerald-600 dark:text-emerald-400"
                                    : seg.status === "started"
                                      ? "text-indigo-500 dark:text-indigo-400"
                                      : "text-rose-500 dark:text-rose-400"
                                }
                              >
                                {seg.provider}
                              </span>
                            </span>
                          ))}
                        </span>
                      ) : null}
                    </div>
                  </div>

                  {/* Stats Grid: 2 rows x 4 cols */}
                  <div className="grid grid-cols-4 gap-x-3 gap-y-0.5 flex-1 text-slate-500 dark:text-slate-400">
                    {/* Row 1: 输入 | 缓存创建 | 首字 | 花费 */}
                    <div className="flex items-center gap-1 h-4" title="Input Tokens">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">输入</span>
                      <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                        {formatInteger(displayInputTokens)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Cache Write">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">缓存创建</span>
                      {displayCacheWriteTokens != null ? (
                        <>
                          <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                            {formatInteger(displayCacheWriteTokens)}
                          </span>
                          {cacheWrite.ttl && displayCacheWriteTokens > 0 && (
                            <span className="text-slate-400 dark:text-slate-500 text-[10px]">
                              ({cacheWrite.ttl})
                            </span>
                          )}
                        </>
                      ) : (
                        <span className="text-slate-300 dark:text-slate-600">—</span>
                      )}
                    </div>
                    <div className="flex items-center gap-1 h-4" title="TTFB">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">首字</span>
                      <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                        {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Cost">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">花费</span>
                      <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                        {displayCostText}
                      </span>
                    </div>

                    {/* Row 2: 输出 | 缓存读取 | 耗时 | 速率 */}
                    <div className="flex items-center gap-1 h-4" title="Output Tokens">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">输出</span>
                      <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                        {formatInteger(displayOutputTokens)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Cache Read">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">缓存读取</span>
                      {displayCacheReadTokens != null ? (
                        <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                          {formatInteger(displayCacheReadTokens)}
                        </span>
                      ) : (
                        <span className="text-slate-300 dark:text-slate-600">—</span>
                      )}
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Duration">
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">耗时</span>
                      <span
                        className={cn(
                          "font-mono tabular-nums truncate",
                          isInProgress
                            ? "text-indigo-600 dark:text-indigo-400 font-medium"
                            : "text-slate-600 dark:text-slate-300"
                        )}
                      >
                        {formatDurationMs(runningMs)}
                      </span>
                    </div>
                    <div
                      className="flex items-center gap-1 h-4"
                      title={
                        displayOutputTokensPerSecond != null
                          ? formatTokensPerSecond(displayOutputTokensPerSecond)
                          : undefined
                      }
                    >
                      <span className="text-slate-400 dark:text-slate-500 shrink-0">速率</span>
                      {displayOutputTokensPerSecond != null ? (
                        <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                          {formatTokensPerSecondShort(displayOutputTokensPerSecond)}
                        </span>
                      ) : (
                        <span className="text-slate-300 dark:text-slate-600">—</span>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        );
      })}
    </>
  );
});
