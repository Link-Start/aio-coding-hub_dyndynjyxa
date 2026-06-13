# Generic Plugin Config UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make plugin configuration feel like a low-code settings panel driven by plugin schema metadata, so Privacy Filter and future plugins can expose titled inputs, switches, selects, checkbox groups, help text, grouping, and warnings without host-side plugin-specific React code.

**Architecture:** Keep backend plugin config persistence unchanged and treat `manifest.configSchema` as the single source of truth. Extend the frontend JSON Schema subset with optional AIO UI metadata (`x-aio-ui`) that describes labels, descriptions, grouping, widget preference, ordering, and warning copy, then compile that schema into a small render model consumed by `PluginConfigSchemaForm`. Privacy Filter becomes an example schema that exercises the generic renderer; `PluginsPage` must not branch on `official.privacy-filter` for configuration UI.

**Tech Stack:** React, TypeScript, TanStack Query, Vitest, Testing Library, existing `src/services/plugins.ts` IPC wrappers, existing `PluginConfigSchemaForm`, JSON Schema subset plus AIO vendor extensions.

---

## Product Scope

This plan changes the plugin configuration user experience, not the plugin runtime.

In scope:
- Add a generic low-code configuration render model for plugin schemas.
- Support field labels, descriptions, placeholders, ordering, grouping, widget hints, option labels, and warning/help copy.
- Support common user controls: text input, password input, number input, switch, select, checkbox group, JSON textarea fallback.
- Keep unknown or unsupported schema parts editable through safe fallback UI.
- Productize `PluginsPage` copy so normal users see value, data access, and settings before raw manifest details.
- Use Privacy Filter only as a generic schema-driven acceptance example.
- Update plugin developer documentation so third-party plugin authors can use the same UI metadata.

Out of scope:
- No backend config storage change.
- No new plugin runtime.
- No plugin-specific React component registry in this iteration.
- No response de-redaction feature.
- No host-managed secret vault.
- No marketplace browsing redesign.

## Core Design Decisions

1. The host must not hard-code config UI by plugin id. `official.privacy-filter` can have official catalog copy, but configuration controls must come from schema metadata.
2. Use JSON Schema-compatible vendor extensions rather than inventing a separate config DSL. The main extension key is `x-aio-ui`, so third-party schemas remain understandable as normal JSON Schema.
3. Compile schema to a render model before rendering. This keeps parsing, defaults, labels, grouping, and widget selection testable without React.
4. The generic renderer should be intentionally small. It supports the widgets this product needs now and falls back to JSON editing for unsupported shapes.
5. Privacy Filter proves the generic system by declaring `sensitiveTypes` as an `array` with `items.enum` and option titles/descriptions, not by receiving a bespoke `PrivacyFilterConfigPanel`.

## Low-Code Template Rendering Architecture

The configuration UI must be treated as a small low-code rendering system, not as a set of plugin-specific React panels. Every configurable plugin follows the same pipeline:

```text
plugin.json configSchema
  -> schema metadata helpers
  -> PluginConfigRenderModel sections and fields
  -> PluginConfigSchemaForm generic templates
  -> persisted plugin config JSON
```

The template renderer owns layout and interaction behavior. Plugins only declare data:

- Fields declare user-facing `title`, `description`, `default`, `required`, `enum`, and type information.
- Fields may declare `x-aio-ui.widget` as a presentation hint for text input, textarea, password input, number input, switch, select, checkbox group, or JSON fallback.
- Fields may declare `x-aio-ui.section` and `x-aio-ui.order` so the host can render grouped settings without hard-coded page structure.
- Enum fields may declare `x-aio-ui.enumLabels` and `x-aio-ui.enumDescriptions` so users see meaningful option names instead of raw ids.
- Warning/help copy stays declarative through `x-aio-ui.warning` and `x-aio-ui.warningWhenPartial`.

The host decides whether a requested widget is compatible with the schema. If a plugin requests an incompatible widget, the renderer must choose a safe fallback instead of crashing or hiding the setting. Unsupported structured fields stay editable through the JSON template so users are never blocked from editing valid plugin config.

This architecture is intentionally generic:

- No renderer code may branch on `plugin_id`, package name, runtime, or official/community source.
- No Privacy Filter strategy id may appear in generic renderer code.
- The renderer must support normal user input, on/off switches, titled fields, titled option groups, stable ordering, and unknown config fallback for any plugin.
- The render model is the contract between schema parsing and React. React components should consume the render model, not raw schema details.
- Backend persistence remains plain JSON. UI metadata changes presentation only and must not change backend validation semantics.

## Supported `x-aio-ui` Shape

The first iteration supports these optional schema metadata fields:

```json
{
  "type": "object",
  "x-aio-ui": {
    "layout": "sections",
    "sections": [
      {
        "id": "routing",
        "title": "处理位置",
        "description": "选择插件在哪些阶段生效。",
        "order": 10
      }
    ]
  },
  "properties": {
    "enabled": {
      "type": "boolean",
      "title": "启用处理",
      "description": "关闭后插件不会修改内容。",
      "default": true,
      "x-aio-ui": {
        "section": "routing",
        "widget": "switch",
        "order": 10
      }
    },
    "mode": {
      "type": "string",
      "title": "处理模式",
      "description": "选择插件的处理强度。",
      "enum": ["balanced", "strict"],
      "x-aio-ui": {
        "section": "routing",
        "widget": "select",
        "enumLabels": {
          "balanced": "平衡",
          "strict": "严格"
        }
      }
    },
    "sensitiveTypes": {
      "type": "array",
      "title": "要保护的内容",
      "description": "选择哪些类型需要处理。",
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
        "section": "routing",
        "widget": "checkboxGroup",
        "warningWhenPartial": "关闭后，这类内容会原样发送给模型，也可能出现在本地日志中。"
      }
    }
  }
}
```

Rules:
- Prefer standard `title`, `description`, `default`, `required`, and `enum` when possible.
- Use `x-aio-ui.widget` only as a hint. The renderer may fall back when the schema type cannot support the requested widget.
- Use `x-aio-ui.section` to group fields. Unknown section ids go into a default `常规设置` section.
- Use `x-aio-ui.order` for stable sorting. Fields without order keep schema property order after ordered fields.
- Use `x-aio-ui.enumLabels` and `x-aio-ui.enumDescriptions` for user-facing option copy.
- Use `x-aio-ui.warning` or `x-aio-ui.warningWhenPartial` for plugin-authored caution copy.

## Iteration Boundaries

Each task below is a separate development unit. Finish the RED/GREEN checks and commit the current unit before starting the next one.

| Unit | Development Boundary | Verification Boundary | Commit Boundary |
| --- | --- | --- | --- |
| Task 1 | Pure schema metadata helpers only | `pluginConfigUiSchema.test.ts` passes | `feat: parse plugin config ui metadata` |
| Task 2 | Pure render model compiler only | `pluginConfigRenderModel.test.ts` passes | `feat: compile plugin config render model` |
| Task 3 | Generic form widgets only | `PluginConfigSchemaForm.test.tsx` passes | `feat: render low-code plugin config fields` |
| Task 4 | Plugins page product copy and generic integration only | `PluginsPage.test.tsx` passes | `feat: productize plugin settings page` |
| Task 5 | Privacy Filter fixture/schema as generic example only | relevant query/service/page tests pass | `test: model privacy filter as schema-driven config` |
| Task 6 | Developer documentation only | docs scan and targeted tests pass | `docs: document plugin config ui metadata` |
| Task 7 | Final verification only | focused tests, typecheck, lint, backend regression pass | no separate commit unless fixing a verified failure |

## File Structure

