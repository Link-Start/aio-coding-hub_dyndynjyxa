import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PluginConfigSchemaForm } from "../PluginConfigSchemaForm";

describe("PluginConfigSchemaForm", () => {
  it("renders object schema fields and submits typed config", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        schema={{
          type: "object",
          required: ["mode", "enabled"],
          properties: {
            mode: { type: "string", enum: ["append_instruction", "rewrite_system_message"] },
            threshold: { type: "integer" },
            enabled: { type: "boolean" },
          },
        }}
        value={{ mode: "append_instruction", threshold: 2, enabled: false }}
        onSubmit={onSubmit}
        pending={false}
      />
    );

    fireEvent.change(screen.getByLabelText("mode"), {
      target: { value: "rewrite_system_message" },
    });
    fireEvent.change(screen.getByLabelText("threshold"), { target: { value: "4" } });
    fireEvent.click(screen.getByLabelText("enabled"));
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({
      mode: "rewrite_system_message",
      threshold: 4,
      enabled: true,
    });
  });
});
