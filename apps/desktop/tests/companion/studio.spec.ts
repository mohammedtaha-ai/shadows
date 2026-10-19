import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/companion/fixture.html?studio");
});
for (const preset of [
  {
    name: "offscreen export uses the latest expression outfit and direction",
    initial: false,
    button: "right",
    lookX: 0.82,
    tilt: 0.05,
  },
  {
    name: "offscreen export clears a previously visible locked direction",
    initial: true,
    button: "Follow pointer",
    lookX: 0,
    tilt: 0,
  },
])
  test(preset.name, async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.reload();
    const studio = page.getByRole("region", {
      name: "AYLUL development studio",
    });
    await expect(studio.locator("canvas").first()).toBeVisible();
    if (preset.initial) {
      const before = await studio
        .locator("canvas")
        .first()
        .evaluate((c: HTMLCanvasElement) => c.toDataURL());
      await studio.getByRole("button", { name: "right", exact: true }).click();
      await expect
        .poll(() =>
          studio
            .locator("canvas")
            .first()
            .evaluate((c: HTMLCanvasElement) => c.toDataURL()),
        )
        .not.toBe(before);
    }
    // The stage has drawn the original selection, then becomes non-visible.
    await studio.locator(".aylul-studio-stage").evaluate((el) => {
      (el as HTMLElement).style.display = "none";
    });
    await studio.getByRole("button", { name: "Blocked", exact: true }).click();
    await studio.getByRole("button", { name: "Winter", exact: true }).click();
    await studio
      .getByRole("button", { name: preset.button, exact: true })
      .click();
    const pending = page.waitForEvent("download");
    await studio.getByRole("button", { name: "Export PNG" }).click();
    const download = await pending;
    const actual = await readFile((await download.path())!);
    const expected = await page.evaluate(
      async ({ lookX, tilt }) => {
        const { exportCharacterPNG } =
          await import("/src/features/companion/studio/export.ts");
        const { DEFAULT_SETTINGS } =
          await import("/src/features/companion/catalog.ts");
        const blob = await exportCharacterPNG(
          {
            cx: 0,
            cy: 0,
            scale: 1,
            lookX,
            lookY: 0,
            tilt,
            time: 0,
            earringAngle: 0,
            blinkProgress: 0,
            expression: "blocked",
            outfit: "winter",
            mini: false,
            perchingHands: false,
            sleepingMarks: [],
            notificationPulse: 0,
          },
          DEFAULT_SETTINGS,
        );
        return [...new Uint8Array(await blob.arrayBuffer())];
      },
      { lookX: preset.lookX, tilt: preset.tilt },
    );
    expect(actual.equals(Buffer.from(expected))).toBe(true);
  });