- Create `src/pages/plugins/pluginConfigUiSchema.ts`
  - Reads standard schema fields and `x-aio-ui` metadata.
  - Provides pure helpers for labels, descriptions, section metadata, widget hints, enum labels, defaults, and ordering.
  - No React dependency.

- Create `src/pages/plugins/pluginConfigRenderModel.ts`
  - Converts a schema and current config into section/field render model objects.
  - Decides which widget to use for each field.
  - Applies defaults for missing values without mutating persisted config until save.
  - No React dependency.

- Modify `src/pages/plugins/pluginConfigValidation.ts`
  - Keep low-level JSON/schema helper functions.
  - Add array item enum helper and safe default coercion used by the render model.

- Modify `src/pages/plugins/PluginConfigSchemaForm.tsx`
  - Render the compiled model instead of directly iterating raw schema properties.
  - Add generic widgets for switches, selects, checkbox groups, numeric/text/password inputs, and JSON fallback.
  - Keep `onSubmit(value)` behavior unchanged.

- Create `src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts`
  - Tests metadata parsing without React.

- Create `src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts`
  - Tests schema-to-render-model compilation without React.

- Modify `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`
  - Tests low-code field labels, descriptions, groups, switches, checkbox groups, select labels, defaults, and JSON fallback.

- Create `src/pages/plugins/pluginProductCopy.ts`
  - Own user-facing labels and explanations for statuses, risks, runtimes, permissions, hooks, install sources, and generic plugin value copy.
  - No Privacy Filter config logic.

- Create `src/pages/plugins/__tests__/pluginProductCopy.test.ts`
  - Tests product copy mapping.

- Modify `src/pages/PluginsPage.tsx`
  - Productize page copy.
  - Keep `PluginConfigSchemaForm` as the only config renderer.
  - Do not import Privacy Filter-specific config code.
  - Treat `PluginConfigSchemaForm` as the low-code template renderer for every plugin config schema.

- Modify `src/pages/__tests__/PluginsPage.test.tsx`
  - Cover productized page copy and verify generic config renderer is used for official and community plugins.

- Modify `src/test/msw/state.ts`
  - Update official Privacy Filter fixture so its existing config shape includes generic `configSchema` UI metadata.

- Modify docs:
  - `docs/plugins/config-schema.md`
  - `docs/plugins/manifest.md`
  - `docs/plugins/official-examples.md`
  - `docs/plugins/sdk.md` if SDK examples include `configSchema`

## User-Facing Design

The Plugins page should answer these questions first:

- What does this plugin do?
- Is it running?
- What data can it read or change?
- What can I configure safely?
- Where can I inspect technical details if I need them?

The settings area should feel like a normal product settings panel:

- Field names use `title`, not raw config keys.
- Details use `description`, not manifest jargon.
- Boolean fields use switches.
- String enum fields use selects with option labels.
- Array enum fields use checkbox groups with option descriptions.
- Groups use section titles such as `处理位置`, `要保护的内容`, or plugin-provided equivalents.
- Unsupported object/array fields remain editable through JSON textarea fallback.

Privacy Filter should be rendered by the same generic path:

- `redactBeforeUpstream`: boolean switch with title `发送给模型前处理`.
- `redactLogs`: boolean switch with title `保存日志前处理`.
- `profile`: string enum select with title `保护强度`.
- `sensitiveTypes`: array enum checkbox group with title `要保护的内容`.
- Turning off only `邮箱地址` must submit `sensitiveTypes` without `"email"`.

Developer details should be secondary:

- Section title: `开发者信息`
- Includes plugin ID, runtime, hooks, API version, installed path, host compatibility.
- Raw identifiers remain visible as support/debug details, not as the primary UX.

## Task 1: Add Config UI Metadata Helpers

**Boundary:** Pure TypeScript schema metadata helpers only. No React changes, no page changes, no Privacy Filter-specific helpers.

**Files:**
- Create: `src/pages/plugins/pluginConfigUiSchema.ts`
- Create: `src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  configFieldDescription,
  configFieldLabel,
  configFieldOrder,
  configFieldPlaceholder,
  configFieldWarning,
  configFieldWidgetHint,
  configSchemaSections,
  enumOptionDescription,
  enumOptionLabel,
} from "../pluginConfigUiSchema";

describe("pluginConfigUiSchema", () => {
  const schema = {
    type: "object",
    "x-aio-ui": {
      sections: [
        { id: "routing", title: "处理位置", description: "选择插件在哪些阶段生效。", order: 20 },
        { id: "content", title: "要保护的内容", order: 10 },
      ],
    },
    properties: {
      redactBeforeUpstream: {
        type: "boolean",
        title: "发送给模型前处理",
        description: "在请求离开本机前替换敏感内容。",
        "x-aio-ui": {
          section: "routing",
          widget: "switch",
          order: 5,
          warning: "关闭后请求正文会原样发送。",
        },
      },
      sensitiveTypes: {
        type: "array",
        title: "要保护的内容",
        description: "选择需要处理的内容类型。",
        items: {
          type: "string",
          enum: ["email", "cn_phone"],
          "x-aio-ui": {
            enumLabels: {
              email: "邮箱地址",
              cn_phone: "中国手机号",
            },
            enumDescriptions: {
              email: "例如 name@example.com。",
              cn_phone: "例如 13344441520。",
            },
          },
        },
        "x-aio-ui": {
          section: "content",
          widget: "checkboxGroup",
          placeholder: "选择至少一种内容类型",
          warningWhenPartial: "关闭后，这类内容会原样发送给模型。",
        },
      },
    },
  };

  it("reads section metadata in stable order", () => {
    expect(configSchemaSections(schema)).toEqual([
      { id: "content", title: "要保护的内容", description: null, order: 10 },
      { id: "routing", title: "处理位置", description: "选择插件在哪些阶段生效。", order: 20 },
    ]);
  });

  it("prefers title and description over raw keys", () => {
    const field = schema.properties.redactBeforeUpstream;
    expect(configFieldLabel("redactBeforeUpstream", field, false)).toBe("发送给模型前处理");
    expect(configFieldLabel("redactBeforeUpstream", field, true)).toBe("发送给模型前处理 *");
    expect(configFieldDescription(field)).toBe("在请求离开本机前替换敏感内容。");
  });

  it("reads widget hints, order, placeholder, and warning copy", () => {
    const field = schema.properties.sensitiveTypes;
    expect(configFieldWidgetHint(field)).toBe("checkboxGroup");
    expect(configFieldOrder(field)).toBe(Number.POSITIVE_INFINITY);
    expect(configFieldPlaceholder(field)).toBe("选择至少一种内容类型");
    expect(configFieldWarning(field, "partial")).toBe("关闭后，这类内容会原样发送给模型。");
  });

  it("reads enum option labels and descriptions from item metadata", () => {
    const items = schema.properties.sensitiveTypes.items;
    expect(enumOptionLabel(items, "email")).toBe("邮箱地址");
    expect(enumOptionDescription(items, "cn_phone")).toBe("例如 13344441520。");
    expect(enumOptionLabel(items, "unknown")).toBe("unknown");
  });
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts
```

Expected:

```text
FAIL src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts
Cannot find module '../pluginConfigUiSchema'
```

- [ ] **Step 3: Implement the metadata helpers**

Create `src/pages/plugins/pluginConfigUiSchema.ts`:

