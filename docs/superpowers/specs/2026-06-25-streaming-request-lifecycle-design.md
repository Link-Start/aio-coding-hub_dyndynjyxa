# Streaming Request Lifecycle Design

日期：2026-06-25

## Summary

本设计修复长时间流式请求在首页和请求日志中状态不可靠的问题。当前补丁为了避免 5 分钟以上流式请求从首页消失，放宽了前端展示层对 pending log 的过滤，但终态语义仍不够清晰：一些请求会在几百分钟后仍显示“进行中”。根因不是展示窗口长短，而是系统没有把“请求是否结束”严格绑定到流生命周期。

新的设计原则是：

1. 请求终态只能由真实生命周期事件产生：流自然结束、上游错误、客户端断开、网关停止/启动恢复。
2. 空闲或长耗时只作为可观测状态，不把请求强制标记为失败或完成。
3. 前端不再用固定时间窗口推断请求是否结束。
4. 后端写入足够的活动信息，让 UI 可以显示“进行中 · 已静默 N 分钟”这类解释性状态。

本设计不改变 provider 路由、失败切换、计费和插件 API 的外部行为，只修正流式请求生命周期和 request log 投影。

## Current State

相关背景：

- Issue #304 描述的是：流式响应超过 5 分钟后，首页最近代理记录里的转圈记录暂时消失，直到流结束才重新出现。
- commit `6c4e3aa7a56b4c26a7c9bb78cbe590cc3439ce34` 通过保留 pending logs、启动/停止时 reconcile 未完成记录、调整前端活动投影修复了“消失”问题。
- 新问题是：有些请求几百分钟后仍处于“进行中”。这说明当前系统只是避免隐藏 pending，但没有足够可靠地区分“仍活跃”“已断开但未 finalize”“历史脏 pending”。

当前关键代码：

- `src-tauri/src/gateway/streams/timing.rs`
- `src-tauri/src/gateway/streams/usage_tee.rs`
- `src-tauri/src/gateway/streams/request_end.rs`
- `src-tauri/src/infra/request_logs.rs`
- `src/services/gateway/requestActivityProjection.ts`
- `src/services/gateway/requestLogState.ts`
- `src/services/gateway/traceStore.ts`
- `src/components/home/HomeRequestLogsPanel.tsx`

现有优点：

- 流式响应已经有 `StreamFinalizeCtx` 和 tee stream finalize 入口。
- request log 已支持 placeholder 和最终 log upsert。
- 启动/网关停止恢复可以 reconcile 未完成 pending。

主要缺口：

- 部分 stream wrapper 仍存在总时长 timeout 语义，可能把“时间到了”当作终止条件。
- request log 没有显式记录流最后活动时间，UI 只能用 created/duration/trace last seen 做间接推断。
- 前端的“进行中”状态主要由 `status == null && error_code == null` 推出，缺少“静默很久”的可解释状态。
- 如果最终事件丢失，pending log 只能等网关停止/启动恢复，用户在运行期无法判断它是活跃还是疑似卡住。

## External Reference

参考项目 `ding113/claude-code-hub` 的处理方式：

- 它把流式请求拆成首字节超时、流式静默期超时、非流式总超时三类配置。
- 活跃请求来自持久状态：dashboard 查询 `durationMs IS NULL`。
- 流式成功需要自然结束或终止标记，例如 `response.completed`、`message_stop`、`[DONE]`、`finish_reason`。
- 客户端断开和上游异常断开分开归因。
- 流式静默期超时会主动关闭客户端流并 abort 上游连接，但该超时是 provider 可配置项，`0` 表示禁用。

本项目不照搬“静默期超时主动终止”，因为产品决策是：连接没有断开时不因空闲强行结束。可吸收的是它的状态建模：终态由真实结束事件写入，活跃状态来自持久记录，终止标记用于辅助判断完成和截断。

## Goals

本阶段要达成：

1. 流式请求的完成状态只由流自然结束、上游错误、客户端 abort、网关停止/启动恢复决定。
2. 移除或降级固定总时长对流式响应的强制结束作用。
3. request log 持久化最近活动时间或等价观测信息。
4. 首页和请求日志能展示 pending 请求的空闲状态，例如“进行中 · 已静默 18 分钟”。
5. 长时间仍在输出的流不会因为固定窗口被隐藏或标为失败。
6. 长时间没有任何新数据的流不会被伪装成正常活跃；UI 要给出“疑似卡住”的提示。
7. 网关停止、应用启动恢复、通道关闭等场景仍能把未完成 pending reconcile 成稳定终态。

## Non-Goals

本阶段不做：

- 不新增 provider 级“静默超时后强制中断”的默认行为。
- 不引入新的请求状态机表。
- 不改 provider failover 策略。
- 不改计费规则。
- 不改插件 API。
- 不重做首页请求日志 UI。
- 不把历史所有 pending 记录做复杂迁移；只确保新逻辑和恢复逻辑稳定。

## Architecture

### 1. 请求生命周期边界

流式请求只有四类终态来源：

- `completed`：上游流自然结束，tee stream 读到 `None`，且没有错误码。
- `upstream_error`：读流过程中出现上游错误、stream error、fake 200、协议终止错误等。
- `client_aborted`：客户端断开导致流结束或 wrapper drop，并被识别为客户端中断。
- `reconciled`：应用启动或网关停止时发现仍未终态的 pending request log，由恢复流程写入 `GatewayStop` 或 `StartupRecovery`。

