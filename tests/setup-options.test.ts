import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_OPTIONS, selectedOptions } from "../src/setup-options.ts";

test("purpose model overrides cannot survive after the model is unchecked", () => {
  const options = {
    ...DEFAULT_OPTIONS,
    claudeModels: { haiku: "light", subagent: "main" },
  };
  assert.deepEqual(selectedOptions("claude", ["main"], options).claudeModels, {
    subagent: "main",
  });
  assert.deepEqual(options.claudeModels, { haiku: "light", subagent: "main" });
});

test("client changes keep only options supported by that client", () => {
  const options = {
    claudeModels: { haiku: "light" },
    artifactMode: "disabled" as const,
    webSearch: "live" as const,
    toolSearch: "enabled" as const,
  };
  assert.deepEqual(selectedOptions("codex", ["light"], options), {
    ...DEFAULT_OPTIONS,
    webSearch: "live",
  });
  assert.deepEqual(selectedOptions("claude-desktop", ["light"], options), DEFAULT_OPTIONS);
  assert.equal(selectedOptions("claude", ["light"], options).toolSearch, "enabled");
  assert.equal(selectedOptions("codex", ["light"], options).toolSearch, "preserve");
});
