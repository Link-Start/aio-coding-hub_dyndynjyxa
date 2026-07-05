// Mirrors src-tauri/src/infra/request_logs/queries.rs `route_from_attempts` +
// `has_failover`（row_to_summary: has_failover = route.len() > 1）; keep in sync.
//
// 规则：连续同 provider 的段折叠为一个 hop（同 provider 重试不算切换），
// 真正切换过 provider（hop 数 > 1）才算 failover。
//
// 关于 skipped 的差异说明（重要，勿被 Rust 侧注释误导）：
// - Rust `route_from_attempts` 只过滤 provider_id<=0，provider_id>0 的 skipped
//   attempt 会计入 hop（其测试 route_includes_skipped_attempts 断言 len==2）；
//   queries.rs:326 注释"skipped 已被过滤"与实现不符。
// - 本函数的输入来自 RealtimeTraceCards 构造的 segments，其状态域只有
//   success/started/failed——skipped outcome 已被映射为 "failed" 并计入段，
//   因此实际输出与 Rust 行为一致。
// - 下方的 skipped 过滤分支在真实输入中永不触发，仅为防御性行为。
//
// 注意：单 provider 失败（含重试后成功）不算 failover——这是与旧前端内联
// 判定（`segments.some(failed)`）的行为差异点，目的是与落库后的徽章一致。

export type TraceRouteSegment = {
  provider: string;
  status: string;
};

export function hasFailoverFromSegments(segments: ReadonlyArray<TraceRouteSegment>): boolean {
  let hopCount = 0;
  let lastProvider: string | null = null;
  for (const seg of segments) {
    if (seg.status === "skipped") continue;
    if (seg.provider === lastProvider) continue;
    lastProvider = seg.provider;
    hopCount += 1;
    if (hopCount > 1) return true;
  }
  return false;
}
