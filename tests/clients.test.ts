import test from "node:test";
import assert from "node:assert/strict";
import { clients, clientFamily, modelSelectable } from "../src/clients.ts";

test("desktop and CLI choices are distinct, and Claude defaults to its desktop entry", () => {
  assert.deepEqual(clients.filter(c => clientFamily(c.id) === "claude").map(c => c.id), ["claude-desktop", "claude", "claude-vscode"]);
  assert.equal(clientFamily("codex-desktop"), "codex");
});
test("Claude Desktop cannot import an incompatible alias through bulk selection", () => {
  for (const id of ["claude-sonnet-5-5", "claude-opus-5-5", "anthropic/claude-haiku-test"]) assert.equal(modelSelectable("claude-desktop", id), true);
  for (const id of ["gpt-test", "custom-alias", "claude-sonnet-", "claude-sonnet-5[1m]"]) assert.equal(modelSelectable("claude-desktop", id), false);
  assert.equal(modelSelectable("claude", "custom-alias"), true);
  assert.equal(modelSelectable("codex-desktop", "custom-alias"), true);
});

test("dedicated media and embedding models are not selectable as chat models", () => {
  for (const id of ["gpt-image-1", "dall-e-3", "whisper-1", "tts-1", "text-embedding-3-large", "sora-2", "grok-imagine-image", "grok-imagine-video", "grok-voice", "xai/grok-imagine-image", "xai/grok-voice"]) {
    for (const client of clients.map((item) => item.id)) {
      assert.equal(modelSelectable(client, id), false, `${client} must reject ${id}`);
    }
  }
  assert.equal(modelSelectable("codex", "gpt-4o"), true);
  assert.equal(modelSelectable("claude", "claude-3-7-sonnet"), true);
});

 test("Grok is a distinct native client and its chat models work in compatible clients", () => {
 assert.equal(clientFamily("grok"), "grok");
 for (const client of ["grok", "codex", "codex-desktop", "claude", "claude-vscode"] as const) {
   for (const id of ["grok-4.3", "grok-4.5", "grok-4.6", "grok-4.7", "grok-build"]) assert.equal(modelSelectable(client, id), true);
 }
 assert.equal(modelSelectable("claude-desktop", "grok-4.7"), false);
 });
