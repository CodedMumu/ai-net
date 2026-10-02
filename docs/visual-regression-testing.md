# Frontend Visual Regression Testing

Reference guide for the Playwright-based visual regression suite that guards the frontend's key UI surfaces against unintentional CSS and layout changes.

---

## Table of Contents

1. [What Visual Regression Testing Covers](#1-what-visual-regression-testing-covers)
2. [Tools Used](#2-tools-used)
3. [Running Visual Tests Locally](#3-running-visual-tests-locally)
4. [Updating Baselines](#4-updating-baselines)
5. [CI Integration](#5-ci-integration)
6. [Diff Threshold Configuration](#6-diff-threshold-configuration)
7. [Adding Tests for New UI Surfaces](#7-adding-tests-for-new-ui-surfaces)

---

## 1. What Visual Regression Testing Covers

The visual suite lives at `frontend/tests/visual/` and is driven by `frontend/playwright.visual.config.ts`, which is kept separate from the functional e2e suite (`frontend/playwright.config.ts`). Screenshot baselines are tied to a single browser engine and OS rendering stack.

### 1.1 Covered Surfaces

Every surface is captured in both **light** and **dark** themes, producing 8 baseline images total.

| Surface | Route | Auth State | Test Description |
|---|---|---|---|
| Landing page | `/` | Unauthenticated | Full-page hero, feature cards, and navigation bar |
| Wallet page | `/wallet` | Unauthenticated | Secret key input form (wallet connect UI) |
| Dashboard | `/dashboard` | Wallet connected (mocked) | KPI cards, recent task table, stats bar |
| Task detail | `/tasks/:id` | Wallet connected (mocked), mock WS | DAG graph, WebSocket connection chip, task metadata |

### 1.2 Baseline File Locations

Baseline PNG files are committed to the repository under:

```
frontend/tests/visual/pages.visual.spec.ts-snapshots/
├── landing-light-visual-chromium-linux.png
├── landing-dark-visual-chromium-linux.png
├── wallet-light-visual-chromium-linux.png
├── wallet-dark-visual-chromium-linux.png
├── dashboard-light-visual-chromium-linux.png
├── dashboard-dark-visual-chromium-linux.png
├── task-detail-light-visual-chromium-linux.png
└── task-detail-dark-visual-chromium-linux.png
```

### 1.3 What Is Not Covered

- **Component-level snapshots** — the visual suite captures full page screenshots, not isolated component renders. Component-level testing is handled by Vitest unit tests.
- **Firefox and WebKit** — only Chromium is used (see [Section 2](#2-tools-used) for rationale).
- **Mobile viewports** — the suite runs at `1440×900` (Desktop Chrome). Mobile visual regression is tracked in a separate backlog item.
- **Animations in progress** — all CSS transitions and JS-driven animations are disabled or waited out before capture (see [Section 1.4](#14-stabilizing-screenshots)).

### 1.4 Stabilizing Screenshots

Two sources of nondeterminism are handled explicitly so snapshots are taken after the UI has fully settled:

**CSS transitions and animations** — disabled globally via `animations: 'disabled'` in `playwright.visual.config.ts`. This is set at the Playwright `expect.toHaveScreenshot` level and applies to every snapshot call in the suite.

**JavaScript-driven motion** (framer-motion components that don't check `prefers-reduced-motion`) — the landing page hero fade-in, the stats bar entrance animation, and the dashboard KPI card spring-animated counters. The `preparePage()` helper in `tests/visual/utils.ts` sets `prefers-reduced-motion: reduce` in the browser context before every test, and `waitForMotionToSettle()` adds a fixed post-`networkidle` wait so screenshots capture the settled state.

**Mock data** — the dashboard requires deterministic fixture data. Two MSW handlers registered in `frontend/src/mocks/handlers.ts` (`GET /api/stats`, `GET /api/wallets/:address/tasks`) serve fixed fixture responses so the KPI numbers and task list are stable across runs.

**WebSocket connection** — the task-detail test starts a local WebSocket server on port 3001 (via `startMockTaskSocket()` in `tests/visual/mockTaskSocket.ts`) so the connection-status chip renders "connected" instead of cycling through reconnect states.

**Wallet auth** — the dashboard and task-detail tests seed `localStorage` (`wallet_pubkey`, `walletAddress`, `wallet_connection_method`) before the app boots so `ProtectedRoute` renders the real page.

---

## 2. Tools Used

### Playwright

The suite uses [Playwright](https://playwright.dev) (`@playwright/test`) for browser automation and the built-in `toHaveScreenshot` assertion for pixel-level comparison. Playwright's screenshot comparison generates a diff image on failure, visible in the HTML report.

**Why not Percy or Chromatic?** Percy and Chromatic are cloud-based services that require network access and external accounts. Running visual comparisons locally and in self-hosted CI avoids external dependencies, keeps baselines in version control, and makes the diff visible in the PR artifact without leaving GitHub.

### Config file

`frontend/playwright.visual.config.ts` is the entry point for all visual tests:

```typescript
export default defineConfig({
  testDir: './tests/visual',
  fullyParallel: false,     // screenshots must be stable; no parallel capture
  retries: process.env.CI ? 1 : 0,
  workers: 1,               // single worker to prevent viewport contention
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.02,  // ≤2% pixel difference allowed
      animations: 'disabled',
    },
  },
  use: {
    baseURL: 'http://localhost:3000',
    viewport: { width: 1440, height: 900 },
  },
  projects: [
    {
      name: 'visual-chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
```

### Why Chromium Only?

Screenshot baselines are pixel-tied to a single browser rendering engine, OS font stack, and sub-pixel anti-aliasing. Mixing Chromium and Firefox baselines would double the number of PNGs to maintain with no additional regression coverage — a CSS layout bug shows up the same way in both browsers. Firefox and WebKit coverage is provided by the separate functional e2e suite (`playwright.config.ts`).

---

## 3. Running Visual Tests Locally

### Prerequisites

- Node.js 20+
- Playwright browsers installed: `npx playwright install chromium`
- Vite dev server **not** already running on `:3001` (the mock WebSocket server uses that port)

### Run all visual tests

```bash
cd frontend
npm run test:visual
```

This command:
1. Starts the Vite dev server on `:3000` (reuses one already running)
2. Runs every spec in `tests/visual/` against it
3. Compares screenshots against baselines in `tests/visual/pages.visual.spec.ts-snapshots/`
4. Produces an HTML report at `frontend/playwright-report-visual/index.html`

### Expected output (all passing)

```
Running 8 tests using 1 worker

  ✓ Visual regression — landing page › landing page renders correctly in light theme (3.2s)
  ✓ Visual regression — landing page › landing page renders correctly in dark theme (2.9s)
  ✓ Visual regression — wallet page › wallet connect page renders correctly in light theme (2.1s)
  ✓ Visual regression — wallet page › wallet connect page renders correctly in dark theme (2.0s)
  ✓ Visual regression — dashboard › dashboard renders correctly in light theme (4.1s)
  ✓ Visual regression — dashboard › dashboard renders correctly in dark theme (3.8s)
  ✓ Visual regression — task detail › task detail renders correctly in light theme (5.3s)
  ✓ Visual regression — task detail › task detail renders correctly in dark theme (5.1s)

  8 passed (31s)
```

### Expected output (with a regression)

```
  ✗ Visual regression — dashboard › dashboard renders correctly in light theme (3.9s)

    1 snapshot comparison(s) failed.

    Error: Screenshot comparison failed:
      Expected: tests/visual/pages.visual.spec.ts-snapshots/dashboard-light-visual-chromium-linux.png
      Received: playwright-report-visual/.../dashboard-light-visual-chromium-linux-actual.png
      Diff:     playwright-report-visual/.../dashboard-light-visual-chromium-linux-diff.png

      maxDiffPixelRatio exceeded: 0.034 > 0.02

  Open the HTML report: npx playwright show-report playwright-report-visual
```

### Viewing the HTML report

```bash
cd frontend
npx playwright show-report playwright-report-visual
```

The report shows expected, actual, and diff images side by side for every snapshot, including passing ones.

---

## 4. Updating Baselines

### When to update

Update baselines whenever an **intentional** UI change makes existing screenshots stale — for example, after changing a component's layout, colours, typography, or adding new UI elements to a covered surface.

Do **not** update baselines to hide a real regression. If a diff appears on a surface you did not intentionally change, investigate the cause before updating.

### Step-by-step baseline update process

**Step 1: Confirm the UI change is intentional**

Review the diff image in the HTML report. Verify that every changed pixel corresponds to the change you made.

**Step 2: Regenerate baselines**

Baselines must be generated on Linux with the same Playwright browser build that CI uses. If you are on Linux, run:

```bash
cd frontend
npm run test:visual:update
```

If you are on macOS or Windows, generate baselines through the pinned Playwright Docker image instead (the same image the CI job uses):

```bash
# From the repo root
docker run --rm \
  -v "$(pwd)/..:/work" \
  -w /work/frontend \
  -e TZ=UTC \
  mcr.microsoft.com/playwright:v1.61.0-jammy \
  bash -c "npm ci && npm run test:visual:update"
```

> **Important:** Keep the image tag (`v1.61.0-jammy`) in sync with the `@playwright/test` version in `frontend/package-lock.json`, and with the tag pinned in the `frontend-visual-regression` job in `.github/workflows/ci.yml`. A mismatch reintroduces the rendering drift this setup is designed to prevent.

**Step 3: Review the updated PNG files**

```bash
cd frontend
git diff --stat tests/visual/pages.visual.spec.ts-snapshots/
```

Confirm only the surfaces you changed have updated baselines. Binary PNG diffs are expected since images change.

**Step 4: Run the tests again to confirm green**

```bash
npm run test:visual
# All 8 tests should pass
```

**Step 5: Commit the updated baselines**

```bash
git add tests/visual/pages.visual.spec.ts-snapshots/
git commit -m "test(visual): update baselines for <surface> changes"
```

Include a brief note in the PR description explaining which surfaces changed and why. Reviewers will inspect the PNG diffs in the PR.

---

## 5. CI Integration

### Job: `frontend-visual-regression`

The visual tests run in the `frontend-visual-regression` job defined in `.github/workflows/ci.yml`.

The job:
1. Checks out the repository
2. Sets up Node.js 20
3. Runs `npm ci` in `frontend/`
4. Runs inside the pinned `mcr.microsoft.com/playwright:v1.61.0-jammy` Docker image so its rendering exactly matches the baseline generation environment
5. Executes `npm run test:visual`
6. Uploads `frontend/playwright-report-visual/` as the **`playwright-visual-report`** artifact on every run (pass **or** fail)
7. On failure, writes a step summary pointing to the artifact

### When tests run

The `frontend-visual-regression` job runs on every push to `main` and on every pull request.

### Viewing failures in a PR

1. Open the PR's **Checks** tab.
2. Find the `frontend-visual-regression` job.
3. If failed, scroll to the step summary — it contains a direct link to the artifact.
4. Download the `playwright-visual-report` artifact.
5. Open `index.html` — it shows the expected, actual, and diff images for every snapshot.

### `continue-on-error: true`

The `frontend-visual-regression` job is configured with `continue-on-error: true`. A visual diff is reported and visible on the PR's checks list and via the artifact, but it does not block merging on its own. Treat a red run as a prompt to open the report and confirm whether the diff represents an intentional UI change (update baselines) or a true regression (fix the code).

### Screenshot artefact retention

Playwright report artifacts are retained for 30 days by default (GitHub's artifact retention policy). Adjust in `.github/workflows/ci.yml` under the `upload-artifact` step's `retention-days` field if needed.

---

## 6. Diff Threshold Configuration

### Current threshold

`maxDiffPixelRatio: 0.02` — up to **2%** of pixels in a snapshot may differ before the test fails.

This provides headroom for:
- Sub-pixel anti-aliasing variation between Playwright minor versions
- Font rendering differences (emoji fallback glyphs, kerning)
- Small timing-dependent rendering artefacts

It is tight enough to catch real layout shifts, reflow bugs, and unintended colour or spacing changes.

### Where to change it

The threshold is set globally in `frontend/playwright.visual.config.ts`:

```typescript
expect: {
  toHaveScreenshot: {
    maxDiffPixelRatio: 0.02,  // change this value
    animations: 'disabled',
  },
},
```

You can also override it per-test for a specific snapshot that is known to have higher variance:

```typescript
await expect(page).toHaveScreenshot('landing-light.png', {
  maxDiffPixelRatio: 0.05,  // looser threshold for this snapshot only
  fullPage: true,
});
```

### Tightening vs. loosening

- **Tighten** (lower value, e.g. `0.01`) when you want stricter enforcement — useful after stabilizing a historically flaky surface.
- **Loosen** (higher value, e.g. `0.05`) when a specific surface has legitimate high-frequency rendering noise (e.g., a canvas element or a chart with floating-point rounding artefacts). Prefer per-test overrides over changing the global value.

### Pixel-count threshold (alternative)

If pixel ratio is too coarse for a specific snapshot, use `maxDiffPixels` instead (absolute pixel count):

```typescript
await expect(page).toHaveScreenshot('wallet-light.png', {
  maxDiffPixels: 100,  // at most 100 pixels may differ
  fullPage: true,
});
```

---

## 7. Adding Tests for New UI Surfaces

Follow these steps when adding a new page or major UI surface that requires visual regression coverage.

### Step 1: Add the test

Add a new `test.describe` block in `frontend/tests/visual/pages.visual.spec.ts`:

```typescript
test.describe('Visual regression — agents page', () => {
  for (const theme of THEMES) {
    test(`agents page renders correctly in ${theme} theme`, async ({ page }) => {
      // 1. Apply standard setup helpers
      await preparePage(page);
      await setTheme(page, theme);

      // 2. Mock wallet if the route requires authentication
      await mockWalletConnection(page);

      // 3. Navigate and wait for content
      await page.goto('/agents');
      await expect(page.locator('[data-testid="agents-list"]')).toBeVisible({
        timeout: INITIAL_LOAD_TIMEOUT,
      });
      await page.waitForLoadState('networkidle');
      await waitForMotionToSettle(page);

      // 4. Take the screenshot
      await expect(page).toHaveScreenshot(`agents-${theme}.png`, { fullPage: true });
    });
  }
});
```

### Step 2: Add MSW handlers for API data (if needed)

If the new surface fetches data from the backend API, add a mock handler to `frontend/src/mocks/handlers.ts` so the page renders deterministic fixture data:

```typescript
// frontend/src/mocks/handlers.ts
http.get('/api/agents', () => {
  return HttpResponse.json([
    {
      id: 'seed-research-agent',
      capabilities: ['research'],
      pricingXLM: 2.5,
      reputationScore: 4.8,
      status: 'online',
    },
    // ... more fixture agents
  ]);
}),
```

### Step 3: Seed test IDs on dynamic elements

Add `data-testid` attributes to loading skeletons and key content containers so the test can wait for the loaded state before snapshotting:

```tsx
// In your React component
<div data-testid="agents-list">
  {agents.map((agent) => (
    <AgentCard key={agent.id} agent={agent} />
  ))}
</div>

// Loading state
<div data-testid="agents-skeleton">
  {/* skeleton items */}
</div>
```

Wait for the skeleton to disappear before capturing:

```typescript
await expect(page.locator('[data-testid="agents-skeleton"]')).toHaveCount(0, {
  timeout: INITIAL_LOAD_TIMEOUT,
});
```

### Step 4: Generate initial baselines

There are no baselines yet for the new surface — the first run will fail with `missing snapshot`. Generate them:

```bash
cd frontend
npm run test:visual:update
# or via Docker for cross-platform consistency:
docker run --rm -v "$(pwd)/..:/work" -w /work/frontend \
  -e TZ=UTC \
  mcr.microsoft.com/playwright:v1.61.0-jammy \
  bash -c "npm ci && npm run test:visual:update"
```

### Step 5: Commit the baselines

```bash
git add tests/visual/pages.visual.spec.ts-snapshots/agents-light-visual-chromium-linux.png
git add tests/visual/pages.visual.spec.ts-snapshots/agents-dark-visual-chromium-linux.png
git commit -m "test(visual): add visual regression baselines for agents page"
```

### Step 6: Verify CI passes

Push the branch and confirm the `frontend-visual-regression` CI job passes with the new tests included.

---

## Helper Reference

The `tests/visual/utils.ts` module exports these helpers used across all visual tests:

| Helper | Purpose |
|---|---|
| `preparePage(page)` | Grants `prefers-reduced-motion: reduce` to suppress JS-driven animations |
| `setTheme(page, 'light' \| 'dark')` | Sets the `data-theme` attribute on `<html>` before navigation |
| `mockWalletConnection(page)` | Seeds `localStorage` with a fake wallet public key so `ProtectedRoute` renders |
| `skipWalletWizard(page)` | Dismisses the first-run wallet setup wizard |
| `waitForMotionToSettle(page)` | Fixed 300ms wait after `networkidle` for residual JS animations |
| `MOCK_TASK_ID` | Deterministic task ID used by the task-detail fixture handlers |

---

## Related Documents

- [Frontend Architecture & Conventions](FRONTEND_ARCHITECTURE.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [CI Workflow](.github/workflows/ci.yml) — `frontend-visual-regression` job
- [Playwright Docs](https://playwright.dev/docs/test-snapshots) — `toHaveScreenshot` API reference