空闲、长耗时和 UI 展示窗口都不是终态来源。

### 2. 活动观测模型

request log 增加“最近活动”语义，字段为：

```text
last_activity_ms INTEGER NULL
activity_details_json TEXT NULL
```

`last_activity_ms` 的写入规则：

- placeholder 插入时设置为 `created_at_ms`。
- 流式 tee 每次读到 chunk 时更新为当前时间，或在最终 log upsert 时至少写入最后已知活动时间。
- 非流式请求可以保持为 `created_at_ms` 或最终完成时间；前端只对 pending 流式请求使用该字段。
- reconcile 未完成 pending 时保留最后活动时间，并把 reconcile 原因写入 `error_details_json`。

最近活动必须落在持久 request log 上，而不是只放在前端 trace store。这样前端刷新、应用重启后仍可解释 pending 状态。

### 3. 流式 wrapper 职责

`TimingOnlyTeeStream`、`UsageSseTeeStream`、`UsageBodyBufferTeeStream` 负责三件事：

- 透传 chunk。
- 收集 usage/ttfb/错误信息。
- 在真实终态时调用 request end finalize。

它们不应使用固定总时长把一个仍连接的流变成 `Poll::Ready(None)`。如果需要观测长耗时，应通过活动字段、日志或 trace event 记录，而不是终止流。

已经存在的 idle timeout 如果语义是“连续无数据后终止”，需要确认默认禁用或只用于明确配置的场景。本阶段按产品决策不新增默认终止行为。

### 4. 前端投影

`requestActivityProjection` 继续合并 request logs 和 live traces，但状态显示要分层：

- `completed`：有 status 或 error_code。
- `in_progress_active`：pending，且最近活动距离当前时间低于提示阈值。
- `in_progress_idle`：pending，且最近活动距离当前时间超过提示阈值。
- `reconciled`：status/error_code 来自 gateway stop/startup recovery。

提示阈值只影响文案和样式，不改变请求终态。阈值使用 10 分钟，常量命名为 `PENDING_IDLE_NOTICE_MS`，避免被理解为生命周期 timeout。

UI 表达：

- 活跃：`进行中`
- 静默：`进行中 · 已静默 18 分钟`
- 长耗时但仍有活动：`进行中 · 已运行 43 分钟`
- 恢复终止：展示已有错误码和恢复原因。

### 5. 恢复与清理

`reconcile_unresolved_pending` 保留，但只在应用启动、网关停止等明确生命周期边界触发。它不应按 pending 年龄周期性扫掉仍可能活跃的请求。

reconcile 写入：

- `status = 499`
- `error_code = GatewayStop` 或 `StartupRecovery`
- `excluded_from_stats = 1`
- `error_details_json` 包含 `reason`、`reconciled_at_ms`、`pending_age_ms`、可选 `last_activity_ms`

## Data Flow

1. 请求进入 gateway，创建 placeholder request log，状态为空，记录 `created_at_ms` 和 `last_activity_ms`。
2. 上游返回流，tee stream 透传 chunk。
3. 每个 chunk 更新内存中的 last activity；写入可以节流，避免每块都落库。
4. 流自然结束或错误时，tee stream finalize，写入最终 request log，status/error_code/duration/token 等字段完整。
5. 前端拉取 request logs 和 live traces，合并为活动投影。
6. 如果 log 仍 pending，前端根据 `last_activity_ms` 显示活跃或静默提示。
7. 应用启动或网关停止时，reconcile 未完成 pending。

## Error Handling

- 上游读流错误：最终 log 写入 stream error，不再保持 pending。
- 客户端 abort：最终 log 写入 client abort 语义，并排除统计或按现有规则处理。
- 日志队列拥塞：placeholder 和最终日志继续使用 write-through fallback，避免丢失终态。
- 活动更新失败：不影响流透传；最终 log 仍是更高优先级事实。
- 前端没有 `last_activity_ms`：回退到 created time，但文案不要暗示确定活跃。

## Testing

后端测试：

- 长时间流式请求只要流未结束，不因固定总时长 finalize。
- 流自然结束后 pending log 更新为终态。
- 上游流错误后 pending log 更新为错误终态。
- 客户端 abort 后 pending log 更新为 abort 终态。
- placeholder 写入后最终 log upsert 不会丢失 `last_activity_ms`。
- reconcile 只处理 status/error_code 为空的 pending rows，并保留 last activity。

前端测试：

- pending 且最近活动新鲜时显示进行中。
- pending 且最近活动很旧时仍显示进行中，但带静默提示。
- completed log 不显示静默提示。
- live trace 和 pending log 合并时不重复显示。
- 老 pending log 不再从列表消失。

集成验证：

- 运行 `cd src-tauri && cargo test request_logs --lib` 或更窄的相关测试。
- 运行 `pnpm test:unit -- src/services/gateway/__tests__/requestActivityProjection.test.ts src/components/home/__tests__/HomeRequestLogsPanel.test.tsx`。
- 运行 `pnpm tauri:check`。

## Success Criteria

- 一个仍在持续输出的 10 分钟以上流式请求始终在首页可见。
- 一个已断开的流式请求不会无限期保持“进行中”。
- 一个连接未断但长时间没有新数据的请求显示为“进行中 · 已静默 N 分钟”，而不是被隐藏或强制失败。
- 网关停止或应用重启后，遗留 pending log 会被 reconcile 成稳定终态。
- 没有新增默认强制中断长流的固定时间。
