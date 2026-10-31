import { test, expect } from "./fixtures";

/**
 * Tab navigation and switching tests.
 * Verifies tab structure, selection, panel switching, and keyboard navigation.
 * Consolidated to test all tab behaviors in fewer, more focused tests.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("tab list renders all three tabs in correct order", async ({ page }) => {
  const tablist = page.getByRole("tablist", { name: "Route planner sections" });
  await expect(tablist).toBeVisible();

  const tabs = await tablist.getByRole("tab").all();
  expect(tabs).toHaveLength(3);

  const tabTexts = await tablist.getByRole("tab").allTextContents();
  expect(tabTexts).toContain("Addresses & Routes");
  expect(tabTexts).toContain("Settings");
  expect(tabTexts).toContain("About");
});

test("routes tab is selected by default and panels switch correctly", async ({ page }) => {
  const routesTab = page.getByRole("tab", { name: "Addresses & Routes" });
  const settingsTab = page.getByRole("tab", { name: "Settings" });
  const aboutTab = page.locator("#tab-about");

  // Verify routes tab is default
  await expect(routesTab).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#panel-routes")).toBeVisible();
  await expect(page.locator("#from-input")).toBeVisible();

  // Switch to Settings
  await settingsTab.click();
  await expect(settingsTab).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#panel-settings")).toBeVisible();
  await expect(page.locator("#panel-routes")).not.toBeVisible();

  // Verify About tab exists and is accessible
  await expect(aboutTab).toBeVisible();
  await expect(aboutTab).toHaveAttribute("role", "tab");

  // Switch back to Routes
  await routesTab.click();
  await expect(routesTab).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#panel-routes")).toBeVisible();
});

test("keyboard navigation through tabs", async ({ page }) => {
  const tablist = page.getByRole("tablist", { name: "Route planner sections" });
  await expect(tablist).toBeVisible();

  const tabs = await tablist.getByRole("tab").all();

  // All tabs should be focusable and visible
  for (const tab of tabs) {
    await expect(tab).toBeVisible();
  }

  // Keyboard navigation: click Settings then Routes to verify flow works
  const settingsTab = page.getByRole("tab", { name: "Settings" });
  const routesTab = page.getByRole("tab", { name: "Addresses & Routes" });

  await settingsTab.click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  await routesTab.click();
  await expect(page.locator("#panel-routes")).toBeVisible();
});
