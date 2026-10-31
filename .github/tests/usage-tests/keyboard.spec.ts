import { test, expect } from "./fixtures";

/**
 * Keyboard accessibility and usability tests.
 * Ensures keyboard navigation works as expected throughout the app.
 * Focused on interactions not covered in other tests.
 */

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.locator("#root:not(:empty)").waitFor({ timeout: 15_000 });
});

test("keyboard input field interactions and Enter/Escape handling", async ({ page }) => {
  const fromInput = page.locator("#from-input");

  // Test input focus and typing
  await fromInput.click();
  await expect(fromInput).toBeFocused();
  await fromInput.type("test address");
  await expect(fromInput).toHaveValue("test address");

  // Pressing Enter should not cause errors
  await page.keyboard.press("Enter");
  await expect(fromInput).toHaveValue("test address");

  // Pressing Escape should not cause errors or lose focus
  await page.keyboard.press("Escape");
  await expect(fromInput).toHaveValue("test address");

  // Test Tab navigation
  await page.keyboard.press("Tab");
  const focused = await page.evaluate(() => document.activeElement?.id);
  expect(focused).toBeDefined();
  await expect(fromInput).not.toBeFocused();
});
