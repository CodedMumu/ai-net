import { defineConfig, devices } from '@playwright/test';

/**
 * Visual-regression config, kept separate from `playwright.config.ts`
 * (functional e2e) because screenshot baselines are pixel-tied to a single
 * browser engine, viewport, and OS. Mixing them into the functional suite's
 * multi-browser matrix would double the number of baselines to maintain for
 * no correctness benefit — a CSS regression shows up the same way in
 * Chromium as it does in Firefox.
 *
 * Baselines are generated/verified with the `mcr.microsoft.com/playwright`
 * Docker image pinned to the `@playwright/test` version in package.json, so
 * local updates match what CI renders. See `tests/visual/README.md`.
 *
 * Three viewport projects are defined so every UI surface is captured at
 * mobile (375 px), tablet (768 px), and desktop (1440 px) widths.
 */
export default defineConfig({
  testDir: './tests/visual',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: [
    ['html', { outputFolder: 'playwright-report-visual', open: 'never' }],
    ['list'],
  ],
  expect: {
    // 0.1 % pixel-difference threshold — tight enough to catch real regressions
    // while absorbing font-rendering noise between identical Linux builds.
    // See docs/visual-regression-testing.md for the rationale.
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.001,
      animations: 'disabled',
    },
  },
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'visual-desktop',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1440, height: 900 },
      },
    },
    {
      name: 'visual-tablet',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 768, height: 1024 },
      },
    },
    {
      name: 'visual-mobile',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 375, height: 812 },
        isMobile: false,
      },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});
