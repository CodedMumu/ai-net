# Frontend Visual Regression Testing

This document describes the Playwright-based visual regression suite that guards the frontend's key UI surfaces against unintentional CSS/layout changes, in both the light and dark themes and across three viewport widths.

---

## Overview

The suite lives at `frontend/tests/visual/` and is driven by its own config, `frontend/playwright.visual.config.ts`, kept separate from the functional e2e suite (`frontend/playwright.config.ts`). Screenshot baselines are tied to a single browser engine and OS rendering stack, so the visual suite runs on Chromium only.

### Viewport projects

Every test is executed at three widths:

| Project | Width | Height |
|---|---|---|
| `visual-desktop` | 1440 px | 900 px |
| `visual-tablet` | 768 px | 1024 px |
| `visual-mobile` | 375 px | 812 px |

### Covered surfaces

Each surface below is captured in both `light` and `dark` themes at all three viewports:

| Surface | Route | Auth state |
|---|---|---|
| Landing page | `/` | Unauthenticated |
| Dashboard | `/dashboard` | Wallet connected (mocked) |
| Task detail — running state | `/tasks/:id` | Connected, mock WebSocket |
| Task detail — pending state | `/tasks/:id` | Connected |
| Task detail — completed state | `/tasks/:id` | Connected |
| Task detail — failed state | `/tasks/:id` | Connected |
| Task submission wizard step 1 | `/tasks/new` | Connected |
| Task submission wizard step 2 | `/tasks/new` | Connected |
| Task submission wizard step 3 | `/tasks/new` | Connected |
| Agent registry list view | `/agents` | Connected |
| Agent detail modal | `/agents` | Connected |
| Task / payment history | `/tasks/history` | Connected |
| Wallet — not connected | `/wallet` | Unauthenticated |
| Wallet — connected | `/wallet` | Connected |
| Error boundary fallback | `/dashboard?__trigger_error=1` | Connected |
| 404 / not found page | `/this-route-does-not-exist-404` | Any |

The dashboard and protected-route tests seed `localStorage` (`wallet_pubkey`, `walletAddress`, `wallet_connection_method`) before the app boots so `ProtectedRoute` renders the real page instead of redirecting. The task-detail test starts a local WebSocket listener on port 3001 so the connection-status chip settles on "connected".

Task states are exercised by appending `?__task_status=<state>` to the task detail URL; the MSW handler in `frontend/src/mocks/handlers.ts` reads this query param and returns the requested status so screenshots are deterministic.

Dashboard KPI and recent-task data come from MSW handlers (`GET /api/stats`, `GET /api/wallets/:address/tasks`). Agent data comes from `GET /api/agents` and `GET /api/agents/:id`.

### Stabilizing screenshots

Two sources of nondeterminism are handled explicitly:

- **CSS transitions/animations** — disabled globally via `expect.toHaveScreenshot.animations: 'disabled'` in `playwright.visual.config.ts`.
- **JS-driven motion** (framer-motion) that doesn't check `prefers-reduced-motion` — `tests/visual/utils.ts`'s `preparePage()` sets `prefers-reduced-motion: reduce`, and `waitForMotionToSettle()` adds a fixed post-`networkidle` wait so screenshots are taken after remaining transitions have settled.

---

## Running locally

```bash
cd frontend
npm run test:visual
```

This starts the Vite dev server (reusing one already running on `:3000`) and runs every visual spec against it, comparing against the committed baselines under `tests/visual/*-snapshots/`.

### Updating baselines

```bash
cd frontend
npm run test:visual:update
```

Run this whenever an intentional UI change makes existing baselines stale, then review and commit the changed PNGs under `tests/visual/pages.visual.spec.ts-snapshots/`.

**Baselines must be generated on Linux with the same browser build CI uses**, or every diff will be dominated by font/anti-aliasing noise. Generate them through the pinned Playwright Docker image:

```bash
docker run --rm -v "$(pwd)/..:/work" -w /work/frontend \
  -e TZ=UTC \
  mcr.microsoft.com/playwright:v1.61.0-jammy \
  bash -c "npm ci && npm run test:visual:update"
```

Keep the image tag in sync with the `@playwright/test` version in `frontend/package-lock.json` and with the tag pinned in `frontend-visual-regression` in `.github/workflows/ci.yml`.

### Adjusting the diff threshold

The allowed difference is `expect.toHaveScreenshot.maxDiffPixelRatio` in `frontend/playwright.visual.config.ts` (currently `0.001`, i.e. **0.1 %** of pixels may differ before a test fails). This is tight enough to catch real regressions while absorbing minor sub-pixel anti-aliasing variation between identical Linux builds.

---

## CI reporting

The `frontend-visual-regression` job in `.github/workflows/ci.yml`:

1. Runs inside the pinned `mcr.microsoft.com/playwright` image.
2. Runs `npm run test:visual`.
3. Uploads the HTML report (`frontend/playwright-report-visual/`) as the `playwright-visual-report` build artifact on every run.
4. **Fails the PR** if any surface exceeds the 0.1 % threshold (`continue-on-error: false`).

A red run means a visual diff exceeded the threshold. Open the report artifact and determine whether the diff is intentional (update baselines) or a regression (fix the code).
