# AIO Coding Hub 插件开发手册

本目录是 AIO Coding Hub 插件系统的中文开发手册。插件可以扩展本地网关、请求和响应 hook、日志脱敏，以及由界面管理的配置表单。

社区插件优先使用 `declarativeRules`。只有当规则运行时无法表达插件逻辑，并且宿主策略明确启用隔离运行时时，才考虑 WASM 或未来的进程运行时。`native` 只保留给宿主内置官方插件。

## 入门路径

- [插件开发总指南](./developer-guide.md)：从插件目录、`plugin.json`、配置表单、fixture replay 到发布的完整路径。
- [快速开始](./getting-started.md)：创建第一个本地插件，完成校验、回放、打包和导入。
- [插件 SDK](./sdk.md)：插件作者和工具链使用的 TypeScript/Rust 契约与校验辅助函数。
- [声明式规则](./declarative-rules.md)：无需执行任意代码即可完成请求改写、日志脱敏、安全检查和提示词追加。
- [官方示例](./official-examples.md)：内置 `official.privacy-filter` 展示的能力边界。

## 核心契约

- [Manifest](./manifest.md)：`plugin.json` 必填字段、运行时声明和命名规则。
- [Manifest v1 完整规范](../plugin-manifest-v1.md)：规范性 manifest 文档和完整示例。
- [Hooks](./hooks.md)：网关和日志 hook 名称、触发阶段、超时和使用场景。
- [Permissions](./permissions.md)：权限名称、风险等级和授权行为。
- [Config Schema](./config-schema.md)：用于界面渲染和后端校验的配置 schema 子集。
- [兼容性](./compatibility.md)：应用版本、插件 API、平台和 WASM ABI 的版本规则。

## 运行时与分发

- [安全与隔离](./security.md)：最小权限、运行时隔离、签名和失败策略。
- [流式响应插件](./streaming.md)：有边界的 stream chunk 处理。
- [WASM 运行时](./wasm-runtime.md)：WASM ABI v1 和资源限制。
- [进程运行时 PoC](./process-runtime-poc.md)：默认关闭的进程隔离设计。
- [发布插件](./publishing.md)：`.aio-plugin` 打包、校验和、签名、更新与回滚。
- [架构审计](./architecture-audit.md)：信任边界、运行时选择、性能和稳定性建议。

## 推荐开发顺序

1. 如果插件不需要执行代码，先选择 `declarativeRules`。
2. 编写 `plugin.json`，只声明必需的 hooks 和 permissions。
3. 添加聚焦的规则文件、fixture，或 WASM 入口代码。
4. 使用 `create-aio-plugin` 校验真实插件目录。
5. 在导入桌面应用前，用 replay fixture 覆盖 Claude 和 Codex/OpenAI Responses 请求形态。
6. 本地行为稳定后再打包 `.aio-plugin`，需要可信分发时再补签名。

## 当前稳定性说明

- 不支持任意 JavaScript 或 TypeScript 插件直接运行。
- WASM 和进程运行时文档描述的是隔离契约；是否允许执行仍由宿主策略控制。
- Manifest 校验只接受已激活 hooks 和 permissions；保留项仅用于未来兼容命名。
- 当前只有 `official.privacy-filter` 是内置官方 `native` 插件。社区扩展应使用 `declarativeRules`、WASM，或未来隔离进程运行时。
