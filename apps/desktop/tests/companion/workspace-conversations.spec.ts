import { test, expect, type Page } from "@playwright/test";

const url = "/tests/companion/fixture.html?shell&signedIn";
const ALPHA = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
const BETA = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2";
const calls = (page: Page) => page.evaluate(() => (window as any).__fx.calls as any[]);
const fx = (page: Page, run: string) => page.evaluate(`(()=>{const fx=window.__fx;${run}})()`);
const conversations = (page: Page) => page.getByRole("list", { name: "Conversations" });

test("workspace_selects_exact_conversation_and_creates_once", async ({ page }) => {
  await page.goto(url);
  await expect(conversations(page).getByRole("button", { name: /^Alpha plan/ })).toBeVisible();
  await expect(page.getByText("Alpha answer")).toBeVisible();
  await conversations(page).getByRole("button", { name: /^Beta notes/ }).click();
  await expect(page.getByText("Beta answer")).toBeVisible();
  const snaps = (await calls(page)).filter((c) => c.action === "snapshot").map((c) => c.conversationId);
  expect(snaps).toContain(BETA);
  await page.getByRole("button", { name: "New conversation", exact: true }).click();
  await expect(conversations(page).getByRole("button", { name: /^New conversation/ })).toBeVisible();
  expect((await calls(page)).filter((c) => c.action === "create")).toHaveLength(1);
});

test("workspace_ignores_previous_account_results", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByText("Alpha answer")).toBeVisible();
  await page.getByLabel("Message", { exact: true }).fill("Private draft");
  await fx(page, "fx.gate=new Promise(r=>{fx.release=r});");
  await page.waitForTimeout(1700);
  await fx(page, "fx.principal={accountId:'acct-b',sessionId:'sess-b',deviceId:'dev-1'};");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Check session", exact: true }).click();
  await fx(page, "fx.release();fx.gate=null;");
  await page.getByRole("button", { name: "Chats", exact: true }).click();
  await expect(page.getByText("Other answer")).toBeVisible();
  await expect(page.getByText("Alpha answer")).toHaveCount(0);
  await expect(page.getByLabel("Message", { exact: true })).toHaveValue("");
});

test("workspace_uncertain_send_keeps_command_without_automatic_repost", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByText("Connected", { exact: true })).toBeVisible();
  await fx(page, "fx.dropSendResponse=true;");
  const box = page.getByLabel("Message", { exact: true });
  await box.fill("Hello once");
  await page.getByRole("button", { name: "Send message" }).click();
  await expect(page.getByRole("button", { name: "Retry send" })).toBeVisible();
  const sends = () => calls(page).then((all) => all.filter((c) => c.action === "send"));
  expect(await sends()).toHaveLength(1);
  await page.getByRole("button", { name: "Retry send" }).click();
  await expect(page.getByText("Reply to Hello once")).toBeVisible();
  await page.waitForTimeout(3200);
  const all = await sends();
  expect(all).toHaveLength(2);
  expect(all[1].commandId).toBe(all[0].commandId);
});

test("workspace_pins_conversation_per_account", async ({ page }) => {
  await page.goto(url);
  const rows = conversations(page).getByRole("button", { name: /^(Alpha plan|Beta notes)/ });
  await expect(rows.first()).toContainText("Alpha plan");
  await page.getByRole("button", { name: "Pin Beta notes" }).click();
  await expect(rows.first()).toContainText("Beta notes");
  await page.reload();
  await expect(conversations(page).getByRole("button", { name: /^(Alpha plan|Beta notes)/ }).first()).toContainText("Beta notes");
  await fx(page, "fx.principal={accountId:'acct-b',sessionId:'sess-b',deviceId:'dev-1'};");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Check session", exact: true }).click();
  await expect(page.getByRole("button", { name: "Pin Other account chat" })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Unpin/ })).toHaveCount(0);
});

