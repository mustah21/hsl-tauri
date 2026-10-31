import { test, expect } from "./fixtures";

/**
 * Input field tests for address search.
 * Tests both from/to inputs with combined test cases to reduce redundancy.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("address input fields are visible, enabled, and have correct labels", async ({ page }) => {
  const fromInput = page.getByRole("textbox", { name: /from location/i });
  const toInput = page.getByRole("textbox", { name: /to location/i });

  // Both inputs should be visible and enabled
  await expect(fromInput).toBeVisible();
  await expect(fromInput).toBeEnabled();
  await expect(fromInput).toHaveAttribute("aria-label", "From location address");
  await expect(fromInput).toHaveAttribute("placeholder", /enter address|click map/i);

  await expect(toInput).toBeVisible();
  await expect(toInput).toBeEnabled();
  await expect(toInput).toHaveAttribute("aria-label", "To location address");
  await expect(toInput).toHaveAttribute("placeholder", /enter address|click map/i);
});

test("type, clear, and focus transitions work on both address fields", async ({ page }) => {
  const fromInput = page.getByRole("textbox", { name: /from location/i });
  const toInput = page.getByRole("textbox", { name: /to location/i });

  // Test from input
  await fromInput.fill("Starting Point");
  await expect(fromInput).toHaveValue("Starting Point");
  await fromInput.click();
  await expect(fromInput).toBeFocused();
  await page.keyboard.press("Control+A");
  await page.keyboard.press("Delete");
  await expect(fromInput).toHaveValue("");

  // Test to input
  await toInput.fill("Destination");
  await expect(toInput).toHaveValue("Destination");
  await toInput.click();
  await expect(toInput).toBeFocused();
  await page.keyboard.press("Control+A");
  await page.keyboard.press("Delete");
  await expect(toInput).toHaveValue("");
});

test("address inputs accept both from and to without conflicts", async ({ page }) => {
  const fromInput = page.getByRole("textbox", { name: /from location/i });
  const toInput = page.getByRole("textbox", { name: /to location/i });

  // Fill both fields
  await fromInput.fill("Central Station");
  await toInput.fill("Airport");

  // Verify both values persist and don't interfere
  await expect(fromInput).toHaveValue("Central Station");
  await expect(toInput).toHaveValue("Airport");
  await expect(fromInput).not.toHaveValue("Airport");
  await expect(toInput).not.toHaveValue("Central Station");

  // Verify focus transitions between fields
  await fromInput.click();
  await expect(fromInput).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(fromInput).not.toBeFocused();
  await expect(toInput).toBeVisible();
});

test("enter known Helsinki addresses and capture map screenshot", async ({ page }) => {
  const fromInput = page.getByRole("textbox", { name: /from location/i });
  const toInput = page.getByRole("textbox", { name: /to location/i });
  const departureInput = page.getByLabel("Departure or arrival time");
  const map = page.locator(".map-container");

  // Set fixed departure date for deterministic screenshot
  await departureInput.fill("2025-12-06T14:23");

  // Enter known Helsinki addresses
  await fromInput.fill("Helsinginkatu 9, 00500 HELSINKI");
  await expect(fromInput).toHaveValue("Helsinginkatu 9, 00500 HELSINKI");

  await toInput.fill("Malminkatu 16, 00100 HELSINKI");
  await expect(toInput).toHaveValue("Malminkatu 16, 00100 HELSINKI");

  // Navigate to Settings tab to select routes
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("#panel-settings")).toBeVisible();

  // Get all checkboxes with aria-label matching "Show all ... lines on map"
  const allLinesCheckboxes = await page
    .locator('input[type="checkbox"].mode-show-all-checkbox')
    .all();
  expect(allLinesCheckboxes.length).toBe(9);

  // Ensure each checkbox is checked to show all route types on the map
  for (const checkbox of allLinesCheckboxes) {
    await expect(checkbox).toBeVisible();

    if (!(await checkbox.isChecked())) {
      await checkbox.check();
    }
    await expect(checkbox).toBeChecked();
  }

  // Wait for the map to be visible
  await expect(map).toBeVisible();

  // Brief settle for SVG paint
  await page.waitForTimeout(2000);

  // Take screenshot of the map only with markers and route
  await expect(map).toHaveScreenshot("map-helsinki-central-markers.png");
});
