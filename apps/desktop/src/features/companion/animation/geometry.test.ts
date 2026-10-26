import { test } from "node:test";
import assert from "node:assert/strict";
import {
  backingSize,
  fitCharacter,
  localPointer,
  normalizeViewport,
} from "./geometry.ts";
test("zero size and invalid viewport stay finite", () => {
  assert.deepEqual(normalizeViewport({ width: NaN, height: -1, dpr: 0 }), {
    width: 0,
    height: 0,
    dpr: 1,
  });
  assert.equal(fitCharacter({ width: 0, height: 0, dpr: 1 }).scale, 0);
});
test("DPR doubles pixels while pointer stays in CSS space", () => {
  assert.deepEqual(backingSize({ width: 320, height: 240, dpr: 2 }), {
    width: 640,
    height: 480,
  });
  assert.deepEqual(
    localPointer({ x: 90, y: 100 }, { x: 10, y: 20, width: 320, height: 240 }),
    { x: 80, y: 80 },
  );
  assert.equal(
    localPointer({ x: 0, y: 0 }, { x: 10, y: 20, width: 320, height: 240 }),
    null,
  );
  assert.ok(
    fitCharacter({ width: 160, height: 240, dpr: 2 }).scale <= 160 / 340,
  );
});
