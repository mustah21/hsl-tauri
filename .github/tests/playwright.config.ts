import { defineConfig } from "@playwright/test";

/**
 * Playwright config for the hsl-routes Tauri desktop app.
 *
 * Launches Chromium and connects it to the Tauri dev server (localhost:3000)
 * which is started by tauri-setup.ts via `npm run tauri dev`.
 *
 * Works on Windows, Linux, and macOS.
 *
 * Run from inside .github/tests/:
 *   npm run test
 */
export default defineConfig({
  testDir: "./usage-tests",

  /* Single Tauri window — no parallelism */
  fullyParallel: false,
  workers: 1,

  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [["html", { open: "never" }]], // Never auto-open HTML report

  /**
   * Global setup spawns `npm run tauri dev` and waits for dev server to be ready.
   * On first run, this may take 60+ seconds (Rust compilation).
   */
  //globalSetup: "./tauri-setup.ts",

  /* Allow plenty of time for cold-start (Vite + Rust compile can take 60+ seconds) */
  globalTimeout: 600_000, // 10 minutes

  /* Per-test timeout */
  timeout: 45_000, // 45 seconds per test

  use: {
    baseURL: "http://localhost:3000",
  },

  expect: {
    toHaveScreenshot: {
      /* Tolerate minor rendering differences across platforms */
      maxDiffPixelRatio: 0.05,
    },
  },

  projects: [
    {
      name: "tauri",
      use: {
        /* Matches the Tauri window size from tauri.conf.json for deterministic snapshots */
        viewport: { width: 1200, height: 800 },
      },
    },
  ],

  webServer: {
    name: "Tauri dev server",
    command: "npm run tauri dev",
    cwd: "../../", // Run from repo root where package.json is
    url: "http://localhost:3001/api/health", // Wait until axum API server is up (direct, no Vite proxy)
    stdout: "pipe",
    stderr: "pipe",
    env: {
      NODE_ENV: "development",
      PORT: "3000", // Ensure Vite dev server runs on port 3000
    },
    timeout: 600_000, // 10 minutes to start (Rust compilation can take a while)
  },
});
