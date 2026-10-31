import { test as base, expect, type Page, type Browser } from "@playwright/test";

/**
 * When PLAYWRIGHT_TAURI=1 (set by tauri-setup.ts), the `page` fixture
 * launches Chromium and connects it to the Tauri dev server.
 */
const IS_TAURI = process.env.PLAYWRIGHT_TAURI === "1";

export const test = IS_TAURI
  ? base.extend<{ page: Page; browser: Browser }>({
      browser: async ({ playwright }, use) => {
        console.log("[fixtures] Launching Chromium browser...");
        const browser = await playwright.chromium.launch({
          args: [
            "--disable-blink-features=AutomationControlled",
            "--disable-dev-shm-usage", // For Linux CI environments
          ],
        });
        console.log("[fixtures] ✓ Chromium launched");
        await use(browser);
        await browser.close();
      },

      page: async ({ browser }, use) => {
        console.log("[fixtures] Creating browser context...");
        const context = await browser.newContext();
        const page = await context.newPage();

        console.log("[fixtures] Navigating to base URL...");
        try {
          await page.goto("/", { waitUntil: "domcontentloaded", timeout: 60000 });
          console.log("[fixtures] ✓ Navigation complete");
        } catch (err) {
          console.error(
            `[fixtures] Navigation failed: ${err instanceof Error ? err.message : err}`,
          );
          // Take screenshot for debugging
          await page.screenshot({ path: "debug-navigation-failed.png" });
          throw err;
        }

        // Wait for React app to mount
        console.log("[fixtures] Waiting for app to mount (#root)...");
        try {
          const rootElement = page.locator("#root");
          await rootElement.waitFor({ state: "visible", timeout: 3000 });
          await page.waitForFunction(() => document.getElementById("root")?.children.length! > 0, {
            timeout: 3000,
          });
          console.log("[fixtures] ✓ App mounted successfully");
        } catch (err) {
          console.error(`[fixtures] App mount failed: ${err instanceof Error ? err.message : err}`);
          await page.screenshot({ path: "debug-app-mount-failed.png" });
          throw err;
        }

        await use(page);
        await context.close();
      },
    })
  : base;

export { expect };
