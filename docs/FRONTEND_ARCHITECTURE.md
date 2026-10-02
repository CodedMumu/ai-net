# Frontend Architecture & Conventions

This document is the authoritative reference for frontend contributors. It covers folder structure, naming rules, state management patterns, component design, styling conventions, error handling, testing, and how to add new pages and features.

---

## Table of Contents

1. [Folder Structure](#1-folder-structure)
2. [Naming Conventions](#2-naming-conventions)
3. [State Management](#3-state-management)
4. [Component Patterns](#4-component-patterns)
5. [Styling Conventions](#5-styling-conventions)
6. [Error Handling](#6-error-handling)
7. [Internationalization (i18n)](#7-internationalization-i18n)
8. [Testing](#8-testing)
9. [Adding a New Page / Route](#9-adding-a-new-page--route)
10. [Verification](#10-verification)

---

## 1. Folder Structure

```
frontend/
├── playwright.config.ts            # Functional E2E Playwright config
├── playwright.visual.config.ts     # Visual regression Playwright config
├── vite.config.ts
└── src/
    ├── App.tsx                     # Root component: providers + router
    ├── main.tsx                    # Entry point — awaits i18n, mounts <App>
    ├── components/                 # Reusable UI components (domain-grouped)
    │   ├── agents/                 # Agent-specific components
    │   │   ├── AgentCard.tsx
    │   │   ├── AgentCard.module.css
    │   │   ├── AgentCard.test.tsx
    │   │   ├── AgentFilterBar.tsx
    │   │   ├── AgentRegistryCard.tsx
    │   │   ├── AgentDetailModal.tsx
    │   │   ├── AgentOutputRenderer.tsx
    │   │   ├── AgentReputationRadar.tsx
    │   │   ├── AgentReputationTrend.tsx
    │   │   ├── AgentTable.tsx
    │   │   ├── DAGPreview.tsx
    │   │   ├── DesignRenderer.tsx
    │   │   ├── CodingRenderer.tsx
    │   │   ├── RiskMatrix.tsx
    │   │   ├── ResearchReportRenderer.tsx
    │   │   ├── ReputationStars.tsx
    │   │   ├── TaskSubmissionForm.tsx
    │   │   └── TaskWizard.module.css
    │   ├── auth/                   # Auth guard components
    │   │   ├── ProtectedRoute.tsx
    │   │   └── ProtectedRoute.test.tsx
    │   ├── common/                 # Generic, domain-agnostic building blocks
    │   │   ├── AccessibleChart.tsx
    │   │   ├── CollapsibleSection.tsx
    │   │   ├── CommandPalette.tsx
    │   │   ├── CopyButton.tsx
    │   │   ├── DataTable.tsx
    │   │   ├── EmptyState.tsx
    │   │   ├── ErrorBoundary.tsx
    │   │   ├── FormField.tsx
    │   │   ├── ImageLightbox.tsx
    │   │   ├── RouteLoader.tsx
    │   │   ├── Skeleton.tsx
    │   │   └── Toast.tsx
    │   ├── dashboard/              # Dashboard-specific widgets
    │   │   ├── AgentOutputPanel.tsx
    │   │   ├── DashboardLayout.tsx
    │   │   ├── KpiCard.tsx
    │   │   ├── NetworkHealthBadge.tsx
    │   │   ├── PaymentTimeline.tsx
    │   │   ├── RecentTasksTable.tsx
    │   │   └── TaskDetailTimeline.tsx
    │   ├── landing/                # Public landing page sections
    │   │   ├── AgentCard.tsx
    │   │   ├── Footer.tsx
    │   │   ├── Hero.tsx
    │   │   ├── LiveDemoSection.tsx
    │   │   ├── Navbar.tsx
    │   │   ├── Sidebar.tsx
    │   │   ├── SpecialistAgentsSection.tsx
    │   │   ├── StatsBar.tsx
    │   │   └── ValuePropsSection.tsx
    │   ├── layout/                 # App shell, navigation, chrome
    │   │   ├── AppShell.tsx
    │   │   ├── Breadcrumb.tsx
    │   │   ├── MobileDrawer.tsx
    │   │   ├── RouteProgressBar.tsx
    │   │   ├── Sidebar.tsx
    │   │   ├── TopNav.tsx
    │   │   ├── navigation.ts       # Route/nav link definitions
    │   │   └── index.ts
    │   ├── notifications/          # Notification center and bell
    │   │   ├── NotificationBell.tsx
    │   │   ├── NotificationCenter.tsx
    │   │   ├── NotificationItem.tsx
    │   │   ├── NotificationPanel.tsx
    │   │   └── index.ts
    │   ├── tasks/                  # Task detail / history components
    │   │   ├── TaskComparison.tsx
    │   │   ├── TaskFilterBar.tsx
    │   │   └── TaskTimeline.tsx
    │   └── wallet/                 # Wallet wizard, history, charts
    │       ├── ExportButton.tsx
    │       ├── PaymentChart.tsx
    │       ├── SendXLMForm.tsx
    │       ├── TransactionTable.tsx
    │       ├── WalletWizard.tsx
    │       ├── WizardProgress.tsx
    │       └── WizardStep.tsx
    ├── context/                    # React Context providers
    │   ├── NotificationContext.tsx
    │   ├── RouteProgressContext.tsx
    │   ├── ThemeContext.tsx
    │   ├── ToastContext.tsx
    │   └── WalletContext.tsx
    ├── hooks/                      # Custom React hooks
    │   ├── useAgentRegistry.ts
    │   ├── useAgentReputation.ts
    │   ├── useAnimatedCounter.ts
    │   ├── useAuthGuard.tsx
    │   ├── useCommandPalette.ts
    │   ├── useCursorPagination.ts
    │   ├── useFocusTrap.ts
    │   ├── useLightbox.ts
    │   ├── useMediaQuery.ts
    │   ├── useNetworkStats.ts
    │   ├── useNodeState.ts
    │   ├── useNotifications.ts
    │   ├── useOnboarding.ts
    │   ├── useParticles.ts
    │   ├── useRouteProgress.ts
    │   ├── useScrollRestoration.ts
    │   ├── useSelectTypeahead.ts
    │   ├── useTaskDraft.ts
    │   ├── useTaskHistory.ts
    │   ├── useTaskMonitor.ts
    │   ├── useTaskOutputs.ts
    │   ├── useTaskPayments.ts
    │   ├── useTaskSubmit.ts
    │   ├── useTaskWebSocket.ts
    │   ├── useTheme.ts
    │   ├── useToast.ts
    │   ├── useTransactionHistory.ts
    │   ├── useTypingAnimation.ts
    │   ├── useWallet.ts
    │   └── useWalletBalance.ts
    ├── i18n/                       # i18next setup and locale files
    │   ├── index.ts                # i18n initialisation + i18nReady promise
    │   ├── options.ts              # Shared base options
    │   ├── parity.test.ts          # Key parity test across all locales
    │   ├── fallback.test.ts        # Fallback/missing key tests
    │   └── locales/
    │       ├── en.json
    │       └── zh.json
    ├── mocks/                      # MSW mock service worker handlers
    │   ├── browser.ts              # Worker setup
    │   └── handlers.ts             # Per-route request handlers
    ├── pages/                      # Route-level page components
    │   ├── AgentsPage.tsx
    │   ├── dashboard.tsx
    │   ├── LandingPage.tsx
    │   ├── NotFoundPage.tsx
    │   ├── RendererDemoPage.tsx    # Dev-only route (/renderer-demo)
    │   ├── TaskDetailPage.tsx
    │   ├── WalletPage.tsx
    │   └── tasks/
    │       ├── NewTaskPage.tsx
    │       └── TaskHistoryPage.tsx
    ├── schemas/                    # Zod validation schemas (form input)
    │   ├── task.ts
    │   └── wallet.ts
    ├── services/                   # API client and service layer
    │   ├── api.ts                  # apiClient (fetch wrapper), named endpoints
    │   ├── freighter.ts            # Freighter wallet integration
    │   └── taskService.ts
    ├── styles/                     # Global stylesheets and design tokens
    │   ├── animations.css
    │   ├── global.css
    │   ├── micro-interactions.css
    │   ├── report.css
    │   └── tokens.css              # CSS custom properties (colour, spacing, type)
    ├── types/                      # Shared TypeScript types
    │   ├── agent.ts
    │   ├── api.ts
    │   └── notification.ts
    └── utils/                      # Pure utility functions
        ├── agentRegistry.ts
        ├── agentUtils.ts
        ├── animationPresets.ts
        ├── buildLiveDag.ts
        ├── format.ts
        ├── fuzzy.ts
        └── time.ts
```

### Key rules

- **Pages are thin wrappers.** They compose context hooks and domain components; they do not contain raw JSX beyond a layout skeleton.
- **Components are stateless where possible.** Data-fetching belongs in hooks; display logic belongs in components.
- **Co-locate tests.** Every `Foo.tsx` should have a `Foo.test.tsx` next to it.

---

## 2. Naming Conventions

### Files

| Kind | Pattern | Example |
|---|---|---|
| Component | `PascalCase.tsx` | `AgentCard.tsx`, `TaskFilterBar.tsx` |
| CSS Module | `PascalCase.module.css` | `AgentCard.module.css` |
| Plain CSS | `PascalCase.css` or `kebab-case.css` | `Toast.css`, `global.css` |
| Hook | `camelCase.ts` (prefix `use`) | `useWallet.ts`, `useTaskWebSocket.ts` |
| Context | `PascalCase.tsx` (suffix `Context`) | `WalletContext.tsx` |
| Type file | `camelCase.ts` | `agent.ts`, `api.ts` |
| Schema | `camelCase.ts` | `wallet.ts`, `task.ts` |
| Utility | `camelCase.ts` | `format.ts`, `time.ts` |
| Service | `camelCase.ts` | `api.ts`, `freighter.ts` |
| Unit test | `*.test.ts` / `*.test.tsx` | `useNodeState.test.ts`, `KpiCard.test.tsx` |
| i18n test | `*.i18n.test.tsx` | `RecentTasksTable.i18n.test.tsx` |
| Skeleton test | `*.skeleton.test.tsx` | `TaskHistoryPage.skeleton.test.tsx` |

### Components

```tsx
// ✅ One component per file. File name == default export name.
// src/components/common/CopyButton.tsx

import React from 'react';
import styles from './CopyButton.module.css';

export interface CopyButtonProps {
  value: string;
  label?: string;
}

const CopyButton: React.FC<CopyButtonProps> = ({ value, label = 'Copy' }) => {
  // ...
};

export default CopyButton;
```

- **Default export** for the primary component.
- **Named exports** for associated types (`CopyButtonProps`) and sub-components.
- **No default export** for hooks, utilities, or context files — use named exports only.

### Hooks

```ts
// ✅ Named export, prefixed with `use`
// src/hooks/useWalletBalance.ts
export function useWalletBalance(publicKey: string | null) { /* ... */ }
```

### Context

```tsx
// ✅ Context + Provider in the same file
// src/context/ThemeContext.tsx
export type ThemeMode = 'light' | 'dark' | 'system'
export interface ThemeContextValue { /* ... */ }
const ThemeContext = createContext<ThemeContextValue>({ /* ... */ })
export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => { /* ... */ }
export default ThemeContext
```

### CSS classes

- Use **camelCase** inside `*.module.css` (`styles.cardTitle`, `styles.isActive`).
- Use **kebab-case** in plain CSS files and Tailwind class names (`text-accent`, `py-4`).
- CSS custom property names use `--kebab-case` (`--surface-canvas`, `--text-primary`).

---

## 3. State Management

The frontend has no Redux or Zustand. State lives in three layers, each with a clear responsibility.

### Layer 1 — React Context (cross-cutting global state)

Use Context for concerns that many components at different levels of the tree need simultaneously.

| Context | File | Purpose |
|---|---|---|
| `WalletContext` | `context/WalletContext.tsx` | Stellar wallet connection, public key, signing |
| `ThemeContext` | `context/ThemeContext.tsx` | Dark/light/system mode with `localStorage` persistence |
| `ToastContext` | `context/ToastContext.tsx` | Global toast notification queue |
| `NotificationContext` | `context/NotificationContext.tsx` | In-app notification feed |
| `RouteProgressContext` | `context/RouteProgressContext.tsx` | Top-of-page route loading bar |

Provider order in `App.tsx` (outermost first):

```tsx
<I18nextProvider i18n={i18n}>
  <ErrorBoundary>
    <ThemeProvider>
      <WalletProvider>
        <ToastProvider>
          <Router>
            <RoutedContent />
          </Router>
        </ToastProvider>
      </WalletProvider>
    </ThemeProvider>
  </ErrorBoundary>
</I18nextProvider>
```

Creating a new context:

```tsx
// src/context/FooContext.tsx
import React, { createContext, useContext, useState } from 'react';

interface FooContextValue {
  count: number;
  increment: () => void;
}

const FooContext = createContext<FooContextValue>({ count: 0, increment: () => {} });

export const FooProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [count, setCount] = useState(0);
  return (
    <FooContext.Provider value={{ count, increment: () => setCount(c => c + 1) }}>
      {children}
    </FooContext.Provider>
  );
};

export function useFoo() {
  return useContext(FooContext);
}

export default FooContext;
```

### Layer 2 — Custom hooks (server state + component data)

All data-fetching, WebSocket subscriptions, and derived server state live in custom hooks. They call `services/api.ts` directly — there is no React Query.

```ts
// src/hooks/useAgentRegistry.ts  (simplified excerpt)
import { useCallback, useEffect, useRef, useState } from 'react';
import { getAgents } from '../services/api';
import type { AgentRecord } from '../types/api';

export interface AgentRegistryResult {
  agents: AgentRecord[];
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

export function useAgentRegistry(): AgentRegistryResult {
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  const fetchAgents = useCallback(async () => {
    try {
      const data = await getAgents();
      if (!mountedRef.current) return;
      setAgents(data);
      setError(null);
    } catch (err) {
      if (!mountedRef.current) return;
      setError(err instanceof Error ? err.message : 'Failed to load agents');
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    fetchAgents();
    return () => { mountedRef.current = false; };
  }, [fetchAgents]);

  return { agents, loading, error, refetch: fetchAgents };
}
```

Patterns enforced in all data-fetching hooks:

- **Mount guard** (`mountedRef`) — never call `setState` after unmount.
- **`loading` is true only on first load** — background refreshes update data in place.
- **`error` is a `string | null`** — not an `Error` object, so it serializes cleanly to JSX.
- **Cleanup in `useEffect` return** — clear intervals, remove event listeners, abort fetches.

### Layer 3 — Local component state

Use `useState` and `useReducer` for ephemeral UI state (open/closed modals, form field values, selected tabs). Keep it co-located in the component that owns it; lift up only when two siblings genuinely need the same value.

### Rules

1. **No Redux or Zustand.** Context + hooks is the entire pattern.
2. **Prefer co-location.** Keep state close to where it is used.
3. **Memoize sparingly.** `useMemo` and `useCallback` only when profiling shows a real cost.
4. **Lift state up** only when two sibling components need the same live value and there is no natural ancestor already providing a context.

---

## 4. Component Patterns

The project uses a loose atomic-design layering without enforcing strict Atomic Design folder names.

### Atoms — `components/common/`

Small, generic, stateless building blocks with no domain knowledge:

- `Skeleton`, `SkeletonText`, `SkeletonCard` — loading placeholders
- `Toast` — single notification message
- `EmptyState` — zero-data placeholder with icon + message
- `FormField` — label + input wrapper with error text
- `CopyButton` — clipboard copy with feedback
- `DataTable` — generic sortable table wrapper
- `CollapsibleSection` — expand/collapse panel
- `AccessibleChart` — chart wrapper with ARIA table fallback

```tsx
// src/components/common/EmptyState.tsx
import React from 'react';
import styles from './EmptyState.module.css';

export interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
}

const EmptyState: React.FC<EmptyStateProps> = ({ icon, title, description, action }) => (
  <div className={styles.root} role="status">
    {icon && <div className={styles.icon}>{icon}</div>}
    <h3 className={styles.title}>{title}</h3>
    {description && <p className={styles.description}>{description}</p>}
    {action && <div className={styles.action}>{action}</div>}
  </div>
);

export default EmptyState;
```

### Molecules — domain components

Composed from atoms and local hooks. Have domain knowledge but no router or context imports:

- `KpiCard` — metric value + label + sparkline
- `AgentRegistryCard` — agent name, capabilities, reputation stars
- `NotificationItem` — single notification with timestamp and type badge
- `WizardStep`, `WizardProgress` — multi-step wizard primitives

### Organisms — `components/<domain>/` complex composites

Full sections that own their data via hooks:

- `TaskSubmissionForm` — multi-step form with Zod validation, DAG preview, and `useTaskSubmit`
- `DAGPreview` — live SVG DAG with agent nodes from `useTaskWebSocket`
- `TransactionTable` — paginated, sortable, filterable table using `useTransactionHistory`
- `TaskTimeline` — chronological event stream from WebSocket stream
- `CommandPalette` — keyboard-searchable command palette using `useCommandPalette`

### Pages — `pages/`

Pages compose organisms and wrappers. Keep them thin:

```tsx
// src/pages/tasks/TaskHistoryPage.tsx  (simplified)
import React, { Suspense } from 'react';
import { useTranslation } from 'react-i18next';
import { useWallet } from '../hooks/useWallet';
import { TaskFilterBar } from '../components/tasks/TaskFilterBar';
import { SkeletonTable } from '../components/common/Skeleton';
import styles from './TaskHistoryPage.module.css';

// Lazy-loaded table (code-split) — heavy component
const TaskHistoryTable = React.lazy(() => import('../components/tasks/TaskHistoryTable'));

const TaskHistoryPage: React.FC = () => {
  const { t } = useTranslation();
  const { address } = useWallet();

  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>{t('tasks.history.title')}</h1>
      <TaskFilterBar />
      <Suspense fallback={<SkeletonTable rows={8} columns={5} />}>
        <TaskHistoryTable walletAddress={address} />
      </Suspense>
    </div>
  );
};

export default TaskHistoryPage;
```

### Component checklist

Before submitting a new component:

- [ ] Props interface exported with a `Props` suffix (`AgentCardProps`).
- [ ] Accessible: meaningful ARIA roles/labels on interactive elements.
- [ ] Loading state: renders `Skeleton` variants while data is loading.
- [ ] Empty state: renders `EmptyState` when data is an empty array.
- [ ] Error state: surfaces `error` from the hook with a descriptive message.
- [ ] `data-testid` on key DOM nodes for test targeting.
- [ ] Matching `.module.css` file for any custom styles.
- [ ] Co-located `.test.tsx` file.

---

## 5. Styling Conventions

The frontend uses a **Tailwind CSS + CSS Modules hybrid** approach, backed by a CSS custom-property design token system.

### Design tokens (`src/styles/tokens.css`)

All colours, spacing steps, shadows, and type scale are defined as CSS custom properties on `:root` (dark theme) and `.theme-light` (light theme override). **Never hard-code hex values in component stylesheets.** Always reference a token:

```css
/* ✅ correct */
.card {
  background: var(--surface-raised);
  color: var(--text-primary);
  border: 1px solid var(--border-subtle);
  box-shadow: var(--shadow-md);
}

/* ❌ wrong */
.card {
  background: #1a1f2e;
  color: #f5f7fa;
}
```

Key token groups:

| Prefix | Examples |
|---|---|
| `--surface-*` | `--surface-canvas`, `--surface-raised`, `--surface-glass` |
| `--text-*` | `--text-primary`, `--text-secondary`, `--text-muted` |
| `--accent-*` | `--accent`, `--accent-hover`, `--accent-surface` |
| `--border-*` | `--border-primary`, `--border-subtle` |
| `--status-*` | `--status-success`, `--status-danger`, `--status-warning` |
| `--shadow-*` | `--shadow-sm`, `--shadow-md`, `--shadow-lg` |

### CSS Modules (component-scoped styles)

Use CSS Modules for any styles that are specific to one component. The class names are scoped automatically by Vite:

```tsx
// AgentRegistryCard.tsx
import styles from './AgentRegistryCard.module.css';

const AgentRegistryCard = () => (
  <div className={styles.card}>
    <span className={styles.agentName}>…</span>
  </div>
);
```

```css
/* AgentRegistryCard.module.css */
.card {
  background: var(--surface-raised);
  border-radius: 12px;
  padding: 1.25rem;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.agentName {
  font-weight: 600;
  color: var(--text-primary);
}
```

Multiple conditional classes: use an array join rather than a classnames library:

```tsx
const cls = [
  styles.button,
  isActive && styles.active,
  disabled && styles.disabled,
  className,
].filter(Boolean).join(' ');
```

### Tailwind utility classes

Use Tailwind for:

- Layout and spacing (`flex`, `gap-4`, `px-6`, `grid`, `items-center`)
- Typography shortcuts (`text-sm`, `font-semibold`, `truncate`)
- One-off responsive overrides (`md:hidden`, `lg:grid-cols-3`)

Avoid Tailwind for colours or shadows — prefer tokens so dark/light themes work automatically.

### Plain CSS (`*.css`)

Used only for:

- Global resets and base styles (`styles/global.css`)
- Keyframe animations (`styles/animations.css`, `styles/micro-interactions.css`)
- Third-party component overrides (e.g. `NotificationCenter.css`)
- Layout components where `:is()`, `:has()`, or complex selectors are required (`TopNav.css`, `Sidebar.css`)

### When to use which

| Situation | Approach |
|---|---|
| Component-local styles | CSS Module |
| Layout + spacing | Tailwind utilities |
| Animation / keyframes | Plain CSS (global or co-located) |
| Colour / shadow / border | CSS token via `var(--...)` |
| Global resets, base typography | `styles/global.css` |
| Dark/light theming | CSS tokens in `tokens.css` |

---

## 6. Error Handling

### `ErrorBoundary` (class component)

`ErrorBoundary` wraps the entire app in `App.tsx`. It catches render-time exceptions and displays a localized fallback with a reload button and a generated error code.

```tsx
// src/components/common/ErrorBoundary.tsx
export class ErrorBoundary extends Component<Props, State> {
  static getDerivedStateFromError(_error: Error): State {
    const errorCode = `ERR-${Math.random().toString(36).substring(2, 8).toUpperCase()}`;
    return { hasError: true, errorCode };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo);
    // TODO: send to a monitoring service (e.g. Sentry)
  }

  render() {
    if (this.state.hasError) {
      return <ErrorFallback errorCode={this.state.errorCode} />;
    }
    return this.props.children;
  }
}
```

Wrap subsections of the UI with additional boundaries to prevent one broken widget from bringing down the whole page:

```tsx
<ErrorBoundary>
  <WalletPage />
</ErrorBoundary>
```

### Hook-level error state

Every data-fetching hook returns `{ data, loading, error }`. Components are responsible for rendering the error string:

```tsx
const { agents, loading, error } = useAgentRegistry();

if (loading) return <SkeletonTable rows={5} columns={4} />;
if (error)   return <EmptyState title="Failed to load agents" description={error} />;
```

### API errors (`ApiError`)

`services/api.ts` throws `ApiError` (extends `Error`) for non-2xx responses. It carries `statusCode` and `path` for structured logging:

```ts
export class ApiError extends Error {
  statusCode: number;
  path: string;
  constructor(statusCode: number, message: string, path: string) { /* ... */ }
}
```

Catching in a hook:

```ts
} catch (err) {
  setError(err instanceof Error ? err.message : 'An unexpected error occurred');
}
```

### Global toast for transient errors

Use `useToast` from `ToastContext` for errors that do not belong to a specific data table (e.g. failed form submissions, WebSocket disconnects):

```tsx
const { showToast } = useToast();

try {
  await submitTask(payload);
} catch (err) {
  showToast(err instanceof Error ? err.message : 'Task submission failed', 'error');
}
```

### 401 / wallet session expiry

`services/api.ts` dispatches `wallet_disconnected` on a `401` response. `WalletContext` listens for this event and resets the connection state. Components do not need special handling.

### Route-level loading (`Suspense` + `RouteLoader`)

Pages are lazy-loaded with `React.lazy`. The `<Suspense>` boundary in `App.tsx` renders `<RouteLoader>` while the chunk is fetching:

```tsx
<Suspense fallback={<RouteLoadingFallback />}>
  <Routes> … </Routes>
</Suspense>
```

---

## 7. Internationalization (i18n)

The app uses **i18next** with **react-i18next**. Locale files are at `src/i18n/locales/` and currently support English (`en.json`) and Simplified Chinese (`zh.json`).

### Initialization

`i18nReady` in `src/i18n/index.ts` is a Promise resolved before the first render in `main.tsx`. This guarantees no translation keys are ever painted as raw strings:

```ts
// main.tsx
import { i18nReady } from './i18n';
i18nReady.then(() => {
  ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
});
```

### Using translations in components

```tsx
import { useTranslation } from 'react-i18next';

const MyComponent = () => {
  const { t } = useTranslation();
  return <h1>{t('dashboard.title')}</h1>;
};
```

### Adding new translation keys

1. Add the key to `src/i18n/locales/en.json` (English is the source of truth).
2. Add the same key to every other locale (`zh.json`, etc.).
3. The parity test at `src/i18n/parity.test.ts` will fail if a key is missing in any locale.

```json
// en.json (excerpt)
{
  "tasks": {
    "history": {
      "title": "Task History",
      "empty": "No tasks found."
    }
  }
}
```

### Writing i18n tests

Use `*.i18n.test.tsx` suffix for tests that exercise language switching:

```tsx
// RecentTasksTable.i18n.test.tsx
it('translates column headers on language switch', async () => {
  renderTable();
  await waitFor(() => expect(screen.getByText('Task ID')).toBeInTheDocument());

  await act(async () => { await i18n.changeLanguage('zh'); });
  expect(screen.getByText('任务 ID')).toBeInTheDocument();
});
```

### Language detection

Language is detected in order: `localStorage` → `navigator`. The detected language is stored in `localStorage` under `i18nextLng`. The `<html lang>` attribute is kept in sync with `i18n.resolvedLanguage` by a listener in `src/i18n/index.ts`.

---

## 8. Testing

The frontend uses **Vitest** (unit/integration) and **Playwright** (E2E + visual regression).

### Unit and integration tests (Vitest + Testing Library)

Run with:

```bash
cd frontend
npm test                  # watch mode
npm run test:coverage     # coverage with thresholds (≥70%)
```

#### File conventions

| Test type | Suffix | What to test |
|---|---|---|
| Component | `*.test.tsx` | Rendering, user interactions, accessibility |
| Hook | `*.test.ts` | State transitions, async fetch behaviour |
| i18n | `*.i18n.test.tsx` | Language-switch fidelity |
| Skeleton | `*.skeleton.test.tsx` | Loading state renders correct skeleton variant |
| Utility | `*.test.ts` | Pure function edge cases |

#### Test structure

```tsx
// src/components/dashboard/KpiCard.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { KpiCard } from './KpiCard';

describe('KpiCard', () => {
  it('renders the label and value', () => {
    render(<KpiCard label="Active Agents" value={42} />);
    expect(screen.getByText('Active Agents')).toBeInTheDocument();
    expect(screen.getByText('42')).toBeInTheDocument();
  });

  it('renders a skeleton while loading', () => {
    render(<KpiCard label="Active Agents" value={null} loading />);
    expect(screen.getByTestId('kpi-skeleton')).toBeInTheDocument();
  });
});
```

#### Mocking API calls

Use `vi.mock` with `vi.hoisted` for service mocks:

```ts
const getAgents = vi.hoisted(() => vi.fn());
vi.mock('@services/api', () => ({ getAgents }));

it('shows agents when fetch resolves', async () => {
  getAgents.mockResolvedValue([mockAgent]);
  render(<AgentTable />);
  await waitFor(() => expect(screen.getByText(mockAgent.name)).toBeInTheDocument());
});
```

#### Mocking WebSocket (MSW / custom)

For WebSocket-dependent hooks, a lightweight in-process WebSocket server is spun up in the test:

```ts
// Simplified pattern from useTaskWebSocket.test.ts
import { WebSocketServer } from 'ws';

let wss: WebSocketServer;
beforeEach(() => { wss = new WebSocketServer({ port: 3001 }); });
afterEach(() => wss.close());
```

#### Context wrappers

Helper `renderWithProviders` wraps components in all required contexts:

```tsx
// tests/utils/renderWithProviders.tsx
export const renderWithProviders = (ui: React.ReactElement) =>
  render(
    <ToastProvider>
      <ThemeProvider>
        {ui}
      </ThemeProvider>
    </ToastProvider>
  );
```

### Playwright E2E tests

```bash
cd frontend
npm run test:e2e           # functional E2E suite
```

Config: `frontend/playwright.config.ts`. Tests live under `frontend/tests/e2e/`.

### Visual regression tests (Playwright)

Screenshot-based tests guard against unintentional CSS/layout regressions across both themes.

```bash
cd frontend
npm run test:visual              # run against committed baselines
npm run test:visual:update       # regenerate baselines after intentional changes
```

Config: `frontend/playwright.visual.config.ts`. Screenshots: `frontend/tests/visual/*-snapshots/`.

**Important:** Baselines must be generated on Linux with the pinned Playwright image to avoid font/anti-aliasing drift:

```bash
docker run --rm -v "$(pwd)/..:/work" -w /work/frontend \
  -e TZ=UTC \
  mcr.microsoft.com/playwright:v1.61.0-jammy \
  bash -c "npm ci && npm run test:visual:update"
```

Covered surfaces (each in `light` and `dark`):

| Surface | Route |
|---|---|
| Landing page | `/` |
| Dashboard | `/dashboard` |
| Task detail | `/tasks/:id` |
| Wallet | `/wallet` |

For full details, see [docs/visual-regression-testing.md](visual-regression-testing.md).

---

## 9. Adding a New Page / Route

Follow these steps to add a new route end-to-end.

### Step 1 — Create the page component

```tsx
// src/pages/AnalyticsPage.tsx
import React from 'react';
import { useTranslation } from 'react-i18next';
import styles from './AnalyticsPage.module.css';

const AnalyticsPage: React.FC = () => {
  const { t } = useTranslation();
  return (
    <div className={styles.page}>
      <h1>{t('analytics.title')}</h1>
      {/* compose domain components here */}
    </div>
  );
};

export default AnalyticsPage;
```

### Step 2 — Create the CSS Module

```css
/* src/pages/AnalyticsPage.module.css */
.page {
  padding: 2rem;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
```

### Step 3 — Register the route in `App.tsx`

Lazy-load the page to keep the initial bundle small:

```tsx
// src/App.tsx  (inside the lazy imports block)
const AnalyticsPage = lazy(() => import('./pages/AnalyticsPage'));

// Inside <Routes> inside <AppShell>:
<Route
  path="/analytics"
  element={
    <ProtectedRoute>
      <AnalyticsPage />
    </ProtectedRoute>
  }
/>
```

Remove `<ProtectedRoute>` if the page is publicly accessible (like `LandingPage`).

### Step 4 — Add a navigation link

Navigation items are defined in `src/components/layout/navigation.ts`. Add a new entry:

```ts
// src/components/layout/navigation.ts
export const navLinks = [
  // existing entries…
  { path: '/analytics', labelKey: 'nav.analytics', icon: BarChartIcon },
];
```

### Step 5 — Add i18n keys

```json
// src/i18n/locales/en.json
{
  "nav": {
    "analytics": "Analytics"
  },
  "analytics": {
    "title": "Analytics"
  }
}
```

Repeat for every other locale (`zh.json`, etc.).

### Step 6 — Write a page test

```tsx
// src/pages/AnalyticsPage.test.tsx
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import AnalyticsPage from './AnalyticsPage';

it('renders the page heading', () => {
  render(
    <MemoryRouter>
      <AnalyticsPage />
    </MemoryRouter>
  );
  expect(screen.getByRole('heading', { name: /analytics/i })).toBeInTheDocument();
});
```

### New component checklist

1. `ComponentName.tsx` in `components/<category>/`
2. `ComponentName.module.css` if custom styles are needed
3. `ComponentName.test.tsx` next to the component
4. Export the component as the default export
5. Export its props interface as a named export

### New hook checklist

1. `useHookName.ts` in `hooks/`
2. `useHookName.test.ts` in `hooks/`
3. Named export only, prefixed with `use`
4. Return `{ data, loading, error, ...actions }` for data-fetching hooks
5. Add a mount guard (`mountedRef`) if it calls `setState` after async work

---

## 10. Verification

Before pushing a frontend change, run:

```bash
cd frontend
npm run lint          # ESLint (naming violations, unused imports, type issues)
npm run typecheck     # tsc --noEmit
npm run test:coverage # Vitest with ≥70% coverage threshold
npm run build         # Vite production bundle (must compile without errors)
```

Or use the repo-wide quality gate from the root:

```bash
npm run gate
```

See [CONTRIBUTING.md](../CONTRIBUTING.md) for the full quality gate specification.