```ts
import type { JsonValue } from "../../services/plugins";
import { isRecord } from "./pluginConfigValidation";

export type PluginConfigWidgetHint =
  | "text"
  | "textarea"
  | "password"
  | "number"
  | "switch"
  | "select"
  | "checkboxGroup"
  | "json";

export type PluginConfigUiSection = {
  id: string;
  title: string;
  description: string | null;
  order: number;
};

function stringOrNull(value: JsonValue | undefined): string | null {
  return typeof value === "string" && value.trim() ? value : null;
}

function numberOrInfinity(value: JsonValue | undefined): number {
  return typeof value === "number" && Number.isFinite(value) ? value : Number.POSITIVE_INFINITY;
}

function ui(schema: JsonValue | undefined): Record<string, JsonValue> {
  if (!isRecord(schema) || !isRecord(schema["x-aio-ui"])) return {};
  return schema["x-aio-ui"];
}

export function configSchemaSections(schema: JsonValue | undefined): PluginConfigUiSection[] {
  const rawSections = ui(schema).sections;
  if (!Array.isArray(rawSections)) return [];
  return rawSections
    .filter(isRecord)
    .map((section) => {
      const id = stringOrNull(section.id) ?? "default";
      return {
        id,
        title: stringOrNull(section.title) ?? id,
        description: stringOrNull(section.description),
        order: numberOrInfinity(section.order),
      };
    })
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title));
}

export function configFieldLabel(
  key: string,
  fieldSchema: JsonValue | undefined,
  required: boolean
): string {
  const base = isRecord(fieldSchema) ? stringOrNull(fieldSchema.title) ?? key : key;
  return required ? `${base} *` : base;
}

export function configFieldDescription(fieldSchema: JsonValue | undefined): string | null {
  return isRecord(fieldSchema) ? stringOrNull(fieldSchema.description) : null;
}

export function configFieldSection(fieldSchema: JsonValue | undefined): string | null {
  return stringOrNull(ui(fieldSchema).section);
}

export function configFieldOrder(fieldSchema: JsonValue | undefined): number {
  return numberOrInfinity(ui(fieldSchema).order);
}

export function configFieldWidgetHint(
  fieldSchema: JsonValue | undefined
): PluginConfigWidgetHint | null {
  const widget = stringOrNull(ui(fieldSchema).widget);
  switch (widget) {
    case "text":
    case "textarea":
    case "password":
    case "number":
    case "switch":
    case "select":
    case "checkboxGroup":
    case "json":
      return widget;
    default:
      return null;
  }
}

export function configFieldPlaceholder(fieldSchema: JsonValue | undefined): string | null {
  return stringOrNull(ui(fieldSchema).placeholder);
}

export function configFieldWarning(
  fieldSchema: JsonValue | undefined,
  state: "always" | "partial" = "always"
): string | null {
  const fieldUi = ui(fieldSchema);
  if (state === "partial") {
    return stringOrNull(fieldUi.warningWhenPartial) ?? stringOrNull(fieldUi.warning);
  }
  return stringOrNull(fieldUi.warning);
}

export function enumOptionLabel(itemSchema: JsonValue | undefined, value: JsonValue): string {
  const labels = ui(itemSchema).enumLabels;
  const key = String(value);
  if (isRecord(labels) && typeof labels[key] === "string") return labels[key];
  return key;
}

export function enumOptionDescription(
  itemSchema: JsonValue | undefined,
  value: JsonValue
): string | null {
  const descriptions = ui(itemSchema).enumDescriptions;
  const key = String(value);
  if (isRecord(descriptions) && typeof descriptions[key] === "string") return descriptions[key];
  return null;
}
```

- [ ] **Step 4: Run the test and verify GREEN**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts
```

Expected:

```text
PASS src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts
```

**Acceptance Criteria:**
- Metadata helpers are pure and independently tested.
- Helpers support section title/description/order.
- Helpers support field title/description/placeholder/widget/order/warning.
- Helpers support enum option labels/descriptions.
- No Privacy Filter-specific logic exists in this file.

- [ ] **Step 5: Commit this unit**

Run:

```bash
git add src/pages/plugins/pluginConfigUiSchema.ts src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts
git commit -m "feat: parse plugin config ui metadata"
```

Expected:

```text
[branch ...] feat: parse plugin config ui metadata
```

## Task 2: Add Generic Config Render Model Compiler

**Boundary:** Pure schema-to-render-model compiler only. No React component changes and no page changes.

**Files:**
- Create: `src/pages/plugins/pluginConfigRenderModel.ts`
- Create: `src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts`
- Modify: `src/pages/plugins/pluginConfigValidation.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { buildPluginConfigRenderModel } from "../pluginConfigRenderModel";

