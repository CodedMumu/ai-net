# 🤝 Contributing to ai-net

Thank you for contributing to **ai-net**! We are building a decentralized autonomous agent network on the **Stellar blockchain** powered by **Venice AI**.

This handbook establishes our complete **Software Development Life Cycle (SDLC)**, branching strategy, conventional commit standards, PR templates, review processes, testing requirements, documentation standards, CI/CD pipeline overview, and contributor recognition.

---

## Table of Contents

1. [Code of Conduct](#1-code-of-conduct)
2. [Finding Issues to Work On](#2-finding-issues-to-work-on)
3. [Branching Strategy](#3-branching-strategy)
4. [Conventional Commits Format](#4-conventional-commits-format)
5. [Pull Request Checklist](#5-pull-request-checklist)
6. [Review Process](#6-review-process)
7. [Testing Requirements](#7-testing-requirements)
8. [Documentation Requirements](#8-documentation-requirements)
9. [CI/CD Pipeline Overview](#9-cicd-pipeline-overview)
10. [Contributor Recognition](#10-contributor-recognition)
11. [Agent & Contributor Guidelines (`AGENTS.md`)](#11-agent--contributor-guidelines)

---

## 1. Code of Conduct

ai-net is committed to providing a welcoming, safe, and constructive environment for all contributors. By participating in this project, you agree to uphold the following standards.

### Our Pledge

We pledge to make participation in our project a harassment-free experience for everyone, regardless of age, body size, visible or invisible disability, ethnicity, sex characteristics, gender identity and expression, level of experience, education, socio-economic status, nationality, personal appearance, race, caste, color, religion, or sexual identity and orientation.

### Our Standards

Positive behaviors that contribute to a great environment include:

- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

Unacceptable behaviors include:

- Trolling, insulting or derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate in a professional setting

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported by opening a private issue or contacting the maintainers directly. All complaints will be reviewed and investigated promptly and fairly.

> **Philosophy**: High Quality over Quantity. Every line of code should be tested, documented, and production-ready. Safety on Stellar — smart contracts manage real XLM. We adhere to defensive programming, minimal state footprint, and strict authorization patterns.

---

## 2. Finding Issues to Work On

### Where to Start

Browse the issue tracker for tagged issues:

- **`good first issue`** — Scoped, well-defined tasks ideal for first-time contributors. No deep codebase knowledge required.
- **`help wanted`** — Issues where maintainers actively need community assistance. May require deeper context.
- **`Stellar Wave`** — Issues designated for the current community sprint cycle.

### Issue Labels Reference

| Label | Meaning |
|---|---|
| `good first issue` | Beginner-friendly, clear scope |
| `help wanted` | Community help actively sought |
| `bug` | Something is not working as expected |
| `enhancement` | A new feature or improvement |
| `documentation` | Docs-only changes |
| `smart-contract` | Soroban/Rust contract work |
| `backend` | Node.js/TypeScript backend work |
| `frontend` | React/Vite UI work |
| `testing` | Test coverage improvements |
| `devops` | CI/CD, infrastructure, tooling |
| `security` | Security-related fixes or hardening |
| `Stellar Wave` | Current sprint cycle |

### Before You Start

1. **Check for existing work** — Search open PRs to make sure the issue isn't already being worked on.
2. **Leave a comment** — On the issue, state your intent to work on it. Maintainers will assign it to you to avoid duplicated effort.
3. **Ask questions early** — If the scope is unclear, ask in the issue before coding. It is much easier to clarify intent before than to rework code after.
4. **Read relevant docs** — Check `docs/`, `AGENTS.md`, and the relevant component's README before starting.

### Exploring the Codebase

```bash
# Quick orientation
cat README.md
cat AGENTS.md
cat docs/DEVELOPER_SETUP.md

# Run tests to verify your environment
npm run gate
```

---

## 3. Branching Strategy

All branches must follow standardized prefixes with an issue number suffix.

### Branch Naming Rules

```
<prefix>/<short-description>-<issue-number>
```

| Type | Prefix | Example | Purpose |
|---|---|---|---|
| **Features** | `feat/` | `feat/freighter-navbar-183` | New feature, UI component, or capability |
| **Bug Fixes** | `fix/` | `fix/cursor-nonce-recovery-241` | Bug or edge-case resolution |
| **Documentation** | `docs/` | `docs/contributing-sdlc-70` | New or updated documentation |
| **Performance** | `perf/` | `perf/storage-compaction-335` | Gas optimization or caching improvements |
| **Refactoring** | `refactor/` | `refactor/venice-client-pool` | Code restructuring without behavior changes |
| **Testing** | `test/` | `test/backend-integration-200` | Adding or fixing test suites |
| **CI/DevOps** | `ci/` | `ci/dependabot-sbom-209` | Workflow, pipeline, or tooling changes |
| **Chore** | `chore/` | `chore/bump-stellar-sdk` | Dependency bumps, maintenance |

### Working With Branches

```bash
# 1. Sync your fork with upstream before creating a branch
git fetch upstream
git checkout main
git merge upstream/main

# 2. Create your branch from the synced main
git checkout -b feat/my-feature-123

# 3. Keep your branch up to date during development
git fetch upstream
git rebase upstream/main
```

> **Never commit directly to `main`.** All changes must go through a PR, regardless of how small.

---

## 4. Conventional Commits Format

Every commit message and PR title must adhere to [Conventional Commits 1.0.0](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description in present tense> (#<issue>)

[optional body explaining rationale, architectural decisions, and trade-offs]

[optional footer: Resolves #<issue>]
```

### Allowed Types

| Type | When to Use |
|---|---|
| `feat` | A new feature for users, agents, or smart contracts |
| `fix` | A bug fix |
| `docs` | Documentation-only changes |
| `test` | Adding missing tests or correcting existing tests |
| `perf` | A code change that improves gas efficiency or execution speed |
| `refactor` | A code change that neither fixes a bug nor adds a feature |
| `chore` | Maintenance tasks, dependency bumps, or toolchain updates |
| `ci` | Changes to CI/CD workflows and automation scripts |

### Common Scopes

| Scope | Applies To |
|---|---|
| `contracts` | Soroban smart contracts (agent-registry, payment-escrow, error-resolver, etc.) |
| `backend` | Node.js/TypeScript coordinator, API routes, and database models |
| `frontend` | React/Vite web application and dashboard |
| `sdk` | Client libraries and type definitions |
| `devops` | CI/CD, Docker, infrastructure-as-code |
| `docs` | Documentation files in `docs/` |

### Good Commit Examples

```
feat(frontend): add Freighter wallet connect button to TopNav (#183)

Adds a "Connect Wallet" button that triggers the Freighter browser
extension flow. Displays truncated public key and XLM balance once
connected. Handles extension-not-installed and user-rejected errors
via toast notifications.

Resolves #183
```

```
fix(backend): resolve nonce desynchronization on concurrent escrow releases (#214)
```

```
docs: expand CONTRIBUTING.md with full SDLC and PR review process (#70)
```

```
ci: configure Dependabot for npm and Cargo ecosystems with SBOM generation (#209)
```

### What to Avoid

- ❌ `fix: stuff` — too vague
- ❌ `WIP: working on feature` — do not push WIP commits to shared branches
- ❌ `update files` — describes nothing
- ❌ Merge commits inside a feature branch — use `git rebase` instead

---

## 5. Pull Request Checklist

Use the PR template when opening a pull request (`.github/PULL_REQUEST_TEMPLATE.md`).

### Before Marking PR Ready for Review

**For all PRs:**
- [ ] PR title follows Conventional Commits format with issue number
- [ ] Branch is up to date with `upstream/main` (no conflicts)
- [ ] `npm run gate` passes locally (format + lint + typecheck + coverage)
- [ ] All new code has corresponding tests
- [ ] No secrets, API keys, or `.env` files are tracked
- [ ] The PR description includes a summary, what was tested, and `Resolves #<issue>`

**For backend changes:**
- [ ] `cd backend && npm run lint && npm run typecheck && npm run test:coverage`
- [ ] New endpoints documented in `docs/API_REFERENCE.md` or `backend/docs/`
- [ ] Database schema changes include migration files under `backend/src/db/migrations/`
- [ ] Rate-limiting and auth middleware applied to new routes

**For frontend changes:**
- [ ] `cd frontend && npm run lint && npm run typecheck && npm run build`
- [ ] Components are accessible (WCAG 2.1 AA — keyboard navigation, ARIA labels)
- [ ] Responsive layout tested at mobile, tablet, and desktop breakpoints
- [ ] New UI strings use `i18n` translation keys; both `en` and `zh` locales updated
- [ ] Visual regression baseline updated if UI changed: `npm run test:visual`

**For smart contract changes:**
- [ ] `cd smart-contracts && cargo fmt --check && cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test` passes for all affected contracts
- [ ] Every on-chain mutation calls `require_auth()`
- [ ] Storage access uses correct type (Instance/Persistent/Temporary)
- [ ] Gas and storage footprint reviewed — no unbounded vectors in contract storage

---

## 6. Review Process

### Step-by-Step PR Flow

1. **Fork & Branch** — Fork `CodedSceptre/ai-net` and cut a branch from `upstream/main` using the naming conventions above.
2. **Implement & Test** — Ensure all tests, lints, and format checks pass locally with `npm run gate`.
3. **Commit Cleanly** — Use Conventional Commits. Squash WIP commits before opening the PR.
4. **Open PR** — Use the PR template. Fill in all checklist items.
5. **CI Must Be Green** — All GitHub Actions workflows must pass before requesting review.
6. **Request Review** — Request at least **2 reviewers** from the maintainer team:
   - Tag `@CodedSceptre/maintainers` for general reviews
   - For smart contract changes, also tag a contract specialist
7. **Address Feedback** — Respond to all review comments. Push updates as new commits (do not force-push during review).
8. **Merge** — A maintainer will merge once 2 approvals are received and all CI checks are green.

### Review Approval Requirements

| Change Type | Minimum Approvals | Required Reviewers |
|---|---|---|
| Documentation only | 1 | Any maintainer |
| Frontend / Backend feature | 2 | Any 2 maintainers |
| Smart contract changes | 2 | 1 must be a contract specialist |
| Security-sensitive changes | 2 | 1 must be a security reviewer |
| Breaking API changes | 2 | All active maintainers must be notified |

### Review SLOs

- Maintainers aim to provide an initial review within **24–48 hours** of PR submission.
- Authors are expected to address review feedback within **3 business days**.
- PRs with no activity for **7 days** after feedback may be closed (reopenable on request).

### For Reviewers

When reviewing a PR, check:

- [ ] Architecture aligns with system design and existing layers (see `docs/architecture/`)
- [ ] Storage and gas costs on Soroban are bounded and minimized
- [ ] Database queries are parameterized and properly indexed
- [ ] Error messages are informative and do not leak internal credentials or stack traces
- [ ] New dependencies are justified, pinned to exact versions, and not typosquatted
- [ ] Tests cover happy paths, edge cases, and error boundaries
- [ ] Breaking changes are documented and migration paths provided

---

## 7. Testing Requirements

### 7.1 Unified Quality Gate

Run this single command before every push:

```bash
npm run gate
```

This executes formatting check, linting, type-checking, and test coverage in one pass. All four must pass before a PR can be merged.

### 7.2 Coverage Thresholds

| Component | Minimum Coverage |
|---|---|
| Backend (Jest) | **75%** statements, branches, functions, lines |
| Frontend (Vitest) | **70%** statements, branches, functions, lines |
| Smart contracts (cargo test) | All unit tests must pass (coverage aspirational at 80%) |

Contributions that reduce coverage below these thresholds will not be merged.

### 7.3 Backend Testing

```bash
cd backend
npm ci
npm run lint             # ESLint check
npm run typecheck        # tsc --noEmit
npm test                 # Unit tests
npm run test:coverage    # Coverage with thresholds (≥75%)
npm run test:integration # Integration tests (Express + in-memory SQLite + MSW)
```

Integration tests live in `backend/tests/integration/` and use:
- **supertest** — HTTP assertions against the full Express app
- **msw** — Mock Service Worker for intercepting Venice AI HTTP calls
- **jest-fake-timers** — Simulate time progression for heartbeat/stale detection

### 7.4 Frontend Testing

```bash
cd frontend
npm ci
npm run lint             # ESLint check
npm run typecheck        # tsc --noEmit
npm test                 # Vitest unit tests
npm run test:coverage    # Coverage with thresholds (≥70%)
npm run build            # Production bundle — must compile cleanly
npm run test:visual      # Playwright visual regression (update baselines if UI changed)
```

### 7.5 Smart Contract Testing

```bash
cd smart-contracts
cargo fmt --all -- --check                                   # Formatting
cargo clippy --all-targets -- -D warnings                    # Linter
cargo test --locked                                          # All unit tests
cargo build --target wasm32v1-none --release                 # Optimized Wasm
```

### 7.6 Test Naming Conventions

- Unit test files: `<module>.test.ts` (co-located with source) or `tests/<module>.test.ts`
- Integration test files: `tests/integration/<feature>.test.ts`
- E2E test files: `tests/e2e/<scenario>.spec.ts` or `frontend/tests/e2e/<scenario>.spec.ts`
- Test descriptions should read as sentences: `it('returns 429 when rate limit exceeded', ...)`

### 7.7 What Must Be Tested

Every PR that adds or changes behavior must include:

- **Happy path** — the primary success case
- **Error cases** — invalid input, network failure, unauthorized access
- **Edge cases** — empty results, boundary values, concurrent access
- **Integration** — for API changes, at least one integration test covering the full request/response cycle

---

## 8. Documentation Requirements

### When Documentation Is Required

| Change | Required Doc Update |
|---|---|
| New API endpoint | `docs/API_REFERENCE.md` + curl examples |
| New CLI command or script | Inline `--help` text + relevant `docs/` page |
| New environment variable | `.env.example` + `docs/DEVELOPER_SETUP.md` |
| Database schema change | Migration file + `README.md` migration section |
| New smart contract entry point | `smart-contracts/docs/` + `docs/ai-agent-integration-guide.md` |
| Breaking change | `CHANGELOG.md` + migration guide in `docs/MIGRATION_GUIDE.md` |
| New component or hook (frontend) | JSDoc comment + `docs/FRONTEND_ARCHITECTURE.md` if pattern is new |

### Documentation Standards

- Write in clear, direct English. Prefer active voice.
- All code samples must be runnable — verify them before committing.
- Update the `docs/README.md` table of contents when adding new doc files.
- Link related docs cross-referentially so readers can navigate the full context.
- Keep `README.md` in sync — if the architecture, quick start, or structure changes, update the root README.

### Inline Code Documentation

- All exported functions, classes, and types must have JSDoc comments (TypeScript) or doc comments (Rust).
- Comments should explain **why**, not **what**. Avoid restating the code.
- Mark TODOs with `// TODO(#<issue>): description` so they're traceable.

---

## 9. CI/CD Pipeline Overview

Every PR triggers the full CI pipeline defined in `.github/workflows/ci.yml`. All jobs must be green before merge.

### CI Jobs

| Job | Trigger | What It Checks |
|---|---|---|
| `backend` | Always | Lint, typecheck, build, unit tests + coverage (≥75%) |
| `frontend` | Always | Lint, typecheck, build, unit tests + coverage (≥70%) |
| `contracts` | Always | Cargo fmt, clippy, tests, Wasm builds |
| `frontend-visual` | Always (non-blocking) | Playwright visual regression screenshots |
| `e2e-backend-smoke` | Backend path changes | Fast E2E: agent registration, task creation |
| `e2e-backend-heavy` | Backend path changes | Full E2E: task lifecycle, WebSocket, agent removal |
| `e2e-frontend-smoke` | Frontend path changes | Fast Playwright: registry, task submission |
| `e2e-frontend-heavy` | Frontend path changes | Playwright: wallet, monitoring, i18n |
| `ci-gate` | Always | Aggregator — passes only if all required jobs pass |

The `ci-gate` job is the single required check for branch protection. A PR cannot merge if `ci-gate` fails.

### Security Audits in CI

- **`npm audit --audit-level=high`** runs on every PR for both `frontend/` and `backend/` — high or critical CVEs fail the build.
- **`cargo audit`** runs in the `contracts` job — high/critical CVEs fail the build.
- **Dependabot** is configured to automatically raise PRs for outdated dependencies on a weekly schedule (see `.github/dependabot.yml`).

### Release Pipeline

On a published GitHub Release:
- The `smart-contracts.yml` workflow builds optimized Wasm binaries, generates SHA256 checksums, and attaches them as release artifacts.
- **SBOM generation** (`@cyclonedx/cyclonedx-npm`) produces a `sbom.json` artifact attached to each release for supply-chain auditability.

### Security Alerts

Dependabot security alerts are enabled on this repository. If you have write access:
1. Navigate to **Settings → Security → Dependabot alerts** to view open alerts.
2. Treat security alert PRs with the same rigor as production code — review, test, and merge promptly.

### Secrets Management

- Never commit secrets, private keys, API tokens, or `.env` files.
- Required environment variables are documented in `.env.example` with placeholder values.
- CI secrets are managed via GitHub repository secrets (`Settings → Secrets and variables`).

---

## 10. Contributor Recognition

We value and recognize every contribution to ai-net, no matter the size.

### How We Recognize Contributors

- **Release notes** — All merged PRs are listed by contributor in the `CHANGELOG.md` under the corresponding release.
- **GitHub Releases** — Significant contributors are acknowledged in the release description.
- **`good first issue` completions** — First-time contributors who close a `good first issue` are highlighted in the next release notes.

### Types of Valued Contributions

Beyond code, we recognize:

- 📝 **Documentation** — Writing or improving guides, references, and tutorials
- 🐛 **Bug Reports** — Detailed, reproducible bug reports with steps to reproduce
- 🧪 **Test Coverage** — Expanding the test suite to cover edge cases and integration scenarios
- 🔍 **Code Reviews** — Thorough, constructive PR reviews
- 💡 **Issue Triage** — Clarifying requirements, labeling issues, and closing duplicates
- 🌐 **Translations** — Adding or improving i18n locale files (`frontend/src/i18n/locales/`)

### How to Get Involved

- Browse [open issues](../../issues) and pick one tagged `good first issue` or `help wanted`
- Join discussions on existing issues — your perspective matters before a line of code is written
- Review open PRs — fresh eyes catch things authors miss

---

## 11. Agent & Contributor Guidelines

For AI coding agents and automated contributors, refer to [AGENTS.md](AGENTS.md) for strict architectural rules, toolchain conventions, and repository standards.

Key invariants for agents:
- Every on-chain mutation must call `require_auth()`.
- Use idempotent task state tracking with proper SQLite transactions.
- Wrap Venice AI calls in circuit breakers with exponential backoff.
- Never write hardcoded credentials, testnet private keys, or API tokens into source files.
- Follow [Frontend Architecture & Conventions](docs/FRONTEND_ARCHITECTURE.md) for all UI work.
