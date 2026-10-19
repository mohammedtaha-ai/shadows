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
