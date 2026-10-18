import { test, expect } from "@playwright/test";
test.setTimeout(60000);
for (const dpr of [1, 2])
  for (const background of ["#100b1b", "#f4effb"])
    test(`narrow mixed text and still at DPR ${dpr} on ${background}`, async ({
      browser,
    }, info) => {
      const context = await browser.newContext({
        baseURL: "http://127.0.0.1:4173",
        viewport: { width: 320, height: 800 },
        deviceScaleFactor: dpr,
        reducedMotion: "reduce",
      });
      const page = await context.newPage();
      const errors: string[] = [];
      page.on("pageerror", (e) => errors.push(String(e)));
      await page.goto("/tests/companion/fixture.html?presence");
      await page.evaluate(
        (color) => (document.body.style.background = color),
        background,
      );
      await page.getByRole("button", { name: "Mixed text" }).click();
      const presence = page.getByLabel("AYLUL companion", { exact: true }),
        bubble = page.getByRole("status", { name: "AYLUL message" });
      await expect(bubble).toBeVisible();
      const p = (await presence.boundingBox())!,
        b = (await bubble.boundingBox())!;
      expect(b.x).toBeGreaterThanOrEqual(p.x);
      expect(b.x + b.width).toBeLessThanOrEqual(p.x + p.width + 0.1);
      expect(b.y + b.height).toBeLessThanOrEqual(p.y + p.height + 0.1);
      const pixels = await page
        .locator("canvas")
        .evaluate((c: HTMLCanvasElement) => {
          const data = c
            .getContext("2d")!
            .getImageData(0, 0, c.width, c.height).data;
          return {
            width: c.width,
            css: c.getBoundingClientRect().width,
            count: data.filter((v, i) => i % 4 === 3 && v > 0).length,
          };
        });
      expect(pixels.count).toBeGreaterThan(100);
      expect(pixels.width).toBeCloseTo(pixels.css * dpr, 0);
      expect(errors).toEqual([]);
      await presence.screenshot({ path: info.outputPath("presence.png") });
      await context.close();
    });
test("review gallery of expressions and outfits plus studio", async ({
  page,
}, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/tests/companion/fixture.html");
  await page.evaluate(async () => {
    const fixture = (window as any).fixture;
    const gallery = document.getElementById("root")!;
    gallery.style.cssText =
      "display:grid;grid-template-columns:repeat(5,160px);gap:8px;background:#140e20;color:#eee;padding:12px;width:840px";
    for (const [expression, outfit] of [
      ...[
        "idle",
        "happy",
        "listening",
        "speaking",
        "thinking",
        "working",
        "success",
        "blocked",
        "sleeping",
      ].map((e) => [e, "default"]),
      ...[
        "crown",
        "moon",
        "scarf",
        "starry",
        "winter",
        "headphones",
        "mask",
        "halo",
        "special",
      ].map((o) => ["idle", o]),
    ]) {
      const result = await fixture.render(expression, outfit);
      const box = document.createElement("div");
      const img = document.createElement("img");
      img.src = result.image;
      img.width = 160;
      box.append(img, document.createTextNode(`${expression} / ${outfit}`));
      gallery.append(box);
    }
  });
  await page
    .locator("#root")
    .screenshot({ path: info.outputPath("gallery.png") });
  await page.goto("/tests/companion/fixture.html?studio");
  await expect(page.getByTestId("preview-grid").locator("canvas")).toHaveCount(
    11,
  );
  await page
    .getByRole("region", { name: "AYLUL development studio" })
    .screenshot({ path: info.outputPath("studio.png") });
});