describe("buildPluginConfigRenderModel", () => {
  it("compiles titled fields into ordered sections", () => {
    const model = buildPluginConfigRenderModel({
      schema: {
        type: "object",
        required: ["enabled"],
        "x-aio-ui": {
          sections: [
            { id: "content", title: "内容策略", order: 20 },
            { id: "routing", title: "处理位置", order: 10 },
          ],
        },
        properties: {
          enabled: {
            type: "boolean",
            title: "启用处理",
            description: "关闭后插件不会修改内容。",
            default: true,
            "x-aio-ui": { section: "routing", widget: "switch", order: 10 },
          },
          mode: {
            type: "string",
            title: "处理模式",
            enum: ["balanced", "strict"],
            "x-aio-ui": {
              section: "content",
              widget: "select",
              order: 5,
              enumLabels: { balanced: "平衡", strict: "严格" },
            },
          },
        },
      },
      value: {},
    });

    expect(model.sections.map((section) => section.title)).toEqual(["处理位置", "内容策略"]);
    expect(model.sections[0].fields[0]).toMatchObject({
      key: "enabled",
      label: "启用处理 *",
      description: "关闭后插件不会修改内容。",
      widget: "switch",
      value: true,
    });
    expect(model.sections[1].fields[0]).toMatchObject({
      key: "mode",
      label: "处理模式",
      widget: "select",
      value: "balanced",
      options: [
        { value: "balanced", label: "平衡", description: null },
        { value: "strict", label: "严格", description: null },
      ],
    });
  });

  it("compiles array enum fields into checkbox groups with option descriptions", () => {
    const model = buildPluginConfigRenderModel({
      schema: {
        type: "object",
        properties: {
          sensitiveTypes: {
            type: "array",
            title: "要保护的内容",
            default: ["email", "cn_phone"],
            items: {
              type: "string",
              enum: ["email", "cn_phone"],
              "x-aio-ui": {
                enumLabels: { email: "邮箱地址", cn_phone: "中国手机号" },
                enumDescriptions: {
                  email: "例如 name@example.com。",
                  cn_phone: "例如 13344441520。",
                },
              },
            },
            "x-aio-ui": {
              widget: "checkboxGroup",
              warningWhenPartial: "关闭后，这类内容会原样发送给模型。",
            },
          },
        },
      },
      value: { sensitiveTypes: ["email"] },
    });

    expect(model.sections[0].fields[0]).toMatchObject({
      key: "sensitiveTypes",
      widget: "checkboxGroup",
      value: ["email"],
      warning: "关闭后，这类内容会原样发送给模型。",
      options: [
        { value: "email", label: "邮箱地址", description: "例如 name@example.com。" },
        { value: "cn_phone", label: "中国手机号", description: "例如 13344441520。" },
      ],
    });
  });

  it("falls back unsupported structured fields to json widgets", () => {
    const model = buildPluginConfigRenderModel({
      schema: {
        type: "object",
        properties: {
          advanced: {
            type: "object",
            title: "高级配置",
          },
        },
      },
      value: { advanced: { retries: 2 } },
    });

    expect(model.sections[0].fields[0]).toMatchObject({
      key: "advanced",
      label: "高级配置",
      widget: "json",
      value: { retries: 2 },
    });
  });
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts
```

Expected:

```text
FAIL src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts
Cannot find module '../pluginConfigRenderModel'
```

- [ ] **Step 3: Add low-level schema helper for array item enum**

Modify `src/pages/plugins/pluginConfigValidation.ts`:

```ts
export function schemaDefault(schema: JsonValue | undefined): JsonValue | undefined {
  if (!isRecord(schema) || !("default" in schema)) return undefined;
  return schema.default;
}

export function schemaItems(schema: JsonValue | undefined): JsonValue | undefined {
  if (!isRecord(schema)) return undefined;
  return schema.items;
}

export function schemaArrayItemEnum(schema: JsonValue | undefined): JsonValue[] {
  const items = schemaItems(schema);
  if (!isRecord(schema) || schema.type !== "array" || !isRecord(items)) return [];
  return schemaEnum(items);
}
```

- [ ] **Step 4: Implement the render model compiler**

Create `src/pages/plugins/pluginConfigRenderModel.ts`:

```ts
import type { JsonValue } from "../../services/plugins";
import {
  configFieldDescription,
  configFieldLabel,
  configFieldOrder,
  configFieldPlaceholder,
  configFieldSection,
  configFieldWarning,
  configFieldWidgetHint,
  configSchemaSections,
  enumOptionDescription,
  enumOptionLabel,
} from "./pluginConfigUiSchema";
import {
  isRecord,
  schemaArrayItemEnum,
  schemaDefault,
  schemaEnum,
  schemaItems,
  schemaProperties,
  schemaRequired,
  schemaType,
} from "./pluginConfigValidation";

export type PluginConfigWidget =
  | "text"
  | "textarea"
  | "password"
  | "number"
  | "switch"
  | "select"
  | "checkboxGroup"
  | "json";

export type PluginConfigOptionModel = {
  value: JsonValue;
  label: string;
  description: string | null;
};

export type PluginConfigFieldModel = {
  key: string;
  label: string;
  description: string | null;
  placeholder: string | null;
  required: boolean;
  type: string | null;
  widget: PluginConfigWidget;
  value: JsonValue | undefined;
  options: PluginConfigOptionModel[];
  warning: string | null;
  order: number;
};

export type PluginConfigSectionModel = {
  id: string;
  title: string;
  description: string | null;
  order: number;
  fields: PluginConfigFieldModel[];
};

export type PluginConfigRenderModel = {
  editable: boolean;
  sections: PluginConfigSectionModel[];
};

function valueForField(config: Record<string, JsonValue>, key: string, fieldSchema: JsonValue) {
  if (Object.prototype.hasOwnProperty.call(config, key)) return config[key];
  return schemaDefault(fieldSchema);
}

function widgetForField(fieldSchema: JsonValue, hasEnum: boolean, hasArrayEnum: boolean) {
  const type = schemaType(fieldSchema);
  const hint = configFieldWidgetHint(fieldSchema);

  if (hint === "checkboxGroup" && hasArrayEnum) return "checkboxGroup";
  if (hint === "select" && hasEnum) return "select";
  if (hint === "switch" && type === "boolean") return "switch";
  if (hint === "textarea" && type === "string") return "textarea";
  if (hint === "password") return "password";
  if (hint === "number" && (type === "number" || type === "integer")) return "number";
  if (hint === "json") return "json";

  if (type === "boolean") return "switch";
  if (hasArrayEnum) return "checkboxGroup";
  if (hasEnum) return "select";
  if (type === "password") return "password";
  if (type === "number" || type === "integer") return "number";
  if (type === "string" || type == null) return "text";
  return "json";
}

export function buildPluginConfigRenderModel(input: {
  schema: JsonValue | null | undefined;
  value: JsonValue;
}): PluginConfigRenderModel {
  const properties = schemaProperties(input.schema);
  const entries = Object.entries(properties);
  if (schemaType(input.schema) !== "object" || entries.length === 0) {
    return { editable: false, sections: [] };
  }

  const config = isRecord(input.value) ? input.value : {};
  const required = schemaRequired(input.schema);
  const declaredSections = configSchemaSections(input.schema);
  const sectionMap = new Map<string, PluginConfigSectionModel>();

  for (const section of declaredSections) {
    sectionMap.set(section.id, { ...section, fields: [] });
  }

  function ensureSection(id: string | null): PluginConfigSectionModel {
    const normalizedId = id ?? "default";
    const existing = sectionMap.get(normalizedId);
    if (existing) return existing;
    const section = {
      id: normalizedId,
      title: normalizedId === "default" ? "常规设置" : normalizedId,
      description: null,
      order: Number.POSITIVE_INFINITY,
      fields: [],
    };
    sectionMap.set(normalizedId, section);
    return section;
  }

  entries.forEach(([key, fieldSchema], index) => {
    const enumValues = schemaEnum(fieldSchema);
    const arrayEnumValues = schemaArrayItemEnum(fieldSchema);
    const items = schemaItems(fieldSchema);
    const hasEnum = enumValues.length > 0;
    const hasArrayEnum = arrayEnumValues.length > 0;
    const widget = widgetForField(fieldSchema, hasEnum, hasArrayEnum);
    const optionSource = hasArrayEnum ? arrayEnumValues : enumValues;
    const optionSchema = hasArrayEnum ? items : fieldSchema;
    const fieldOrder = configFieldOrder(fieldSchema);

    ensureSection(configFieldSection(fieldSchema)).fields.push({
      key,
      label: configFieldLabel(key, fieldSchema, required.has(key)),
      description: configFieldDescription(fieldSchema),
      placeholder: configFieldPlaceholder(fieldSchema),
      required: required.has(key),
      type: schemaType(fieldSchema),
      widget,
      value: valueForField(config, key, fieldSchema),
      options: optionSource.map((value) => ({
        value,
        label: enumOptionLabel(optionSchema, value),
        description: enumOptionDescription(optionSchema, value),
      })),
      warning:
        widget === "checkboxGroup"
          ? configFieldWarning(fieldSchema, "partial")
          : configFieldWarning(fieldSchema),
      order: fieldOrder === Number.POSITIVE_INFINITY ? Number.POSITIVE_INFINITY + index : fieldOrder,
    });
  });

  const sections = [...sectionMap.values()]
    .map((section) => ({
      ...section,
      fields: section.fields.sort((left, right) => left.order - right.order),
    }))
    .filter((section) => section.fields.length > 0)
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title));

  return { editable: true, sections };
}
```

- [ ] **Step 5: Run the test and verify GREEN**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts
```

Expected:

```text
PASS src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts
```

**Acceptance Criteria:**
- The compiler is pure and independently tested.
- The compiler supports titled sections and ordered fields.
- The compiler chooses widgets generically from schema type, enum, array enum, and widget hints.
- The compiler applies defaults to the render model.
- No plugin id or Privacy Filter-specific rule appears in this file.

- [ ] **Step 6: Commit this unit**

Run:

```bash
git add src/pages/plugins/pluginConfigValidation.ts src/pages/plugins/pluginConfigRenderModel.ts src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts
git commit -m "feat: compile plugin config render model"
```

Expected:

```text
[branch ...] feat: compile plugin config render model
```

## Task 3: Render Low-Code Config Widgets

**Boundary:** Generic `PluginConfigSchemaForm` rendering only. Do not change `PluginsPage` and do not add Privacy Filter-specific components.

**Files:**
- Modify: `src/pages/plugins/PluginConfigSchemaForm.tsx`
- Modify: `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`

- [ ] **Step 1: Write failing tests**

Add to `src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx`:

```tsx
it("renders titled sections, descriptions, switches, selects, and checkbox groups", () => {
  const onSubmit = vi.fn();

  render(
    <PluginConfigSchemaForm
      identity="publisher.sample:1"
      schema={{
        type: "object",
        required: ["redactBeforeUpstream"],
        "x-aio-ui": {
          sections: [
            { id: "routing", title: "处理位置", description: "选择插件在哪些阶段生效。", order: 10 },
            { id: "content", title: "要保护的内容", order: 20 },
          ],
        },
        properties: {
          redactBeforeUpstream: {
            type: "boolean",
            title: "发送给模型前处理",
            description: "在请求离开本机前替换敏感内容。",
            default: true,
            "x-aio-ui": { section: "routing", widget: "switch", order: 10 },
          },
          profile: {
            type: "string",
            title: "保护强度",
            default: "balanced",
            enum: ["balanced", "strict"],
            "x-aio-ui": {
              section: "routing",
              widget: "select",
              order: 20,
              enumLabels: { balanced: "平衡", strict: "严格" },
            },
          },
          sensitiveTypes: {
            type: "array",
            title: "要保护的内容",
            default: ["email", "cn_phone"],
            items: {
              type: "string",
              enum: ["email", "cn_phone"],
              "x-aio-ui": {
                enumLabels: { email: "邮箱地址", cn_phone: "中国手机号" },
                enumDescriptions: {
                  email: "例如 name@example.com。",
                  cn_phone: "例如 13344441520。",
                },
              },
            },
            "x-aio-ui": {
              section: "content",
              widget: "checkboxGroup",
              warningWhenPartial: "关闭后，这类内容会原样发送给模型。",
            },
          },
        },
      }}
      value={{ sensitiveTypes: ["email", "cn_phone"] }}
      pending={false}
      onSubmit={onSubmit}
    />
  );

  expect(screen.getByText("处理位置")).toBeInTheDocument();
  expect(screen.getByText("选择插件在哪些阶段生效。")).toBeInTheDocument();
  expect(screen.getByText("要保护的内容")).toBeInTheDocument();
  expect(screen.getByLabelText("发送给模型前处理 *")).toBeChecked();
  expect(screen.getByRole("combobox", { name: "保护强度" })).toHaveValue("balanced");
  expect(screen.getByText("平衡")).toBeInTheDocument();
  expect(screen.getByLabelText("邮箱地址")).toBeChecked();
  expect(screen.getByText("例如 name@example.com。")).toBeInTheDocument();

  fireEvent.click(screen.getByLabelText("邮箱地址"));
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

  expect(onSubmit).toHaveBeenCalledWith({
    redactBeforeUpstream: true,
    profile: "balanced",
    sensitiveTypes: ["cn_phone"],
  });
});

it("keeps unsupported object fields editable through json fallback", () => {
  const onSubmit = vi.fn();

  render(
    <PluginConfigSchemaForm
      identity="publisher.advanced:1"
      schema={{
        type: "object",
        properties: {
          advanced: { type: "object", title: "高级配置" },
        },
      }}
      value={{ advanced: { retries: 2 } }}
      pending={false}
      onSubmit={onSubmit}
    />
  );

  fireEvent.change(screen.getByLabelText("高级配置"), {
    target: { value: "{\"retries\":3}" },
  });
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

  expect(onSubmit).toHaveBeenCalledWith({ advanced: { retries: 3 } });
});
```

- [ ] **Step 2: Run the form test and verify RED**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

Expected:

```text
FAIL src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
Unable to find an element with the text: 处理位置
```

- [ ] **Step 3: Update the form to use the render model**

Modify `src/pages/plugins/PluginConfigSchemaForm.tsx`:

```tsx
// Add imports.
import {
  buildPluginConfigRenderModel,
  type PluginConfigFieldModel,
} from "./pluginConfigRenderModel";
```

Replace direct `schemaProperties`, `schemaRequired`, and raw property iteration with:

```tsx
const model = useMemo(() => buildPluginConfigRenderModel({ schema, value: draft }), [schema, draft]);
```

Keep this empty state:

```tsx
if (!model.editable) {
  return (
    <div className="space-y-3">
      <div className="text-sm text-muted-foreground">此插件没有可编辑配置。</div>
      <pre className="max-h-48 overflow-auto rounded-md border border-border bg-muted/40 p-3 text-xs">
        {JSON.stringify(value ?? {}, null, 2)}
      </pre>
    </div>
  );
}
```

Add a local renderer:

```tsx
function renderField(field: PluginConfigFieldModel) {
  const label = field.label;
  const current = field.value;

  if (field.widget === "switch") {
    return (
      <label
        key={field.key}
        className="flex items-center justify-between gap-3 rounded-md border border-border px-3 py-2"
      >
        <span>
          <span className="block text-sm font-medium">{label}</span>
          {field.description ? (
            <span className="block text-xs text-muted-foreground">{field.description}</span>
          ) : null}
        </span>
        <Switch
          aria-label={label}
          checked={Boolean(current)}
          onCheckedChange={(checked) => setField(field.key, checked)}
        />
      </label>
    );
  }

  if (field.widget === "select") {
    return (
      <label key={field.key} className="grid gap-1.5 text-sm">
        <span className="font-medium">{label}</span>
        {field.description ? <span className="text-xs text-muted-foreground">{field.description}</span> : null}
        <select
          aria-label={label}
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={fieldToText(current, field.type)}
          onChange={(event) => setField(field.key, event.target.value)}
        >
          {field.options.map((option) => (
            <option key={String(option.value)} value={String(option.value)}>
              {option.label}
            </option>
          ))}
        </select>
        {field.warning ? <span className="text-xs text-warning">{field.warning}</span> : null}
      </label>
    );
  }

  if (field.widget === "checkboxGroup") {
    const currentArray = Array.isArray(current) ? current : [];
    return (
      <fieldset key={field.key} className="grid gap-2 rounded-md border border-border px-3 py-2">
        <legend className="px-1 text-sm font-medium">{label}</legend>
        {field.description ? <div className="text-xs text-muted-foreground">{field.description}</div> : null}
        {field.options.map((option) => {
          const itemText = String(option.value);
          const checked = currentArray.some((value) => String(value) === itemText);
          return (
            <label key={itemText} className="flex items-start gap-2 text-sm">
              <input
                aria-label={option.label}
                className="mt-1"
                type="checkbox"
                checked={checked}
                onChange={(event) => {
                  const next = event.target.checked
                    ? [...currentArray, option.value]
                    : currentArray.filter((value) => String(value) !== itemText);
                  setField(field.key, next);
                }}
              />
              <span>
                <span className="block font-medium">{option.label}</span>
                {option.description ? (
                  <span className="block text-xs text-muted-foreground">{option.description}</span>
                ) : null}
              </span>
            </label>
          );
        })}
        {field.warning && currentArray.length < field.options.length ? (
          <div className="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            {field.warning}
          </div>
        ) : null}
      </fieldset>
    );
  }

  if (field.widget === "json") {
    return (
      <label key={field.key} className="grid gap-1.5 text-sm">
        <span className="font-medium">{label}</span>
        {field.description ? <span className="text-xs text-muted-foreground">{field.description}</span> : null}
        <Textarea
          aria-label={label}
          value={fieldToText(current, field.type)}
          onChange={(event) => setField(field.key, coerceConfigField(event.target.value, field.type))}
        />
      </label>
    );
  }

  return (
    <label key={field.key} className="grid gap-1.5 text-sm">
      <span className="font-medium">{label}</span>
      {field.description ? <span className="text-xs text-muted-foreground">{field.description}</span> : null}
      <Input
        aria-label={label}
        placeholder={field.placeholder ?? undefined}
        type={
          field.widget === "password"
            ? "password"
            : field.widget === "number"
              ? "number"
              : "text"
        }
        value={fieldToText(current, field.type)}
        onChange={(event) => setField(field.key, coerceConfigField(event.target.value, field.type))}
      />
      {field.warning ? <span className="text-xs text-warning">{field.warning}</span> : null}
    </label>
  );
}
```

Render sections:

