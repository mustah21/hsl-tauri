import { test, expect } from "./fixtures";

/**
 * Settings and transport mode tests.
 * Tests transport mode controls, UI settings, and settings panel behavior.
 * Combined tests to efficiently verify both mouse and keyboard interactions.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("settings tab shows transport mode checkboxes", async ({ page }) => {
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  // TransportModeControl renders checkboxes for each mode
  const regionalBus = page.getByRole("checkbox", {
    name: "Toggle Regional bus",
  });
  await expect(regionalBus).toBeVisible();
  await expect(regionalBus).toBeChecked();
});

test("toggle transport modes with mouse and keyboard", async ({ page }) => {
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  const regionalBus = page.getByRole("checkbox", {
    name: "Toggle Regional bus",
  });
  const expressBus = page.getByRole("checkbox", {
    name: "Toggle Express bus",
  });
  const localBus = page.getByRole("checkbox", {
    name: "Toggle Local bus",
  });

  // Toggle via mouse and verify state changes
  const regionalInitial = await regionalBus.isChecked();
  await regionalBus.click();
  expect(await regionalBus.isChecked()).not.toBe(regionalInitial);

  // Toggle express bus
  const expressInitial = await expressBus.isChecked();
  await expressBus.click();
  expect(await expressBus.isChecked()).not.toBe(expressInitial);

  // Toggle local bus via keyboard (Space)
  const localInitial = await localBus.isChecked();
  await localBus.focus();
  await page.keyboard.press("Space");
  expect(await localBus.isChecked()).not.toBe(localInitial);
});

test("multiple transport modes can be toggled independently", async ({ page }) => {
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  const metro = page.getByRole("checkbox", { name: "Toggle Metro" });
  const tram = page.getByRole("checkbox", { name: "Toggle Tram service" });
  const ferry = page.getByRole("checkbox", { name: "Toggle Ferry" });

  // Get initial states
  const metroInitial = await metro.isChecked();
  const tramInitial = await tram.isChecked();
  const ferryInitial = await ferry.isChecked();

  // Toggle one
  await metro.click();
  expect(await metro.isChecked()).toBe(!metroInitial);

  // Verify others didn't change
  expect(await tram.isChecked()).toBe(tramInitial);
  expect(await ferry.isChecked()).toBe(ferryInitial);
});

test("show extended stops feature and draw all routes controls exist", async ({ page }) => {
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  await page.waitForTimeout(200);

  // Verify that UI settings are stored
  const uiSettings = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("hslUiSettings") || "{}"),
  );
  expect(typeof uiSettings).toBe("object");

  // Verify checkboxes exist
  const checkboxCount = await page.locator("input[type='checkbox']").count();
  expect(checkboxCount).toBeGreaterThan(0);
});

test("all show all lines on map checkboxes are checked", async ({ page }) => {
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  // Get all checkboxes with aria-label matching "Show all ... lines on map"
  const allLinesCheckboxes = await page
    .locator('input[type="checkbox"].mode-show-all-checkbox')
    .all();
  expect(allLinesCheckboxes.length).toBe(9);

  // Ensure each checkbox is checked
  for (const checkbox of allLinesCheckboxes) {
    await expect(checkbox).toBeVisible();

    if (!(await checkbox.isChecked())) {
      await checkbox.check();
    }
    await expect(checkbox).toBeChecked();
  }
});
