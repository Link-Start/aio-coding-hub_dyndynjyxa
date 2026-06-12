// Usage: Render and edit the manifest configSchema subset supported by the plugin host.

import { useMemo, useState } from "react";
import type { JsonValue } from "../../services/plugins";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { Textarea } from "../../ui/Textarea";
import {
  coerceConfigField,
  isRecord,
  schemaEnum,
  schemaProperties,
  schemaRequired,
  schemaType,
  type PluginConfigObject,
} from "./pluginConfigValidation";

export type PluginConfigSchemaFormProps = {
  schema: JsonValue | null | undefined;
  value: JsonValue;
  pending: boolean;
  onSubmit: (value: JsonValue) => void;
};

function fieldToText(value: JsonValue | undefined, type: string | null): string {
  if (value == null) return "";
  if (type === "object" || type === "array") {
    return JSON.stringify(value, null, 2);
  }
  return String(value);
}

function initialObject(value: JsonValue): PluginConfigObject {
  return isRecord(value) ? { ...value } : {};
}

export function PluginConfigSchemaForm({
  schema,
  value,
  pending,
  onSubmit,
}: PluginConfigSchemaFormProps) {
  const properties = useMemo(() => schemaProperties(schema), [schema]);
  const required = useMemo(() => schemaRequired(schema), [schema]);
  const [draft, setDraft] = useState<PluginConfigObject>(() => initialObject(value));
  const propertyEntries = Object.entries(properties);

  if (schemaType(schema) !== "object" || propertyEntries.length === 0) {
    return (
      <div className="space-y-3">
        <div className="text-sm text-muted-foreground">此插件没有可编辑配置。</div>
        <pre className="max-h-48 overflow-auto rounded-md border border-border bg-muted/40 p-3 text-xs">
          {JSON.stringify(value ?? {}, null, 2)}
        </pre>
      </div>
    );
  }

  function setField(key: string, next: JsonValue) {
    setDraft((current) => ({ ...current, [key]: next }));
  }

  return (
    <form
      className="space-y-4"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(draft);
      }}
    >
      <div className="grid gap-3">
        {propertyEntries.map(([key, fieldSchema]) => {
          const type = schemaType(fieldSchema);
          const enumValues = schemaEnum(fieldSchema);
          const current = draft[key];
          const label = required.has(key) ? `${key} *` : key;

          if (type === "boolean") {
            return (
              <label
                key={key}
                className="flex items-center justify-between gap-3 rounded-md border border-border px-3 py-2"
              >
                <span className="text-sm font-medium">{label}</span>
                <Switch
                  aria-label={key}
                  checked={Boolean(current)}
                  onCheckedChange={(checked) => setField(key, checked)}
                />
              </label>
            );
          }

          if (enumValues.length > 0) {
            return (
              <label key={key} className="grid gap-1.5 text-sm">
                <span className="font-medium">{label}</span>
                <select
                  aria-label={key}
                  className="h-9 rounded-md border border-input bg-background px-3 text-sm"
                  value={fieldToText(current, type)}
                  onChange={(event) => setField(key, event.target.value)}
                >
                  {enumValues.map((item) => (
                    <option key={String(item)} value={String(item)}>
                      {String(item)}
                    </option>
                  ))}
                </select>
              </label>
            );
          }

          if (type === "object" || type === "array") {
            return (
              <label key={key} className="grid gap-1.5 text-sm">
                <span className="font-medium">{label}</span>
                <Textarea
                  aria-label={key}
                  value={fieldToText(current, type)}
                  onChange={(event) => setField(key, coerceConfigField(event.target.value, type))}
                />
              </label>
            );
          }

          return (
            <label key={key} className="grid gap-1.5 text-sm">
              <span className="font-medium">{label}</span>
              <Input
                aria-label={key}
                type={
                  type === "password"
                    ? "password"
                    : type === "number" || type === "integer"
                      ? "number"
                      : "text"
                }
                value={fieldToText(current, type)}
                onChange={(event) => setField(key, coerceConfigField(event.target.value, type))}
              />
            </label>
          );
        })}
      </div>

      <div className="flex justify-end">
        <Button type="submit" disabled={pending}>
          保存配置
        </Button>
      </div>
    </form>
  );
}
