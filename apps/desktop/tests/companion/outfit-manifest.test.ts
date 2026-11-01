import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { test } from "node:test";
import { OUTFITS } from "../../src/features/companion/catalog.ts";

const folder = new URL("../../../../web/src/assets/outfits/", import.meta.url);

function validate(manifest: unknown, imageExists: (id: string) => boolean) {
  assert.deepEqual(manifest, OUTFITS);
  for (const { id } of OUTFITS) assert.ok(imageExists(id), `Missing image: ${id}`);
}

test("outfit manifest matches the catalog and every image exists", () => {
  const manifest: unknown = JSON.parse(readFileSync(new URL("manifest.json", folder), "utf8"));
  validate(manifest, (id) => existsSync(new URL(`${id}.png`, folder)));
});

test("outfit manifest rejects changed ids, labels, order and missing images", () => {
  const fixture = OUTFITS.map((outfit) => ({ ...outfit }));
  assert.throws(() => validate([{ ...fixture[0], id: "wrong" }, ...fixture.slice(1)], () => true));
  assert.throws(() => validate([{ ...fixture[0], label: "Wrong" }, ...fixture.slice(1)], () => true));
  assert.throws(() => validate([fixture[1], fixture[0], ...fixture.slice(2)], () => true));
  assert.throws(() => validate(fixture, (id) => id !== fixture[0].id));
});