```tsx
<div className="grid gap-4">
  {model.sections.map((section) => (
    <section key={section.id} className="grid gap-3">
      <div>
        <h3 className="text-sm font-semibold text-foreground">{section.title}</h3>
        {section.description ? (
          <p className="text-xs text-muted-foreground">{section.description}</p>
        ) : null}
      </div>
      <div className="grid gap-3">{section.fields.map(renderField)}</div>
    </section>
  ))}
</div>
```

- [ ] **Step 4: Run the form tests and verify GREEN**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

Expected:

```text
PASS src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
```

**Acceptance Criteria:**
- Generic schema metadata renders user-facing titles and descriptions.
- Boolean fields render as switches.
- Scalar enums render as selects with option labels.
- Array enums render as checkbox groups with option labels/descriptions.
- Unsupported structured values remain editable through JSON fallback.
- Form submits the same config object shape expected by `plugin_save_config`.
- No plugin id branch exists in `PluginConfigSchemaForm`.

- [ ] **Step 5: Commit this unit**

Run:

```bash
git add src/pages/plugins/PluginConfigSchemaForm.tsx src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
git commit -m "feat: render low-code plugin config fields"
```

Expected:

```text
[branch ...] feat: render low-code plugin config fields
```

## Task 4: Productize PluginsPage Without Plugin-Specific Config Branches

**Boundary:** Main page composition and copy only. Do not add a dedicated Privacy Filter config component.

**Files:**
- Create: `src/pages/plugins/pluginProductCopy.ts`
- Create: `src/pages/plugins/__tests__/pluginProductCopy.test.ts`
- Modify: `src/pages/PluginsPage.tsx`
- Modify: `src/pages/__tests__/PluginsPage.test.tsx`

- [ ] **Step 1: Write failing tests for product copy helpers**

Create `src/pages/plugins/__tests__/pluginProductCopy.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  describePluginPermission,
  describePluginRuntime,
  pluginRiskLabel,
  pluginStatusLabel,
} from "../pluginProductCopy";

describe("pluginProductCopy", () => {
  it("translates plugin statuses into user-facing Chinese labels", () => {
    expect(pluginStatusLabel("enabled")).toBe("运行中");
    expect(pluginStatusLabel("disabled")).toBe("已关闭");
    expect(pluginStatusLabel("quarantined")).toBe("已隔离");
  });

  it("translates permission ids into user impact copy", () => {
    expect(describePluginPermission("request.body.read")).toEqual({
      label: "读取你发送给模型的内容",
      detail: "用于检查或分析请求正文。",
      risk: "high",
    });
    expect(describePluginPermission("request.body.write")).toEqual({
      label: "修改你发送给模型的内容",
      detail: "用于在发送前替换、追加或删除请求正文。",
      risk: "high",
    });
    expect(describePluginPermission("log.redact")).toEqual({
      label: "处理本地请求日志",
      detail: "用于在日志保存前隐藏敏感信息。",
      risk: "medium",
    });
  });

  it("describes runtimes without making implementation jargon primary", () => {
    expect(describePluginRuntime("native:privacyFilter")).toEqual({
      label: "内置隐私过滤引擎",
      detail: "由 AIO Coding Hub 提供，用于本地处理。",
    });
    expect(describePluginRuntime("declarativeRules")).toEqual({
      label: "规则插件",
      detail: "根据声明式规则处理请求、响应或日志。",
    });
  });

  it("maps risk levels to readable labels", () => {
    expect(pluginRiskLabel("low")).toBe("低风险");
    expect(pluginRiskLabel("medium")).toBe("中风险");
    expect(pluginRiskLabel("high")).toBe("高风险");
    expect(pluginRiskLabel("critical")).toBe("关键风险");
  });
});
```

- [ ] **Step 2: Run copy tests and verify RED**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginProductCopy.test.ts
```

Expected:

```text
FAIL src/pages/plugins/__tests__/pluginProductCopy.test.ts
Cannot find module '../pluginProductCopy'
```

- [ ] **Step 3: Implement copy helpers**

Create `src/pages/plugins/pluginProductCopy.ts` with status, risk, runtime, and permission copy mappings. Use the same function names from the tests. Keep this file focused on host-level concepts such as permissions and runtime. Do not include Privacy Filter config strategy ids.

- [ ] **Step 4: Write failing page tests**

Modify `src/pages/__tests__/PluginsPage.test.tsx` by adding:

```tsx
it("presents plugin value, data access, settings, and developer metadata in that order", () => {
  vi.mocked(usePluginsListQuery).mockReturnValue({
    data: [summary()],
    isLoading: false,
    isFetching: false,
    error: null,
  } as any);

  renderWithProviders(<PluginsPage />);

  expect(screen.getByText("这个插件会做什么")).toBeInTheDocument();
  expect(screen.getByText("数据访问")).toBeInTheDocument();
  expect(screen.getByText("设置")).toBeInTheDocument();
  expect(screen.getByText("开发者信息")).toBeInTheDocument();
  expect(screen.getByText("读取你发送给模型的内容")).toBeInTheDocument();
});

it("uses the generic schema form for official plugin configuration", () => {
  vi.mocked(usePluginsListQuery).mockReturnValue({
    data: [
      summary({
        plugin_id: "official.privacy-filter",
        name: "Privacy Filter",
        runtime: "native:privacyFilter",
      }),
    ],
    isLoading: false,
    isFetching: false,
    error: null,
  } as any);
  vi.mocked(usePluginQuery).mockReturnValue({
    data: detail({
      summary: summary({
        plugin_id: "official.privacy-filter",
        name: "Privacy Filter",
        runtime: "native:privacyFilter",
      }),
      manifest: {
        ...detail().manifest,
        id: "official.privacy-filter",
        name: "Privacy Filter",
        runtime: { kind: "native", engine: "privacyFilter" },
        permissions: ["request.body.read", "request.body.write", "log.redact"],
        configSchema: {
          type: "object",
          properties: {
            sensitiveTypes: {
              type: "array",
              title: "要保护的内容",
              items: {
                type: "string",
                enum: ["email", "cn_phone"],
                "x-aio-ui": {
                  enumLabels: { email: "邮箱地址", cn_phone: "中国手机号" },
                },
              },
              "x-aio-ui": { widget: "checkboxGroup" },
            },
          },
        },
      },
      config: { sensitiveTypes: ["email", "cn_phone"] },
      granted_permissions: ["request.body.read", "request.body.write", "log.redact"],
    }),
    isLoading: false,
    isFetching: false,
    error: null,
  } as any);

  renderWithProviders(<PluginsPage />);

  expect(screen.getByLabelText("邮箱地址")).toBeChecked();
  expect(screen.queryByLabelText("sensitiveTypes")).not.toBeInTheDocument();
});
```

- [ ] **Step 5: Run page tests and verify RED**

Run:

```bash
pnpm test:unit src/pages/__tests__/PluginsPage.test.tsx
```

Expected:

```text
FAIL src/pages/__tests__/PluginsPage.test.tsx
Unable to find an element with the text: 这个插件会做什么
```

- [ ] **Step 6: Update `PluginsPage.tsx` imports and copy**

Add:

```tsx
import {
  describePluginPermission,
  describePluginRuntime,
  pluginRiskLabel,
  pluginStatusLabel,
} from "./plugins/pluginProductCopy";
```

Use:
- `pluginStatusLabel(plugin.status)` instead of raw status copy.
- `pluginRiskLabel(plugin.permission_risk)` in risk pills.
- `describePluginRuntime(plugin.runtime).label` in the list.
- `describePluginPermission(permission)` in permission rows.

Update primary sections:
- `这个插件会做什么`
- `数据访问`
- `设置`
- `开发者信息`

Keep:

```tsx
<PluginConfigSchemaForm
  identity={`${detail.summary.plugin_id}:${detail.manifest.configVersion ?? 1}:${detail.summary.updated_at}`}
  schema={detail.manifest.configSchema}
  value={detail.config}
  pending={savingConfig}
  onSubmit={onSaveConfig}