test("workspace_offline_keeps_saved_history_and_recovery", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByText("Alpha answer")).toBeVisible();
  await fx(page, "fx.offline=true;");
  const retry = page.getByRole("button", { name: "Retry connection" });
  await expect(retry).toBeVisible({ timeout: 6000 });
  await expect(page.getByText("Alpha answer")).toBeVisible();
  await expect(conversations(page).getByRole("button", { name: /^Alpha plan/ })).toBeVisible();
  await fx(page, "fx.offline=false;");
  await retry.click();
  await expect(retry).toBeHidden();
  await expect(page.getByText("Connected", { exact: true })).toBeVisible();
  expect((await calls(page)).filter((c) => c.action === "send")).toHaveLength(0);
});

test("workspace_unavailable_bot_configuration_never_dispatches", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByText("Alpha answer")).toBeVisible();
  await page.getByRole("button", { name: "Bots", exact: true }).click();
  const create = page.getByRole("button", { name: "Create bot", exact: true });
  await expect(create).toBeDisabled();
  await expect(create).toHaveAccessibleDescription(/Not available yet.*bot management/);
  const save = page.getByRole("button", { name: "Save changes", exact: true });
  await expect(save).toBeDisabled();
  await expect(page.getByLabel("Model", { exact: true })).toBeDisabled();
  const before = (await calls(page)).filter((c) => !["status", "snapshot"].includes(c.action));
  await create.click({ force: true }).catch(() => {});
  await save.click({ force: true }).catch(() => {});
  const after = (await calls(page)).filter((c) => !["status", "snapshot"].includes(c.action));
  expect(after).toEqual(before);
});

test("workspace_composer_never_invents_a_model_name", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByText("Model: managed by the account backend", { exact: true })).toBeVisible();
  const text = await page.locator(".assistant form").innerText();
  expect(text).not.toMatch(/MiniMax|Claude|GPT|Hermes|Anthropic|OpenAI/i);
});

test("workspace_settings_keep_account_and_character_independent", async ({ page }) => {
  await page.goto(url + "&companionFailed");
  await page.getByRole("navigation", { name: "Workspace" }).getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("button", { name: "Check session", exact: true })).toBeEnabled();
  await expect(page.getByRole("heading", { name: "Appearance" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Integrations" })).toBeVisible();
  await page.getByRole("navigation", { name: "Workspace" }).getByRole("button", { name: "Activity", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Activity", exact: true })).toBeVisible();
});

test("workspace_narrow_layout_collapses_inspector", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 800 });
  await page.goto(url);
  const details = page.getByRole("complementary", { name: "Bot details" });
  await expect(details).toBeHidden();
  await expect(page.getByLabel("Message", { exact: true })).toBeVisible();
  const open = page.getByRole("button", { name: "Bot settings", exact: true });
  await open.click();
  await expect(details).toBeVisible();
  await page.getByRole("button", { name: "Close bot details" }).click();
  await expect(details).toBeHidden();
  await expect(page.getByLabel("Message", { exact: true })).toBeVisible();
});

test("workspace_keyboard_reaches_navigation_list_and_composer", async ({ page }) => {
  await page.goto(url);
  await expect(page.getByText("Alpha answer")).toBeVisible();
  await page.getByRole("button", { name: "Chats", exact: true }).focus();
  const seen = new Set<string>();
  for (let n = 0; n < 40; n++) {
    await page.keyboard.press("Tab");
    seen.add(await page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? document.activeElement?.textContent?.trim().slice(0, 30) ?? ""));
  }
  for (const label of [/^Alpha plan/, /^Activity$/, /^Message$/, /^Close bot details$/]) {
    expect([...seen].some((value) => label.test(value))).toBe(true);
  }
});

test("workspace_mixed_text_uses_auto_direction", async ({ page }) => {
  await page.goto(url);
  await fx(page, "fx.data['acct-a'].turns['" + ALPHA + "'].push({id:'dddddddd-dddd-4ddd-8ddd-dddddddddddd',commandId:'dddddddd-dddd-4ddd-8ddd-ddddddddddd1',input:'مرحبا hello',output:'أهلا بك',state:'done',createdAt:'2026-10-19T10:00:00.000Z',updatedAt:'2026-10-19T10:00:00.000Z'});");
  const arabic = page.getByText("أهلا بك", { exact: true });
  await expect(arabic).toBeVisible({ timeout: 5000 });
  await expect(arabic).toHaveAttribute("dir", "auto");
  await expect(page.getByText("مرحبا hello", { exact: true })).toHaveAttribute("dir", "auto");
});
