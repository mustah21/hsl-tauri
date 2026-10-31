import { test, expect } from "./fixtures";

/**
 * Basic app initialization and rendering tests.
 * Ensures the app mounts correctly and core elements are present.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("window title and core elements render on startup", async ({ page }) => {
  // Check title
  await expect(page).toHaveTitle("HSL Route Planner");

  // Verify root is not empty
  await expect(page.locator("#root")).not.toBeEmpty();

  // Check that critical UI elements are present
  const fromInput = page.locator("#from-input");
  const toInput = page.locator("#to-input");
  const map = page.locator(".map-container");
  const tablist = page.getByRole("tablist", { name: "Route planner sections" });

  await expect(fromInput).toBeVisible();
  await expect(toInput).toBeVisible();
  await expect(map).toBeVisible();
  await expect(tablist).toBeVisible();
});

test("map container has usable dimensions", async ({ page }) => {
  const map = page.locator(".map-container");
  await expect(map).toBeVisible();

  const mapBounds = await map.boundingBox();
  expect(mapBounds).not.toBeNull();
  if (mapBounds) {
    expect(mapBounds.width).toBeGreaterThan(0);
    expect(mapBounds.height).toBeGreaterThan(0);
  }
});