/>
```

Do not add any branch like:

```tsx
detail.summary.plugin_id === "official.privacy-filter"
```

- [ ] **Step 7: Run copy and page tests and verify GREEN**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginProductCopy.test.ts src/pages/__tests__/PluginsPage.test.tsx
```

Expected:

```text
PASS
```

**Acceptance Criteria:**
- The page explains plugin value and data access in user-facing Chinese before developer metadata.
- Configuration rendering remains generic through `PluginConfigSchemaForm`.
- Official plugins and community plugins use the same config rendering path.
- No Privacy Filter-specific config branch exists in `PluginsPage`.
- Empty state no longer tells users to import `plugin.json`; it should mention `.aio-plugin`.

- [ ] **Step 8: Commit this unit**

Run:

```bash
git add src/pages/plugins/pluginProductCopy.ts src/pages/plugins/__tests__/pluginProductCopy.test.ts src/pages/PluginsPage.tsx src/pages/__tests__/PluginsPage.test.tsx
git commit -m "feat: productize plugin settings page"
```

Expected:

```text
[branch ...] feat: productize plugin settings page
```

## Task 5: Model Privacy Filter As A Schema-Driven Example

**Boundary:** Fixture and official example schema only. Do not add app behavior and do not add a Privacy Filter-specific React component.

**Files:**
- Modify: `src/test/msw/state.ts`
- Modify: `src/query/__tests__/plugins.test.tsx` or `src/services/__tests__/plugins.test.ts`
- Modify: `src/pages/__tests__/PluginsPage.test.tsx` only if a page fixture assertion is needed.

- [ ] **Step 1: Write or update fixture assertion**

In the existing official install/detail test, assert that the official Privacy Filter schema exposes generic low-code metadata:

```ts
expect(result.manifest.configSchema).toMatchObject({
  type: "object",
  properties: {
    redactBeforeUpstream: {
      type: "boolean",
      title: "发送给模型前处理",
      "x-aio-ui": { widget: "switch" },
    },
    redactLogs: {
      type: "boolean",
      title: "保存日志前处理",
      "x-aio-ui": { widget: "switch" },
    },
    sensitiveTypes: {
      type: "array",
      title: "要保护的内容",
      "x-aio-ui": { widget: "checkboxGroup" },
    },
  },
});
```

- [ ] **Step 2: Run the relevant test and verify RED**

Run one of:

```bash
pnpm test:unit src/query/__tests__/plugins.test.tsx
```

or:

```bash
pnpm test:unit src/services/__tests__/plugins.test.ts
```

Expected:

```text
FAIL ... expected configSchema to match object
```

- [ ] **Step 3: Update official Privacy Filter fixture schema**

In `src/test/msw/state.ts`, make the official Privacy Filter `configSchema` use the current backend config shape with low-code UI metadata:

```ts
configSchema: {
  type: "object",
  required: ["redactBeforeUpstream", "redactLogs", "profile"],
  "x-aio-ui": {
    sections: [
      {
        id: "routing",
        title: "处理位置",
        description: "选择隐私过滤在哪些阶段生效。",
        order: 10,
      },
      {
        id: "content",
        title: "要保护的内容",
        description: "选择需要自动替换的敏感信息类型。",
        order: 20,
      },
    ],
  },
  properties: {
    redactBeforeUpstream: {
      type: "boolean",
      title: "发送给模型前处理",
      description: "在请求离开本机前替换你选择的敏感信息。",
      default: true,
      "x-aio-ui": { section: "routing", widget: "switch", order: 10 },
    },
    redactLogs: {
      type: "boolean",
      title: "保存日志前处理",
      description: "在本地日志写入前替换你选择的敏感信息。",
      default: true,
      "x-aio-ui": { section: "routing", widget: "switch", order: 20 },
    },
    profile: {
      type: "string",
      title: "保护强度",
      description: "当前版本提供平衡模式。",
      default: "balanced",
      enum: ["balanced"],
      "x-aio-ui": {
        section: "routing",
        widget: "select",
        order: 30,
        enumLabels: { balanced: "平衡" },
      },
    },
    sensitiveTypes: {
      type: "array",
      title: "要保护的内容",
      description: "关闭某一项后，这类内容不会被该插件处理。",
      default: [
        "email",
        "cn_phone",
        "cn_id_card",
        "bank_card_candidate",
        "ipv4",
        "openai_key",
        "aws_access_key",
        "github_token",
        "google_api_key",
        "slack_token",
        "jwt",
        "private_key",
        "context_secret",
      ],
      items: {
        type: "string",
        enum: [
          "email",
          "cn_phone",
          "cn_id_card",
          "bank_card_candidate",
          "ipv4",
          "openai_key",
          "aws_access_key",
          "github_token",
          "google_api_key",
          "slack_token",
          "jwt",
          "private_key",
          "context_secret",
        ],
        "x-aio-ui": {
          enumLabels: {
            email: "邮箱地址",
            cn_phone: "中国手机号",
            cn_id_card: "身份证号",
            bank_card_candidate: "银行卡号",
            ipv4: "IP 地址",
            openai_key: "OpenAI Key",
            aws_access_key: "AWS Access Key",
            github_token: "GitHub Token",
            google_api_key: "Google API Key",
            slack_token: "Slack Token",
            jwt: "JWT",
            private_key: "私钥片段",
            context_secret: "上下文密钥",
          },
          enumDescriptions: {
            email: "例如 name@example.com。",
            cn_phone: "例如 13344441520。",
            cn_id_card: "中国大陆居民身份证号码。",
            bank_card_candidate: "通过校验规则识别常见银行卡号。",
            ipv4: "例如 192.168.1.10。",
            openai_key: "常见 sk- 开头的 OpenAI 密钥。",
            aws_access_key: "常见 AKIA 开头的访问密钥。",
            github_token: "ghp、github_pat 等令牌。",
            google_api_key: "常见 AIza 开头的 Google API Key。",
            slack_token: "Slack bot、user、app token。",
            jwt: "常见 JSON Web Token。",
            private_key: "PEM 私钥内容。",
            context_secret: "password、api_key、token 等上下文中的敏感值。",
          },
        },
      },
      "x-aio-ui": {
        section: "content",
        widget: "checkboxGroup",
        order: 10,
        warningWhenPartial: "关闭后，这类内容会原样发送给模型，也可能出现在本地日志中。",
      },
    },
  },
},
config: {
  redactBeforeUpstream: true,
  redactLogs: true,
  profile: "balanced",
  sensitiveTypes: [
    "email",
    "cn_phone",
    "cn_id_card",
    "bank_card_candidate",
    "ipv4",
    "openai_key",
    "aws_access_key",
    "github_token",
    "google_api_key",
    "slack_token",
    "jwt",
    "private_key",
    "context_secret",
  ],
},
```

- [ ] **Step 4: Run fixture and page tests and verify GREEN**

Run:

```bash
pnpm test:unit src/query/__tests__/plugins.test.tsx src/services/__tests__/plugins.test.ts src/pages/__tests__/PluginsPage.test.tsx
```

Expected:

```text
PASS
```

**Acceptance Criteria:**
- Privacy Filter config UI is fully driven by schema metadata.
- Users can disable only email processing through the generic checkbox group.
- Saved config remains the existing backend shape with `sensitiveTypes`.
- No new Privacy Filter-specific frontend component or helper is created.

- [ ] **Step 5: Commit this unit**

Run:

```bash
git add src/test/msw/state.ts src/query/__tests__/plugins.test.tsx src/services/__tests__/plugins.test.ts src/pages/__tests__/PluginsPage.test.tsx
git commit -m "test: model privacy filter as schema-driven config"
```

Expected:

```text
[branch ...] test: model privacy filter as schema-driven config
```

## Task 6: Document Generic Plugin Config UI Metadata

**Boundary:** Documentation only. Do not change runtime or frontend behavior.

**Files:**
- Modify: `docs/plugins/config-schema.md`
- Modify: `docs/plugins/manifest.md`
- Modify: `docs/plugins/official-examples.md`
- Modify: `docs/plugins/sdk.md`

- [ ] **Step 1: Update config schema guide**

In `docs/plugins/config-schema.md`, document:

```markdown
## UI Metadata

The host renders `configSchema` as a low-code settings panel. Plugin authors should prefer standard JSON Schema fields first:

- `title`: user-facing field name
- `description`: helper text below the title
- `default`: value used when the saved config omits the field
- `enum`: allowed values
- `required`: required object properties

AIO Coding Hub also supports the vendor extension `x-aio-ui` for presentation hints. These hints do not change backend validation.

Supported root `x-aio-ui` fields:

- `sections`: ordered groups of fields

Supported field `x-aio-ui` fields:

- `section`: section id
- `order`: numeric order inside a section
- `widget`: `text`, `textarea`, `password`, `number`, `switch`, `select`, `checkboxGroup`, or `json`
- `placeholder`: input placeholder for text-like fields
- `warning`: always-visible warning copy
- `warningWhenPartial`: warning copy shown when a checkbox group is partially selected
- `enumLabels`: map enum values to user-facing labels
- `enumDescriptions`: map enum values to helper text

The host may ignore an incompatible widget hint. For example, `checkboxGroup` only applies to `array` fields whose `items.enum` is present.
```

Add the Privacy Filter-style example from the Core Design section, shortened if needed.

- [ ] **Step 2: Update manifest and SDK docs**

In `docs/plugins/manifest.md`, add a short pointer:

```markdown
`configSchema` may include standard JSON Schema presentation fields and AIO `x-aio-ui` metadata. See [Config Schema](./config-schema.md).
```

In `docs/plugins/sdk.md`, update the TypeScript manifest example to include `title`, `description`, and at least one `x-aio-ui.widget` field.

- [ ] **Step 3: Update official example docs**

In `docs/plugins/official-examples.md`, explain that Privacy Filter demonstrates:

- native official runtime;
- schema-driven configuration UI;
- checkbox group config for strategy selection;
- no host-side plugin-specific page component.

- [ ] **Step 4: Verify documentation has no old guidance**

Run:

```bash
rg -n "redactEmails|redactSecrets|PrivacyFilterConfigPanel|plugin-specific React|sensitiveTypes raw JSON" docs/plugins docs/plugin-manifest-v1.md || true
```

Expected:

```text
```

- [ ] **Step 5: Commit this unit**

Run:

```bash
git add docs/plugins/config-schema.md docs/plugins/manifest.md docs/plugins/official-examples.md docs/plugins/sdk.md
git commit -m "docs: document plugin config ui metadata"
```

Expected:

```text
[branch ...] docs: document plugin config ui metadata
```

**Acceptance Criteria:**
- Plugin authors can understand how to create titled, grouped, low-code config forms.
- Docs clearly say `x-aio-ui` is a presentation hint and does not change validation.
- Docs do not imply Privacy Filter needs host-specific UI code.

## Task 7: Final Verification and Product Acceptance

**Boundary:** Verification only. No new code unless a verification failure reveals a root cause; if so, write a failing test before fixing.

- [ ] **Step 1: Run focused frontend tests**

Run:

```bash
pnpm test:unit src/pages/plugins/__tests__/pluginConfigUiSchema.test.ts
pnpm test:unit src/pages/plugins/__tests__/pluginConfigRenderModel.test.ts
pnpm test:unit src/pages/plugins/__tests__/PluginConfigSchemaForm.test.tsx
pnpm test:unit src/pages/plugins/__tests__/pluginProductCopy.test.ts
pnpm test:unit src/pages/__tests__/PluginsPage.test.tsx
```

Expected:

```text
PASS
```

- [ ] **Step 2: Run broader plugin frontend tests**

Run:

```bash
pnpm test:unit src/query/__tests__/plugins.test.tsx src/services/__tests__/plugins.test.ts
```

Expected:

```text
PASS
```

- [ ] **Step 3: Run source typecheck**

Run:

```bash
pnpm typecheck
```

Expected:

```text
exit code 0
```

- [ ] **Step 4: Run lint**

Run:

```bash
pnpm lint
```

Expected:

```text
exit code 0
```

- [ ] **Step 5: Run plugin backend regression tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml plugin --lib
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Manual product acceptance checklist**

Open the Plugins page in the app or component test harness and verify:

- The empty state says `.aio-plugin`, not `plugin.json`.
- Official Privacy Filter is presented as a recommended plugin.
- A non-developer can understand what the selected plugin does without reading hook/runtime names.
- Permission rows explain user impact in Chinese and still show raw permission ids as secondary detail.
- Settings render field titles and descriptions from `configSchema`.
- Boolean config fields render as switches.
- Scalar enum fields render as selects with user-facing option labels.
- Array enum fields render as checkbox groups with option descriptions.
- Unsupported object/array config fields remain editable through JSON fallback.
- A community plugin with the same schema shape receives the same template-rendered controls as an official plugin.
- Privacy Filter lets the user disable only `邮箱地址` and save.
- The saved Privacy Filter config contains `sensitiveTypes` without `"email"`.
- Privacy Filter can re-enable `邮箱地址` and save.
- Developer metadata is still available under `开发者信息`.
- Existing enable/disable/uninstall/update/rollback buttons still work.

**Final Acceptance Criteria:**
- The page no longer feels like a raw manifest inspector for primary workflows.
- Plugin configuration is generic and schema/template-driven.
- The template renderer supports user input fields, switches, selects, checkbox groups, titles, descriptions, grouping, ordering, warnings, and JSON fallback without plugin-specific React code.
- Privacy Filter uses the same configuration renderer as third-party plugins.
- Backend config compatibility is preserved.
- Plugin authors have documentation for building titled, grouped, low-code settings forms.
- All verification commands in this task pass with fresh output.

## Execution Notes

- Implement one task at a time.
- For every behavior change, follow Red-Green-Refactor:
  - write the failing test;
  - run it and confirm the expected failure;
  - implement the smallest code change;
  - run the same test until it passes;
  - only then move to the next task.
- Do not commit between red and green.
- Do not add plugin-specific config branches to `PluginsPage`.
- Do not create `PrivacyFilterConfigPanel`.
- Do not put Privacy Filter strategy ids in generic renderer code.
- Keep user-facing copy in Simplified Chinese.
- Keep raw developer identifiers visible only as secondary detail.

## Plan Self-Review

- Spec coverage: covers generic low-code config rendering, user input, switches, selects, checkbox groups, titles, descriptions, grouping, warnings, Privacy Filter as schema-driven acceptance example, and developer documentation.
- Placeholder scan: no `TODO`, `TBD`, `implement later`, or unspecified implementation steps are intentionally left in this plan.
- Type consistency: render model consistently uses `PluginConfigRenderModel`, `PluginConfigSectionModel`, `PluginConfigFieldModel`, and `PluginConfigWidget`; UI metadata consistently uses `x-aio-ui`.
- Generality check: no planned runtime code path should branch on `official.privacy-filter` for configuration rendering.
- Superpowers compliance: every development unit has RED, GREEN, acceptance criteria, and a commit boundary before the next unit begins.
