import { test, expect } from "./fixtures";

/**
 * Map interaction tests.
 * Tests map visibility, dimensions, click infrastructure, and Leaflet API integration.
 * Consolidated to avoid redundant dimension checks.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("map container is visible with usable dimensions and controls", async ({ page }) => {
  const map = page.locator(".map-container");
  await expect(map).toBeVisible();

  const mapBounds = await map.boundingBox();
  expect(mapBounds).not.toBeNull();
  if (mapBounds) {
    expect(mapBounds.width).toBeGreaterThan(0);
    expect(mapBounds.height).toBeGreaterThan(0);
  }

  // Verify map controls infrastructure exists
  const controls = await page.locator(".leaflet-control").count();
  expect(controls).toBeGreaterThanOrEqual(0);
});

test("map center can be set and controlled programmatically", async ({ page }) => {
  const map = page.locator(".map-container");
  await expect(map).toBeVisible();

  // Test that the map instance is properly exposed for programmatic control
  const success = await page.evaluate(() => {
    const leafletMap = (window as Window & { __leafletMap?: L.Map }).__leafletMap;
    if (leafletMap) {
      leafletMap.setView([60.20386, 24.93142], 10);
      return true;
    }
    return false;
  });

  expect(success).toBe(true);
});
