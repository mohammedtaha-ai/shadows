import { test, expect } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/companion/fixture.html");
});
test("all expressions and outfits draw without global engine", async ({
  page,
}) => {
  const samples = await page.evaluate(async () => {
    const fixture = (window as any).fixture;
    const images = [];
    for (const expression of [
      "idle",
      "happy",
      "listening",
      "speaking",
      "thinking",
      "working",
      "success",
      "blocked",
      "sleeping",
    ]) {
      for (const outfit of [
        "default",
        "crown",
        "moon",
        "scarf",
        "starry",
        "winter",
        "headphones",
        "mask",
        "halo",
        "special",
      ]) {
        const result = await fixture.render(expression, outfit);
        images.push(result);
      }
    }
    return images;
  });
  for (const sample of samples) {
    expect(sample.error).toBeUndefined();
    expect(sample.visible).toBeGreaterThan(100);
  }
  expect(new Set(samples.map((s) => s.image)).size).toBeGreaterThan(10);
});
test("same inputs produce same pixels", async ({ page }) => {
  expect(
    await page.evaluate(
      async () => (await (window as any).fixture.render()).deterministic,
    ),
  ).toBe(true);
});
test("character render preserves transparent corners", async ({ page }) => {
  expect(
    await page.evaluate(
      async () => (await (window as any).fixture.render()).corner,
    ),
  ).toBe(0);
});
test("defaults match source", async ({ page }) => {
  const settings = await page.evaluate(
    async () => (await (window as any).fixture.render()).defaults,
  );
  expect(settings).toMatchObject({
    depth3D: 1.35,
    glowIntensity: 1.25,
    floatSpeed: 1,
    earringSwing: 1.25,
  });
});
