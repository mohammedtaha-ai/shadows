import { test, expect } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const native = CanvasRenderingContext2D.prototype.clearRect;
    Object.assign(window, { draws: 0 });
    CanvasRenderingContext2D.prototype.clearRect = function (...args) {
      (window as any).draws++;
      return native.apply(this, args);
    };
  });
  await page.goto("/tests/companion/fixture.html?presence");
});
test("strict remount owns one loop", async ({ page }) => {
  await expect(page.getByRole("button", { name: "Poke AYLUL" })).toBeVisible();
  await page.getByRole("button", { name: "Mount toggle" }).click();
  const before = await page.evaluate(() => (window as any).draws);
  await page.clock.install();
  await page.clock.runFor(1000);
  expect(await page.evaluate(() => (window as any).draws)).toBe(before);
  await page.getByRole("button", { name: "Mount toggle" }).click();
  await expect(page.getByRole("button", { name: "Poke AYLUL" })).toBeVisible();
});
for (const reduced of [false, true])
  test(
    reduced
      ? "reduced motion updates stay still"
      : "paused changes repaint without animation",
    async ({ page }) => {
      if (reduced) await page.emulateMedia({ reducedMotion: "reduce" });
      await expect(
        page.getByRole("button", { name: "Poke AYLUL" }),
      ).toBeVisible();
      if (!reduced)
        await page.getByRole("button", { name: "Pause motion" }).click();
      await page.clock.install();
      await page.clock.runFor(32);
      const before = await page
        .locator("canvas")
        .evaluate((c: HTMLCanvasElement) => c.toDataURL());
      await page.getByRole("button", { name: "Change expression" }).click();
      await page.clock.runFor(32);
      await expect(
        page.getByRole("status", { name: "AYLUL status" }),
      ).toHaveText("blocked");
      expect(
        await page
          .locator("canvas")
          .evaluate((c: HTMLCanvasElement) => c.toDataURL()),
      ).not.toBe(before);
      const count = await page.evaluate(() => (window as any).draws);
      await page.clock.runFor(1000);
      expect(await page.evaluate(() => (window as any).draws)).toBe(count);
    },
  );
test("zero size then DPR resize", async ({ page }) => {
  await page.getByRole("button", { name: "Zero size" }).click();
  await page.getByRole("button", { name: "Zero size" }).click();
  await expect(page.locator("canvas")).toBeVisible();
  expect(
    await page
      .locator("canvas")
      .evaluate((c) => c.getBoundingClientRect().width),
  ).toBeLessThanOrEqual(300);
});
test("same text new message expires independently", async ({ page }) => {
  await expect(page.getByRole("button", { name: "Poke AYLUL" })).toBeVisible();
  await page.clock.install();
  await page.getByRole("button", { name: "New message" }).click();
  await page.clock.runFor(3000);
  await page.getByRole("button", { name: "Change expression" }).click();
  await page.clock.runFor(1001);
  await expect(page.getByRole("status", { name: "AYLUL message" })).toHaveCount(
    0,
  );
  await page.getByRole("button", { name: "New message" }).click();
  await page.clock.runFor(3000);
  await expect(page.getByRole("status", { name: "AYLUL message" })).toHaveText(
    "Owner message",
  );
});
test("poke preserves controlled message", async ({ page }) => {
  await page.getByRole("button", { name: "New message" }).click();
  await page.getByRole("button", { name: "Poke AYLUL" }).press("Space");
  await expect(page.getByRole("status", { name: "AYLUL message" })).toHaveText(
    "Owner message",
  );
  await expect(page.getByRole("status", { name: "AYLUL status" })).toHaveText(
    "idle",
  );
});
test("plain mixed-direction text wraps", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await page.getByRole("button", { name: "Mixed text" }).click();
  const bubble = page.getByRole("status", { name: "AYLUL message" });
  await expect(bubble).toContainText("<img onerror=alert(1)>");
  await expect(bubble.locator("img")).toHaveCount(0);
  expect(await bubble.evaluate((e) => e.scrollWidth <= e.clientWidth)).toBe(
    true,
  );
});
test("canvas and audio failures stay local", async ({ page }) => {
  await page.addInitScript(() => {
    HTMLCanvasElement.prototype.getContext = (() => null) as any;
  });
  await page.reload();
  await expect(page.getByRole("alert")).toContainText("unavailable");
  await expect(
    page.getByRole("button", { name: "Mount toggle" }),
  ).toBeEnabled();
});
test("audio refusal leaves the companion usable and sound off", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "AudioContext", {
      value: class {
        constructor() {
          throw new Error("no device");
        }
      },
    });
  });
  await page.reload();
  await page.getByRole("button", { name: "Enable sound" }).click();
  await expect(
    page.getByText("Sound is unavailable.", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Poke AYLUL" })).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Enable sound" }),
  ).toBeVisible();
});
test("hidden document cancels frames until restored", async ({ page }) => {
  await expect(page.getByRole("button", { name: "Poke AYLUL" })).toBeVisible();
  await page.clock.install();
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "hidden",
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  const before = await page.evaluate(() => (window as any).draws);
  await page.clock.runFor(1000);
  expect(await page.evaluate(() => (window as any).draws)).toBe(before);
  await page.getByRole("button", { name: "Change expression" }).click();
  await page.clock.runFor(100);
  expect(await page.evaluate(() => (window as any).draws)).toBe(before);
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await page.clock.runFor(32);
  expect(await page.evaluate(() => (window as any).draws)).toBeGreaterThan(
    before,
  );
});
test("live DPR change resizes backing pixels while paused", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Pause motion" }).click();
  const session = await page.context().newCDPSession(page);
  await session.send("Emulation.setDeviceMetricsOverride", {
    width: 400,
    height: 700,
    deviceScaleFactor: 2,
    mobile: false,
  });
  await expect
    .poll(() =>
      page
        .locator("canvas")
        .evaluate(
          (c: HTMLCanvasElement) => c.width / c.getBoundingClientRect().width,
        ),
    )
    .toBe(2);
  await session.send("Emulation.clearDeviceMetricsOverride");
  await session.detach();
});
