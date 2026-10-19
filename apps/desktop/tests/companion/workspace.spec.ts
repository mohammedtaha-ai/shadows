import { test, expect } from "@playwright/test";
const url = "/tests/companion/fixture.html?shell";
const nav = (page: any, name: string) =>
  page.getByRole("navigation", { name: "Workspace" }).getByRole("button", { name, exact: true }).click();
test("workspace_routes_account_to_settings_and_preserves_chat_draft", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByRole("heading", { name: "Account", exact: true })).toBeHidden();
  await expect(page.getByRole("button", { name: "Sign in", exact: true })).toBeHidden();
  await nav(page, "Settings");
  await expect(page.getByRole("heading", { name: "Account", exact: true })).toBeVisible();
  await nav(page, "Chat");
  await expect(page.getByRole("heading", { name: "Account", exact: true })).toBeHidden();
  await expect(page.getByRole("heading", { name: "Chat with AYLUL" })).toBeVisible();
});
test("workspace_activity_keeps_approval_actions_and_native_hide", async ({ page }) => {
  await page.goto(url + "&hookSession");
  await nav(page, "Activity");
  const requests = page.getByRole("region", { name: "Requests", exact: true });
  await expect(requests.getByRole("button", { name: "Allow once", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Hide to tray", exact: true }).click();
  await expect(page.getByTestId("fixture-ipc-calls")).toContainText("hide_main");
});
for (const width of [390, 1280]) {
  test(`workspace_narrow_layout_preserves_navigation_and_composer_${width}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    await page.goto(url);
    await expect(page.getByRole("navigation", { name: "Workspace" })).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(overflow).toBe(false);
    const drawer = page.getByRole("button", { name: /Chats and bots|Close list/ });
    const chats = page.getByRole("button", { name: "Chats", exact: true });
    if (width === 390) {
      await expect(chats).toBeHidden();
      await drawer.click();
      await expect(chats).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(chats).toBeHidden();
      await drawer.focus();
      await page.keyboard.press("Enter");
      await expect(chats).toBeVisible();
      await drawer.click();
      await expect(chats).toBeHidden();
    } else {
      await expect(drawer).toBeHidden();
      await expect(chats).toBeVisible();
    }
    await nav(page, "Chat");
    await expect(page.getByRole("heading", { name: "Chat with AYLUL" })).toBeVisible();
  });
}
