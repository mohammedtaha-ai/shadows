import { expect, test } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { OUTFITS } from "../../src/features/companion/catalog.ts";

test("export outfit previews", async ({ page }, info) => {
  test.skip(process.env.EXPORT_OUTFITS !== "1", "Explicit outfit export only");
  await page.goto("/companion.html");
  const folder = new URL("../../../../web/src/assets/outfits/", import.meta.url);
  mkdirSync(folder, { recursive: true });
  for (const outfit of OUTFITS) {
    const result = await page.evaluate(async (id) => {
      const modulePath = "/src/features/companion/studio/export.ts";
      const { exportOutfitPreview } = await import(/* @vite-ignore */ modulePath);
      const blob: Blob = await exportOutfitPreview(id);
      const image = await createImageBitmap(blob);
      const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
      return { width: image.width, height: image.height, type: blob.type, bytes };
    }, outfit.id);
    expect([result.width, result.height, result.type]).toEqual([160, 160, "image/png"]);
    const image = Buffer.from(result.bytes);
    writeFileSync(new URL(`${outfit.id}.png`, folder), image);
    await info.attach(`${outfit.id}.png`, { body: image, contentType: "image/png" });
  }
  writeFileSync(new URL("manifest.json", folder), `${JSON.stringify(OUTFITS, null, 2)}\n`);
});
