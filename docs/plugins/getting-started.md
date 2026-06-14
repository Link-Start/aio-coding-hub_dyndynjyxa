# 插件快速开始

这份指南用于创建、校验、打包并导入一个本地 AIO Coding Hub 插件。

完整文档导航见 [插件开发手册](./README.md)，从 0 到发布的主线说明见 [插件开发总指南](./developer-guide.md)。

## 选择运行时

如果插件可以表达为 regex matching、replacement、warning、blocking 或 appending messages，优先使用 `declarativeRules`。`declarativeRules` 是 vNext 的默认社区运行时。

只有当插件确实需要 code execution，并且能放进隔离的 WASM ABI 时，才使用 WASM。不支持任意 JavaScript 和 TypeScript 插件。

## 安装与检查 SDK

社区插件应使用 `@aio-coding-hub/plugin-sdk` 获取共享的 manifest、hook、permission 和 validation types。

```bash
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
```

SDK 细节见 [插件 SDK](./sdk.md)。

WASM 插件应使用 Rust `aio-plugin-wasm-sdk` contracts：

```bash
pnpm plugin-wasm-sdk:test
```

最小 Rust 示例位于 `packages/plugin-wasm-sdk/examples/redactor`。

## 创建插件

使用 `create-aio-plugin` scaffold 本地插件：

```bash
pnpm --filter create-aio-plugin test
pnpm create-aio-plugin acme.redactor rule
pnpm create-aio-plugin acme.policy wasm
```

每个 scaffold 都包含 `plugin.json`。规则插件还包含 `rules/main.json`；WASM 插件包含一个最小 Rust entrypoint skeleton。

## 最小声明式规则插件

`plugin.json`:

```json
{
  "id": "acme.redactor",
  "name": "Acme Redactor",
  "version": "0.1.0",
  "apiVersion": "1.0.0",
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 50,
      "failurePolicy": "fail-open"
    }
  ],
  "permissions": ["request.body.read", "request.body.write"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  }
}
```

`rules/main.json`:

```json
{
  "rules": [
    {
      "id": "redact-openai-key",
      "hook": "gateway.request.afterBodyRead",
      "target": {
        "field": "request.body",
        "jsonPath": "$.messages[*].content"
      },
      "match": {
        "regex": "sk-(?:proj-)?[A-Za-z0-9_-]{20,}"
      },
      "action": {
        "kind": "replace",
        "replacement": "[REDACTED]"
      }
    }
  ]
}
```

规则细节见 [声明式规则运行时](./declarative-rules.md)。

## 本地开发流程

1. 编辑 `plugin.json`。
2. 用 `pnpm create-aio-plugin validate ./acme.redactor` 校验真实插件目录。
3. 用 `pnpm create-aio-plugin replay ./acme.redactor ./fixtures/request.json gateway.request.afterBodyRead` 回放 fixture。
4. 用 `pnpm create-aio-plugin pack ./acme.redactor` 打包为 `acme.redactor.aio-plugin`。
5. 用 `pnpm create-aio-plugin sign <bytes> [privateKey]` 对 package bytes 签名。
6. 用 `pnpm create-aio-plugin verify <bytes> <signature> <publicKey>` 校验 package bytes。
7. 从 Plugins 页面导入该包。

WASM gateway execution 受策略控制，在 vNext 默认关闭。`plugin.wasm` artifacts 会由 `create-aio-plugin pack` 作为 binary files 打包。

## 推荐流程

1. Scaffold 一个声明式规则插件。

   ```bash
   pnpm create-aio-plugin acme.redactor rule
   ```

2. 校验 `plugin.json` 和规则文件。

   ```bash
   pnpm create-aio-plugin validate ./acme.redactor
   ```

3. 添加 Claude 和 Codex request shapes 作为 replay fixtures。

   Claude fixture：

   ```json
   {
     "request": {
       "body": "{\"messages\":[{\"role\":\"user\",\"content\":\"SECRET_TOKEN\"}]}"
     }
   }
   ```

   Codex/OpenAI Responses fixture：

   ```json
   {
     "request": {
       "body": "{\"input\":[{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"SECRET_TOKEN\"}]}]}"
     }
   }
   ```

4. 在本地回放两个 fixtures。

   ```bash
   pnpm create-aio-plugin replay ./acme.redactor ./fixtures/claude-request.json gateway.request.afterBodyRead
   pnpm create-aio-plugin replay ./acme.redactor ./fixtures/codex-request.json gateway.request.afterBodyRead
   ```

5. 打包插件。

   ```bash
   pnpm create-aio-plugin pack ./acme.redactor
   ```

6. 从 Plugins 页面本地安装。

   使用 Plugins 页面里的本地包安装操作，选择 `acme.redactor.aio-plugin`。

7. 授权请求的 permissions 并启用插件。

   启用前确认插件请求的 `request.body.read` 和 `request.body.write` permissions。

8. 检查 audit logs。

   命中请求后，插件详情面板应展示 hook completion 或 block/failure events，且不应存储 sensitive payload text。

## 下一步参考

- [Manifest](./manifest.md)
- [Hooks](./hooks.md)
- [Permissions](./permissions.md)
- [配置 Schema](./config-schema.md)
- [官方示例](./official-examples.md)
- [发布插件](./publishing.md)
