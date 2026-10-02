# Release Engineering Guide

This document is the canonical reference for cutting, signing, deploying, and
verifying releases of **ai-net** across its three tracks: backend (Node.js),
frontend (React/Vite), and smart contracts (Rust/Soroban on Stellar).

---

## Table of Contents

1. [Versioning Scheme](#1-versioning-scheme)
2. [Creating a Release Branch](#2-creating-a-release-branch)
3. [Running the Full Quality Gate Before Tagging](#3-running-the-full-quality-gate-before-tagging)
4. [Creating a Git Tag and GitHub Release](#4-creating-a-git-tag-and-github-release)
5. [Changelog Generation](#5-changelog-generation)
6. [Artifact Signing](#6-artifact-signing)
7. [Deploying Smart Contract Upgrades for a Release](#7-deploying-smart-contract-upgrades-for-a-release)
8. [Post-Release Verification Checklist](#8-post-release-verification-checklist)
9. [Hotfix Process](#9-hotfix-process)

---

## 1. Versioning Scheme

ai-net follows [Semantic Versioning 2.0.0](https://semver.org/) for all
release tracks. A single git tag covers backend, frontend, and smart contracts
together.

| Increment | When to use | Example |
|-----------|------------|---------|
| **MAJOR** (`X.0.0`) | Breaking API changes, contract storage-layout changes, incompatible protocol changes | `2.0.0` |
| **MINOR** (`x.Y.0`) | New backward-compatible features, new agent types, new API endpoints | `1.3.0` |
| **PATCH** (`x.y.Z`) | Bug fixes, documentation, dependency bumps — no interface changes | `1.2.4` |

### Pre-release labels

Append a pre-release identifier when publishing to testnet ahead of mainnet:

```
v1.3.0-rc.1     # release candidate
v1.3.0-beta.1   # public beta
v1.3.0-alpha.1  # early development preview
```

### Smart contract versioning

Contract artifacts carry the same SemVer as the repository tag. The version
string is embedded in each `deploy-<network>.json` manifest produced by
`smart-contracts/scripts/deploy.sh`:

```json
{
  "contract": "agent-registry",
  "version": "1.3.0",
  "network": "testnet",
  "contractId": "C...",
  "wasmHash": "abc123...",
  "deployedAt": "2026-09-29T14:00:00Z"
}
```

---

## 2. Creating a Release Branch

Every release — except [hotfixes](#9-hotfix-process) — originates from a
dedicated `release/vX.Y.Z` branch cut from `main`.

```bash
# 1. Ensure main is up-to-date
git checkout main
git pull origin main

# 2. Cut the release branch
VERSION="1.3.0"
git checkout -b "release/v${VERSION}"

# 3. Bump version numbers in all relevant manifests
#    (edit manually, then verify with grep)
#    Files to update:
#      - package.json           (root)
#      - backend/package.json
#      - frontend/package.json
#      - smart-contracts/package.json
#      - smart-contracts/Cargo.toml  (workspace version)

# Quick audit — all four should print the new version:
grep '"version"' package.json backend/package.json \
     frontend/package.json smart-contracts/package.json

grep '^version' smart-contracts/Cargo.toml

# 4. Commit the version bumps
git add package.json backend/package.json frontend/package.json \
        smart-contracts/package.json smart-contracts/Cargo.toml
git commit -m "chore(release): bump version to v${VERSION}"

# 5. Push and open a PR against main for review
git push -u origin "release/v${VERSION}"
gh pr create \
  --base main \
  --title "chore(release): release v${VERSION}" \
  --body "Release branch for v${VERSION}. See CHANGELOG.md for full notes."
```

> **Note:** Never push directly to `main`. The release branch PR goes through
> the normal review and CI gate before merging.

---

## 3. Running the Full Quality Gate Before Tagging

The full quality gate **must pass** before a tag is created. Run it locally
first, then confirm CI is green on the release branch.

### Local gate (single command)

```bash
# From the repository root:
npm run gate
```

This runs in sequence:

1. `prettier --check` — formatting across all TypeScript, JSON, Markdown, YAML
2. `eslint` — linting on backend and frontend
3. `tsc --noEmit` — type-check root, backend, and frontend
4. Jest/Vitest with coverage thresholds (≥75% backend, ≥70% frontend)

### Smart contract gate

```bash
cd smart-contracts

# Format check
cargo fmt --all -- --check

# Linter (zero warnings tolerated)
cargo clippy --locked \
  -p error-resolver -p error-registry -p agent-registry -p agent-bidding \
  --all-targets -- -D warnings

# Unit tests
cargo test --locked \
  -p error-resolver -p error-registry -p agent-registry -p agent-bidding

# Optimized Wasm builds
cargo build --locked -p agent-registry --target wasm32v1-none --release
cargo build --locked -p error-registry --target wasm32v1-none --release
cargo build --locked -p agent-bidding  --target wasm32v1-none --release

ls -la target/wasm32v1-none/release/*.wasm
```

### CI gate

The `ci-gate (required)` GitHub Actions job aggregates all pipeline results.
The release branch PR **must show a green `ci-gate` check** before tagging.

Required jobs that must succeed:

| Job | Description |
|-----|-------------|
| `backend` | Lint, type-check, build, unit tests + coverage |
| `frontend` | Lint, type-check, build, unit tests + coverage |
| `smart-contracts` | `cargo fmt`, `clippy`, `cargo test`, Wasm builds |
| `e2e-backend-smoke` | Registry/list/create smoke suite |
| `e2e-backend-heavy` | WS/lifecycle suite |
| `e2e-frontend-smoke` | Playwright registry + task submission |
| `e2e-frontend-heavy` | Playwright WS lifecycle, wallet, i18n |

---

## 4. Creating a Git Tag and GitHub Release

Once the release branch is merged to `main` and CI is green, tag and publish.

### Tag creation

Always use **annotated tags** so the tag message is stored in git history.

```bash
VERSION="1.3.0"

# Pull the freshly merged main
git checkout main
git pull origin main

# Create an annotated tag
git tag -a "v${VERSION}" -m "Release v${VERSION}

$(sed -n "/^## \[${VERSION}\]/,/^## \[/p" CHANGELOG.md | head -n -1)"

# Push the tag (immutable once pushed — double-check before this step)
git push origin "v${VERSION}"
```

### GitHub Release

Use the `gh` CLI to create the release and attach artifacts:

```bash
VERSION="1.3.0"
WASM_DIR="smart-contracts/target/wasm32v1-none/release"

# Collect Wasm artifacts and checksums
ARTIFACTS=(
  "${WASM_DIR}/agent_registry.wasm"
  "${WASM_DIR}/error_registry.wasm"
  "${WASM_DIR}/agent_bidding.wasm"
  "SHA256SUMS"
  "SHA256SUMS.asc"   # GPG signature — see Section 6
)

# Extract changelog section for this version as release notes
NOTES=$(sed -n "/^## \[${VERSION}\]/,/^## \[/p" CHANGELOG.md | head -n -1)

gh release create "v${VERSION}" \
  --title "v${VERSION}" \
  --notes "${NOTES}" \
  "${ARTIFACTS[@]}"
```

Tags and releases are **immutable**. If a release needs correction, create a
new patch version (`v1.3.1`) rather than deleting and recreating the tag.

---

## 5. Changelog Generation

ai-net uses [Conventional Commits](https://www.conventionalcommits.org/) to
drive automated changelog generation. Each commit type maps to a changelog
section:

| Commit prefix | Changelog section |
|--------------|------------------|
| `feat:` | **Features** |
| `fix:` | **Bug Fixes** |
| `perf:` | **Performance** |
| `feat!:` / `BREAKING CHANGE:` | **⚠ Breaking Changes** |
| `docs:`, `chore:`, `refactor:`, `test:`, `ci:` | **Other Changes** |

### Automated generation script

The following script generates the CHANGELOG entry for a new version. Run it
from the repository root after the version bump commit but before tagging:

```bash
#!/usr/bin/env bash
# scripts/generate-changelog.sh
# Usage: ./scripts/generate-changelog.sh <NEW_VERSION>
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
DATE=$(date +%Y-%m-%d)
RANGE="${LAST_TAG:+${LAST_TAG}..}HEAD"

section() {
  local label="$1"; shift
  local pattern="$1"; shift
  local entries
  entries=$(git log ${RANGE} --pretty=format:"- %s (%h)" --no-merges \
            | grep -E "^- ${pattern}" || true)
  if [[ -n "${entries}" ]]; then
    echo "### ${label}"
    echo ""
    echo "${entries}"
    echo ""
  fi
}

{
  echo "## [${VERSION}] - ${DATE}"
  echo ""
  section "⚠ Breaking Changes"  "(feat!|fix!|refactor!|BREAKING CHANGE)"
  section "Features"             "feat(\(.+\))?:"
  section "Bug Fixes"            "fix(\(.+\))?:"
  section "Performance"          "perf(\(.+\))?:"
  section "Other Changes"        "(docs|chore|refactor|test|ci)(\(.+\))?:"
} > /tmp/new-changelog-entry.md

# Prepend the new entry into CHANGELOG.md (after the header)
if [[ -f CHANGELOG.md ]]; then
  # Insert after line 1 (the '# Changelog' header)
  { head -n 1 CHANGELOG.md; echo ""; cat /tmp/new-changelog-entry.md; tail -n +2 CHANGELOG.md; } \
    > /tmp/CHANGELOG.md.tmp && mv /tmp/CHANGELOG.md.tmp CHANGELOG.md
else
  { echo "# Changelog"; echo ""; cat /tmp/new-changelog-entry.md; } > CHANGELOG.md
fi

echo "✅ CHANGELOG.md updated for v${VERSION}"
echo "   Review the entry, commit it, then tag."
```

Make the script executable once and commit it:

```bash
chmod +x scripts/generate-changelog.sh
git add scripts/generate-changelog.sh
git commit -m "chore: add changelog generation script"
```

Usage in the release flow:

```bash
./scripts/generate-changelog.sh 1.3.0
# Review and edit CHANGELOG.md as needed
git add CHANGELOG.md
git commit -m "chore(release): update CHANGELOG for v1.3.0"
```

### CI automation

The `release.yml` workflow (`.github/workflows/release.yml`) automatically
triggers on tag pushes matching `v*.*.*` and calls the changelog script, then
creates a draft GitHub Release with the generated notes:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'
      - 'v[0-9]+.[0-9]+.[0-9]+-rc.[0-9]+'

permissions:
  contents: write

jobs:
  release:
    name: Create GitHub Release
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - name: Checkout repository
        uses: actions/checkout@v5
        with:
          fetch-depth: 0          # full history for changelog generation

      - name: Set up Node.js
        uses: actions/setup-node@v5
        with:
          node-version: 24

      - name: Set up Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32v1-none

      - name: Extract version from tag
        id: version
        run: echo "VERSION=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - name: Generate changelog entry
        run: |
          chmod +x scripts/generate-changelog.sh
          ./scripts/generate-changelog.sh "${{ steps.version.outputs.VERSION }}"

      - name: Build optimized Wasm artifacts
        working-directory: smart-contracts
        run: |
          cargo build --locked -p agent-registry --target wasm32v1-none --release
          cargo build --locked -p error-registry --target wasm32v1-none --release
          cargo build --locked -p agent-bidding  --target wasm32v1-none --release

      - name: Generate SHA256SUMS
        working-directory: smart-contracts/target/wasm32v1-none/release
        run: |
          sha256sum agent_registry.wasm error_registry.wasm agent_bidding.wasm \
            > ../../../../SHA256SUMS
          cat ../../../../SHA256SUMS

      - name: Import GPG key
        uses: crazy-max/ghaction-import-gpg@v6
        with:
          gpg_private_key: ${{ secrets.GPG_PRIVATE_KEY }}
          passphrase: ${{ secrets.GPG_PASSPHRASE }}

      - name: Sign SHA256SUMS
        run: |
          gpg --batch --yes --detach-sign --armor \
            --output SHA256SUMS.asc SHA256SUMS

      - name: Extract release notes from CHANGELOG
        id: notes
        run: |
          VERSION="${{ steps.version.outputs.VERSION }}"
          NOTES=$(sed -n "/^## \[${VERSION}\]/,/^## \[/p" CHANGELOG.md | head -n -1)
          echo "NOTES<<EOF" >> "$GITHUB_OUTPUT"
          echo "${NOTES}" >> "$GITHUB_OUTPUT"
          echo "EOF" >> "$GITHUB_OUTPUT"

      - name: Create GitHub Release (draft)
        uses: softprops/action-gh-release@v2
        with:
          draft: true
          name: "v${{ steps.version.outputs.VERSION }}"
          body: ${{ steps.notes.outputs.NOTES }}
          files: |
            smart-contracts/target/wasm32v1-none/release/agent_registry.wasm
            smart-contracts/target/wasm32v1-none/release/error_registry.wasm
            smart-contracts/target/wasm32v1-none/release/agent_bidding.wasm
            SHA256SUMS
            SHA256SUMS.asc
```

> **Draft releases:** The workflow creates a *draft* release. A maintainer
> reviews the attached artifacts and release notes, then publishes it manually.
> This prevents accidental publication if a build artifact is corrupt.

---

## 6. Artifact Signing

All release artifacts are integrity-checked with SHA256 checksums and
optionally signed with a GPG key held by a designated release manager.

### Generating checksums

```bash
cd smart-contracts/target/wasm32v1-none/release

# Create SHA256SUMS for all Wasm artifacts
sha256sum agent_registry.wasm error_registry.wasm agent_bidding.wasm \
  > ../../../../SHA256SUMS

# Verify the file looks correct before signing
cat ../../../../SHA256SUMS
# Expected output format:
# abc123...  agent_registry.wasm
# def456...  error_registry.wasm
# 789ghi...  agent_bidding.wasm
```

### GPG signing

```bash
# Sign SHA256SUMS with a detached ASCII-armored signature
gpg --batch --yes \
    --detach-sign --armor \
    --output SHA256SUMS.asc \
    SHA256SUMS

# Verify the signature before uploading
gpg --verify SHA256SUMS.asc SHA256SUMS
# Expected: "Good signature from <release-manager-key>"
```

The signing key fingerprint must be published in the repository's
`docs/SECURITY.md` (or equivalent) so that downstream consumers can import it:

```bash
# Consumers: import and verify
gpg --keyserver hkps://keys.openpgp.org --recv-keys <FINGERPRINT>
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
```

### Verifying a downloaded release artifact

```bash
VERSION="1.3.0"

# 1. Download artifacts and signatures from the GitHub Release
gh release download "v${VERSION}" \
  --pattern "*.wasm" \
  --pattern "SHA256SUMS" \
  --pattern "SHA256SUMS.asc"

# 2. Import the release manager's key (one-time)
gpg --keyserver hkps://keys.openpgp.org --recv-keys <FINGERPRINT>

# 3. Verify GPG signature
gpg --verify SHA256SUMS.asc SHA256SUMS

# 4. Verify file integrity
sha256sum -c SHA256SUMS

# 5. Optionally inspect the Wasm module
stellar contract inspect --wasm agent_registry.wasm
```

### CI secrets

The CI workflow requires two repository secrets for signing:

| Secret | Description |
|--------|-------------|
| `GPG_PRIVATE_KEY` | ASCII-armored GPG private key (`gpg --armor --export-secret-keys <FP>`) |
| `GPG_PASSPHRASE` | Passphrase for the GPG key |

These must be configured in **Settings → Secrets and variables → Actions** for
the repository.

---

## 7. Deploying Smart Contract Upgrades for a Release

Smart contract upgrades follow a strict testnet → mainnet sequence. Never
deploy directly to mainnet without a passing testnet verification run.

### Prerequisites

```bash
cd smart-contracts
cp .env.example .env
# Required variables:
#   STELLAR_SECRET_KEY     — deployer/upgrade-manager keypair
#   VENICE_API_KEY         — Venice AI key for integration tests
#   SOROBAN_RPC_URL        — network RPC endpoint
#   NETWORK_PASSPHRASE     — Stellar network passphrase
```

### Step 1 — Build Wasm artifacts

```bash
cd smart-contracts

cargo build --locked -p agent-registry --target wasm32v1-none --release
cargo build --locked -p error-registry --target wasm32v1-none --release
cargo build --locked -p agent-bidding  --target wasm32v1-none --release
```

### Step 2 — Deploy to testnet (first)

```bash
# Deploy all contracts to testnet
./scripts/deploy.sh --network testnet

# Verify deployment manifests and on-chain state
./scripts/verify.sh --network testnet
```

### Step 3 — Run E2E tests on testnet

```bash
# Full market-report pipeline against testnet (60-120 s)
cp .env.example .env
# Set RUN_STELLAR_E2E_TESTS=true

npm run test:e2e
```

All tests must pass before proceeding to mainnet.

### Step 4 — Upgrade via upgrade-manager (testnet)

```bash
# Dry run first — shows what would change without executing
./scripts/upgrade.sh --network testnet --dry-run

# Upgrade a specific contract through the upgrade-manager
./scripts/upgrade.sh --network testnet --use-upgrade-manager agent-registry

# Upgrade all contracts
./scripts/upgrade.sh --network testnet --use-upgrade-manager

# Post-upgrade verification
./scripts/verify.sh --network testnet
```

### Step 5 — Announce mainnet upgrade

Notify stakeholders **at least 48 hours** before the mainnet upgrade window.
Include:

- Contracts being upgraded
- Version (`old → new`)
- Estimated maintenance window
- Rollback plan (48-hour rollback window is built into the upgrade-manager)

### Step 6 — Upgrade mainnet

```bash
# Dry run against mainnet
./scripts/upgrade.sh --network mainnet --dry-run

# Upgrade with upgrade-manager
./scripts/upgrade.sh --network mainnet --use-upgrade-manager agent-registry

# Verify mainnet state
./scripts/verify.sh --network mainnet
```

### Step 7 — Commit deployment manifests

```bash
git add smart-contracts/deployments/
git commit -m "chore(contracts): update deployment manifests for v${VERSION}"
git push origin main
```

### Upgrade system capabilities

The upgrade system (`upgrade.sh` + upgrade-manager contract) provides:

- ✅ Pre/post migration hooks
- ✅ Version compatibility checks
- ✅ 48-hour rollback window for emergency recovery
- ✅ Gas estimation for migration operations
- ✅ On-chain event tracking for upgrade monitoring

For in-depth upgrade procedures and troubleshooting, see
[smart-contracts/docs/UPGRADE_GUIDE.md](../smart-contracts/docs/UPGRADE_GUIDE.md)
and
[smart-contracts/docs/STORAGE_MIGRATION.md](../smart-contracts/docs/STORAGE_MIGRATION.md).

---

## 8. Post-Release Verification Checklist

After publishing the GitHub Release and completing contract upgrades, work
through this checklist. Each item must be confirmed by a named maintainer.

### GitHub Release

- [ ] Tag pushed to `origin` and visible on GitHub
- [ ] GitHub Release published (not draft)
- [ ] Release notes match `CHANGELOG.md` for this version
- [ ] Wasm artifacts (`*.wasm`) attached to the release
- [ ] `SHA256SUMS` and `SHA256SUMS.asc` attached to the release
- [ ] GPG signature verified locally: `gpg --verify SHA256SUMS.asc SHA256SUMS`

### Backend & Frontend

- [ ] Docker images tagged and pushed: `ghcr.io/epta-node/ai-net-backend:v${VERSION}`
- [ ] Docker images tagged and pushed: `ghcr.io/epta-node/ai-net-frontend:v${VERSION}`
- [ ] Production deployment completed (if applicable)
- [ ] Health endpoint returns expected version: `curl https://<host>/health`
- [ ] No new error spikes in application logs within 30 minutes of deploy

### Smart Contracts (Testnet)

- [ ] `./scripts/verify.sh --network testnet` exits 0
- [ ] On-chain deployment manifest (`deploy-testnet.json`) committed to `main`
- [ ] E2E test suite passes against new testnet contracts

### Smart Contracts (Mainnet)

- [ ] `./scripts/verify.sh --network mainnet` exits 0
- [ ] On-chain deployment manifest (`deploy-mainnet.json`) committed to `main`
- [ ] Stakeholders notified of completed upgrade
- [ ] Contract events monitored for 24 hours post-upgrade
- [ ] Rollback window (48 h) noted: expires at `<timestamp + 48h>`

### Documentation

- [ ] `CHANGELOG.md` updated and committed on `main`
- [ ] `README.md` version badge updated if applicable
- [ ] Any new APIs or breaking changes reflected in `docs/API_REFERENCE.md`

---

## 9. Hotfix Process

A hotfix addresses a critical production defect that cannot wait for the next
regular release cycle. It branches from the published tag — **not from main** —
so that only the targeted fix ships.

### When to use a hotfix

- Security vulnerabilities in production contracts or backend
- Data-loss or fund-at-risk bugs
- CI-breaking regressions in a deployed version

For non-critical fixes (cosmetic issues, documentation gaps, minor performance
regressions), use the normal release flow.

### Hotfix workflow

```bash
# 1. Branch from the last release tag (not main)
VERSION="1.2.3"          # current production version
HOTFIX_VERSION="1.2.4"   # patch increment

git checkout "v${VERSION}"
git checkout -b "hotfix/v${HOTFIX_VERSION}"

# 2. Apply the minimal targeted fix — one commit where possible
#    Then verify it locally:
npm run gate              # full quality gate
cd smart-contracts && cargo test --locked \
  -p error-resolver -p error-registry -p agent-registry -p agent-bidding

# 3. Bump the patch version
#    Edit package.json files and Cargo.toml as in Section 2
git add package.json backend/package.json frontend/package.json \
        smart-contracts/package.json smart-contracts/Cargo.toml
git commit -m "chore(release): bump version to v${HOTFIX_VERSION}"

# 4. Update CHANGELOG
./scripts/generate-changelog.sh "${HOTFIX_VERSION}"
git add CHANGELOG.md
git commit -m "chore(release): update CHANGELOG for v${HOTFIX_VERSION}"

# 5. Push and open a PR — target main for review
git push -u origin "hotfix/v${HOTFIX_VERSION}"
gh pr create \
  --base main \
  --title "fix: hotfix v${HOTFIX_VERSION} — <brief description>" \
  --body "**Hotfix** for <issue description>.

Fixes #<issue-number>

Changes:
- <what was fixed and why>

This PR targets \`main\` to keep history linear. Tag will be applied after merge."

# 6. After the PR is reviewed and CI passes, merge to main
# 7. Tag the hotfix release from main
git checkout main
git pull origin main

git tag -a "v${HOTFIX_VERSION}" -m "Hotfix v${HOTFIX_VERSION}: <brief description>"
git push origin "v${HOTFIX_VERSION}"

# 8. Create GitHub Release for the hotfix
gh release create "v${HOTFIX_VERSION}" \
  --title "v${HOTFIX_VERSION} (hotfix)" \
  --notes "$(sed -n "/^## \[${HOTFIX_VERSION}\]/,/^## \[/p" CHANGELOG.md | head -n -1)"
```

### Hotfix contract upgrades

If the hotfix involves a smart contract change, follow the full
[Section 7](#7-deploying-smart-contract-upgrades-for-a-release) upgrade sequence.
Do not abbreviate testnet verification for hotfixes — a broken mainnet contract
upgrade is harder to recover from than the original bug.

### Hotfix vs regular release — key differences

| Aspect | Regular release | Hotfix |
|--------|----------------|--------|
| Branch base | `main` | Last production tag (`vX.Y.Z`) |
| Branch prefix | `release/vX.Y.Z` | `hotfix/vX.Y.Z` |
| Version increment | Major, minor, or patch | Patch only |
| Review SLO | 24–48 h standard | 2–4 h expedited |
| Post-merge target | `main` | `main` |
| Stakeholder notice | Standard (48 h for contracts) | Immediate (ASAP after discovery) |

---

## Release Tracks Summary

| Track | Cadence | Artifacts |
|-------|---------|-----------|
| **Smart Contracts** | On-demand (audit milestone or critical fix) | `.wasm` files, `SHA256SUMS`, `SHA256SUMS.asc`, deployment manifests |
| **Backend** | Bi-weekly or on critical fix | Docker image, deployment manifest |
| **Frontend** | Bi-weekly or on critical fix | Static build, Docker image |

All tracks share the same SemVer tag. A single `vX.Y.Z` git tag covers the
entire monorepo.

---

*For questions about this process, open an issue or ping `#releases` in the
project Discord.*
