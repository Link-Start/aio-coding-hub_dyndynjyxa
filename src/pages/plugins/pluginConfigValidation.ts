// Usage: Small JSON-schema subset helpers for plugin config editing.

import type { JsonValue } from "../../services/plugins";

export type PluginConfigObject = Record<string, JsonValue>;

export function isRecord(value: JsonValue | unknown): value is Record<string, JsonValue> {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

export function schemaType(schema: JsonValue | undefined): string | null {
  if (!isRecord(schema)) return null;
  const type = schema.type;
  return typeof type === "string" ? type : null;
}

export function schemaProperties(schema: JsonValue | undefined): Record<string, JsonValue> {
  if (!isRecord(schema) || !isRecord(schema.properties)) return {};
  return schema.properties;
}

export function schemaRequired(schema: JsonValue | undefined): Set<string> {
  if (!isRecord(schema) || !Array.isArray(schema.required)) return new Set();
  return new Set(schema.required.filter((item): item is string => typeof item === "string"));
}

export function schemaEnum(schema: JsonValue | undefined): JsonValue[] {
  if (!isRecord(schema) || !Array.isArray(schema.enum)) return [];
  return schema.enum as JsonValue[];
}

export function coerceConfigField(raw: string, type: string | null): JsonValue {
  switch (type) {
    case "integer": {
      const parsed = Number.parseInt(raw, 10);
      return Number.isFinite(parsed) ? parsed : 0;
    }
    case "number": {
      const parsed = Number.parseFloat(raw);
      return Number.isFinite(parsed) ? parsed : 0;
    }
    case "array":
    case "object": {
      try {
        return JSON.parse(raw) as JsonValue;
      } catch {
        return raw;
      }
    }
    default:
      return raw;
  }
}
