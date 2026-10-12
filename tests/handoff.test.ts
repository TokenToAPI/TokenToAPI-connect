import assert from "node:assert/strict";
import test from "node:test";
import { handoffSelection, shouldAutoAccept } from "../src/handoff.ts";

test("handoff selects only fetched compatible models and a selected default", () => {
  const models = [{ id: "gpt-5.5", name: "GPT" }, { id: "claude-sonnet-4-6", name: "Sonnet" }];
  assert.deepEqual(handoffSelection("claude-desktop", models), { ids: ["claude-sonnet-4-6"], defaultId: "claude-sonnet-4-6" });
  assert.deepEqual(handoffSelection("codex", models), { ids: ["gpt-5.5", "claude-sonnet-4-6"], defaultId: "gpt-5.5" });
});
test("auto handoff never replaces a busy operation or an edited wizard", () => {
  assert.equal(shouldAutoAccept(true, false, false), true);
  assert.equal(shouldAutoAccept(false, false, false), false);
  assert.equal(shouldAutoAccept(true, true, false), false);
  assert.equal(shouldAutoAccept(true, false, true), false);
});
