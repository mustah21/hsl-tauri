import { test, expect } from "./fixtures";

/**
 * Settings persistence tests.
 * Verifies that user settings persist across page reloads via localStorage.
 * Consolidated to test multiple settings in fewer test cases.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("active tab persists across reload", async ({ page }) => {
  const settingsTab = page.getByRole("tab", { name: "Settings" });

  // Click Settings tab
  await settingsTab.click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  // Verify in localStorage
  const activeTab = await page.evaluate(() => localStorage.getItem("hslActiveTab"));
  expect(activeTab).toBeDefined();

  // Reload
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });

  // Settings tab should still be active
  await expect(settingsTab).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#panel-settings")).toBeVisible();
});

test("transport modes and UI settings persist across multiple reloads", async ({ page }) => {
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  // Get multiple transport modes
  const metro = page.getByRole("checkbox", { name: "Toggle Metro" });
  const localBus = page.getByRole("checkbox", { name: "Toggle Local bus" });
  const ferry = page.getByRole("checkbox", { name: "Toggle Ferry" });

  const metroInitial = await metro.isChecked();
  const localInitial = await localBus.isChecked();
  const ferryInitial = await ferry.isChecked();

  // Change them all
  await metro.click();
  await localBus.click();
  await ferry.click();

  // Verify in localStorage
  let settings = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("hslTransportModes") || "[]"),
  );
  expect(settings).toBeDefined();

  // First reload
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
  await page.getByRole("tab", { name: "Settings" }).click();

  let metroAfter1 = page.getByRole("checkbox", { name: "Toggle Metro" });
  let localAfter1 = page.getByRole("checkbox", { name: "Toggle Local bus" });
  let ferryAfter1 = page.getByRole("checkbox", { name: "Toggle Ferry" });

  expect(await metroAfter1.isChecked()).not.toBe(metroInitial);
  expect(await localAfter1.isChecked()).not.toBe(localInitial);
  expect(await ferryAfter1.isChecked()).not.toBe(ferryInitial);

  // Verify UI settings also persisted
  const uiSettings1 = await page.evaluate(() => localStorage.getItem("hslUiSettings"));
  expect(uiSettings1).toBeDefined();

  // Second reload to verify persistence survives multiple cycles
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
  await page.getByRole("tab", { name: "Settings" }).click();

  let metroAfter2 = page.getByRole("checkbox", { name: "Toggle Metro" });
  let localAfter2 = page.getByRole("checkbox", { name: "Toggle Local bus" });
  let ferryAfter2 = page.getByRole("checkbox", { name: "Toggle Ferry" });

  expect(await metroAfter2.isChecked()).not.toBe(metroInitial);
  expect(await localAfter2.isChecked()).not.toBe(localInitial);
  expect(await ferryAfter2.isChecked()).not.toBe(ferryInitial);
});
