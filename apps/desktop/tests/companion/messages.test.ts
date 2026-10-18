import { test } from "node:test";
import assert from "node:assert/strict";
import {
  createMessageSlot,
  receiveMessage,
  visibleMessage,
} from "../../src/features/companion/animation/messages.ts";
test("same ID does not extend expiry and same text new ID does", () => {
  const slot = createMessageSlot();
  receiveMessage(slot, { id: "a", text: "hello" }, 0);
  receiveMessage(slot, { id: "a", text: "hello" }, 3000);
  assert.equal(visibleMessage(slot, 4000), null);
  receiveMessage(slot, { id: "b", text: "hello" }, 4000);
  assert.equal(visibleMessage(slot, 7999)?.id, "b");
  receiveMessage(slot, null, 5000);
  assert.equal(visibleMessage(slot, 6000), null);
});
test("replacement owns its deadline and invalid duration falls back", () => {
  const slot = createMessageSlot();
  receiveMessage(slot, { id: "a", text: "A" }, 0);
  receiveMessage(slot, { id: "b", text: "B", durationMs: NaN }, 3500);
  assert.equal(visibleMessage(slot, 4001)?.text, "B");
  assert.equal(visibleMessage(slot, 7500), null);
  receiveMessage(slot, { id: "c", text: "C", durationMs: 0 }, 8000);
  assert.equal(visibleMessage(slot, 8000), null);
});
