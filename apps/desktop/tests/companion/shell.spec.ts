import { test, expect } from "@playwright/test";
const nav=(page:any,name:string)=>page.getByRole('navigation',{name:'Workspace'}).getByRole('button',{name,exact:true}).click();
test("production starts idle without synthetic operations", async ({
  page,
}) => {
  await page.goto("/tests/companion/fixture.html?shell");
  await expect(page.getByRole('button',{name:'Hide AYLUL',exact:true})).toBeVisible();
  await expect(page.locator('canvas')).toHaveCount(0);
  await nav(page,'Settings');
  await expect(
    page.getByRole("heading", { name: "Account", exact: true }),
  ).toBeVisible();
  await expect(page.getByTestId("fixture-ipc-calls")).not.toContainText(
    "working",
  );
});
test("companion failure preserves account and hide actions", async ({
  page,
}) => {
  await page.goto("/tests/companion/fixture.html?shell&companionFailed");
  await expect(page.getByRole("alert")).toContainText("unavailable");
  await expect(page.getByRole('button',{name:'Retry AYLUL',exact:true})).toBeVisible();
  await nav(page,'Settings');
  await page
    .getByRole("button", { name: "Check session", exact: true })
    .click();
  await page.getByRole("button", { name: "Hide to tray", exact: true }).click();
  await expect(page.getByTestId("fixture-ipc-calls")).toContainText(
    "hide_main",
  );
  await expect(
    page.getByRole("heading", { name: "Account", exact: true }),
  ).toBeVisible();
});
test("studio changes never change auth", async ({ page }) => {
  await page.goto("/tests/companion/fixture.html?shell");
  await nav(page,'Settings');
  await expect(page.getByText("Signed out", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Open development studio", exact: true })
    .click();
  const studio = page.getByRole("region", { name: "AYLUL development studio" });
  await studio.getByRole("button", { name: "Working", exact: true }).click();
  await expect(page.getByText("Signed out", { exact: true })).toBeVisible();
  await expect(page.getByRole('button',{name:'Hide AYLUL',exact:true})).toBeVisible();
});
test('main controls use actual native outcomes and rejected hide stays visible',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&hideRefused');
  await page.getByRole('button',{name:'Hide AYLUL',exact:true}).click();
  await expect(page.getByRole('alert')).toContainText('Unable');
  await expect(page.getByRole('button',{name:'Hide AYLUL',exact:true})).toBeVisible();
});
test('main toggles companion and pauses without changing account',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell');
  await page.getByRole('button',{name:'Hide AYLUL',exact:true}).click();
  await page.getByRole('button',{name:'Show AYLUL',exact:true}).click();
  await page.getByRole('button',{name:'Pause motion',exact:true}).click();
  await expect(page.getByRole('button',{name:'Resume motion',exact:true})).toBeVisible();
  await nav(page,'Settings');
  await expect(page.getByText('Signed out',{exact:true})).toBeVisible();
});

test('merged main keeps account companion and local hook requests together',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&hookSession');
  await nav(page,'Activity');
  await expect(page.getByRole('heading',{name:'Sessions',exact:true})).toBeVisible();
  const requests=page.getByRole('region',{name:'Requests',exact:true});
  await expect(requests.getByText('session-one',{exact:false})).toBeVisible();
  await expect(page.getByRole('button',{name:'Hide AYLUL',exact:true})).toBeVisible();
  await requests.getByRole('button',{name:'Allow once',exact:true}).click();
  await expect(requests.getByText('Nothing is waiting for you.',{exact:true})).toBeVisible();
  await expect(page.getByTestId('fixture-ipc-calls')).toContainText('request.answer');
  await nav(page,'Settings');
  await expect(page.getByText('Signed out',{exact:true})).toBeVisible();
});

test('unavailable local sidecar preserves account and companion controls',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&sidecarUnavailable');
  await nav(page,'Activity');
  await expect(page.getByText("AYLUL's local service is not running. Restart AYLUL.",{exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'Hide AYLUL',exact:true})).toBeVisible();
  await nav(page,'Settings');
  await page.getByRole('button',{name:'Check session',exact:true}).click();
  await expect(page.getByText('Signed out',{exact:true})).toBeVisible();
});