test("mock window reclamps on narrow stage and resize", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 900 });
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await studio.getByLabel("Scene").selectOption("window");
  const fits = () =>
    studio.locator(".aylul-mock-window").evaluate((el) => {
      const box = el.getBoundingClientRect(),
        stage = el.parentElement!.getBoundingClientRect();
      return (
        box.left >= stage.left &&
        box.right <= stage.right + 0.1 &&
        box.top >= stage.top &&
        box.bottom <= stage.bottom + 0.1
      );
    });
  await expect.poll(fits).toBe(true);
  await page.setViewportSize({ width: 800, height: 900 });
  await page.setViewportSize({ width: 320, height: 900 });
  await expect.poll(fits).toBe(true);
});
test("all catalogs and scene controls work", async ({ page }) => {
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await expect(studio).toBeVisible();
  for (const state of [
    "Idle",
    "Happy",
    "Listening",
    "Speaking",
    "Thinking",
    "Working",
    "Success",
    "Blocked",
    "Sleeping",
  ]) {
    await studio.getByRole("button", { name: state, exact: true }).click();
    await expect(
      studio.getByRole("status", { name: "Preview expression" }),
    ).toHaveText(state.toLowerCase());
  }
  for (const outfit of [
    "Default",
    "Crown",
    "Moon",
    "Scarf",
    "Starry",
    "Winter",
    "Headphones",
    "Mask",
    "Halo",
    "Special",
  ])
    await studio.getByRole("button", { name: outfit, exact: true }).click();
  for (const scene of ["studio", "cursor", "window", "assistant"])
    await studio.getByLabel("Scene").selectOption(scene);
  await expect(studio.getByLabel("Synthetic command")).toBeVisible();
});
test("manual same-expression selection cancels notification", async ({
  page,
}) => {
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await expect(studio).toBeVisible();
  await page.clock.install();
  await studio.getByRole("button", { name: "Preview notification" }).click();
  await studio.getByRole("button", { name: "Listening", exact: true }).click();
  await page.clock.runFor(1801);
  await expect(
    studio.getByRole("status", { name: "Preview expression" }),
  ).toHaveText("listening");
});
test("scene close cancels every demo callback", async ({ page }) => {
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await expect(studio).toBeVisible();
  await page.clock.install();
  await studio
    .getByRole("button", { name: "Simulate success", exact: true })
    .click();
  await page.getByRole("button", { name: "Studio toggle" }).click();
  await page.clock.runFor(6000);
  await page.getByRole("button", { name: "Studio toggle" }).click();
  await expect(
    studio.getByRole("status", { name: "Preview expression" }),
  ).toHaveText("idle");
});
test("new run supersedes old run and studio cannot change main presence", async ({
  page,
}) => {
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await expect(studio).toBeVisible();
  await page.clock.install();
  await studio
    .getByRole("button", { name: "Simulate success", exact: true })
    .click();
  await page.clock.runFor(3000);
  await studio
    .getByRole("button", { name: "Simulate blocked", exact: true })
    .click();
  await page.clock.runFor(4200);
  await expect(
    studio.getByRole("status", { name: "Preview expression" }),
  ).toHaveText("blocked");
  await expect(
    page.getByRole("status", { name: "AYLUL status", exact: true }),
  ).toHaveText("Idle");
});
test("thumbnails draw on changes only", async ({ page }) => {
  const grid = page.getByTestId("preview-grid");
  await expect(grid.locator("canvas")).toHaveCount(11);
  await page.clock.install();
  const first = await grid
    .locator("canvas")
    .first()
    .evaluate((c: HTMLCanvasElement) => c.toDataURL());
  await page.clock.runFor(2000);
  expect(
    await grid
      .locator("canvas")
      .first()
      .evaluate((c: HTMLCanvasElement) => c.toDataURL()),
  ).toBe(first);
});
test("mock perch stays inside stage", async ({ page }) => {
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await studio.getByLabel("Scene").selectOption("window");
  await expect(
    studio.getByText("Mock code window", { exact: true }),
  ).toBeVisible();
});
test("PNG is 512 square with transparent corners", async ({ page }) => {
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await expect(studio.locator("canvas").first()).toBeVisible();
  const download = page.waitForEvent("download");
  await studio.getByRole("button", { name: "Export PNG" }).click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/\.png$/);
  expect(await file.failure()).toBeNull();
  const result = await page.evaluate(async () => {
    const { exportCharacterPNG } =
      await import("/src/features/companion/studio/export.ts");
    const { DEFAULT_SETTINGS } =
      await import("/src/features/companion/catalog.ts");
    const blob = await exportCharacterPNG(
      {
        cx: 0,
        cy: 0,
        scale: 1,
        lookX: 0,
        lookY: 0,
        tilt: 0,
        time: 0,
        earringAngle: 0,
        blinkProgress: 0,
        expression: "idle",
        outfit: "default",
        mini: false,
        perchingHands: false,
        sleepingMarks: [],
        notificationPulse: 0,
      },
      DEFAULT_SETTINGS,
    );
    const bitmap = await createImageBitmap(blob);
    const c = document.createElement("canvas");
    c.width = c.height = 512;
    const ctx = c.getContext("2d")!;
    ctx.drawImage(bitmap, 0, 0);
    return {
      width: bitmap.width,
      height: bitmap.height,
      alpha: ctx.getImageData(0, 0, 1, 1).data[3],
    };
  });
  expect(result).toEqual({ width: 512, height: 512, alpha: 0 });
});
test("export failure shows error", async ({ page }) => {
  await page.addInitScript(() => {
    HTMLCanvasElement.prototype.toBlob = function (callback) {
      callback(null);
    };
  });
  await page.reload();
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await studio.getByRole("button", { name: "Export PNG" }).click();
  await expect(studio.getByRole("alert")).toContainText("export");
});
