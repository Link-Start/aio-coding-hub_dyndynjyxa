# Plugin Config Schema

Plugins may declare a `configSchema` JSON Schema subset in `plugin.json`.

Supported scalar types:

- `string`
- `number`
- `integer`
- `boolean`
- `password`

Supported structured types:

- `array`
- `object`

The `enum` keyword is supported on scalar fields. In checker terms, enum is supported as a keyword on scalar fields, for example:

```json
{ "type": "string", "enum": ["strict", "balanced"] }
```

The GUI renders scalar enum fields as a select control. The GUI renders `password` fields as password inputs.

vNext does not provide host-managed secret storage for community plugin config. Saved config values remain regular plugin config values and may be returned in backend detail payloads. The backend validates config before persisting it; the frontend validation is only a convenience layer.

## UI Metadata

The host renders `configSchema` as a low-code settings panel. Prefer standard JSON Schema presentation fields first:

- `title`: user-facing field name.
- `description`: helper text below the title.
- `default`: value used when saved config omits the field.
- `enum`: allowed values.
- `required`: required object properties.

AIO Coding Hub also supports the vendor extension `x-aio-ui` for presentation hints. These hints do not change backend validation.

Supported root `x-aio-ui` fields:

- `sections`: ordered groups of fields.

Supported field `x-aio-ui` fields:

- `section`: section id.
- `order`: numeric order inside a section.
- `widget`: `text`, `textarea`, `password`, `number`, `switch`, `select`, `checkboxGroup`, or `json`.
- `placeholder`: input placeholder for text-like fields.
- `warning`: always-visible warning copy.
- `warningWhenPartial`: warning copy shown when a checkbox group is partially selected.
- `enumLabels`: map enum values to user-facing labels.
- `enumDescriptions`: map enum values to helper text.

The host may ignore an incompatible widget hint. For example, `checkboxGroup` only applies to `array` fields whose `items.enum` is present.

## Example

```json
{
  "type": "object",
  "required": ["redactBeforeUpstream", "redactLogs", "profile"],
  "x-aio-ui": {
    "sections": [
      {
        "id": "routing",
        "title": "处理位置",
        "description": "选择插件在哪些阶段生效。",
        "order": 10
      },
      {
        "id": "content",
        "title": "要保护的内容",
        "description": "选择需要自动替换的敏感信息类型。",
        "order": 20
      }
    ]
  },
  "properties": {
    "redactBeforeUpstream": {
      "type": "boolean",
      "title": "发送给模型前处理",
      "description": "在请求离开本机前替换你选择的敏感信息。",
      "default": true,
      "x-aio-ui": {
        "section": "routing",
        "widget": "switch",
        "order": 10
      }
    },
    "profile": {
      "type": "string",
      "title": "保护强度",
      "default": "balanced",
      "enum": ["balanced", "strict"],
      "x-aio-ui": {
        "section": "routing",
        "widget": "select",
        "order": 20,
        "enumLabels": {
          "balanced": "平衡",
          "strict": "严格"
        }
      }
    },
    "sensitiveTypes": {
      "type": "array",
      "title": "要保护的内容",
      "description": "关闭某一项后，这类内容不会被该插件处理。",
      "default": ["email", "cn_phone"],
      "items": {
        "type": "string",
        "enum": ["email", "cn_phone"],
        "x-aio-ui": {
          "enumLabels": {
            "email": "邮箱地址",
            "cn_phone": "中国手机号"
          },
          "enumDescriptions": {
            "email": "例如 name@example.com。",
            "cn_phone": "例如 13344441520。"
          }
        }
      },
      "x-aio-ui": {
        "section": "content",
        "widget": "checkboxGroup",
        "order": 10,
        "warningWhenPartial": "关闭后，这类内容会原样发送给模型，也可能出现在本地日志中。"
      }
    }
  }
}
```
