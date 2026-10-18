import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests/companion",
  testMatch: "**/*.spec.ts",
  timeout: 30000,
  workers: 1,
  use: {
    browserName: "chromium",
    channel: "chrome",
    baseURL: "http://127.0.0.1:4173",
  },
  webServer: {
    command: "npm run dev -- --port 4173 --strictPort",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
  },
});
