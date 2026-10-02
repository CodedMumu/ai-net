# 🛠️ Developer Setup Guide

Welcome to **ai-net**! This guide takes you from a clean machine to a fully running local development environment in under 30 minutes. Every command is copy-paste ready.

> **Tested on**: Ubuntu 22.04 LTS · macOS 14 (Apple Silicon) · Windows 11 with WSL2

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Clone and Install](#2-clone-and-install)
3. [Environment Variables](#3-environment-variables)
4. [Start the Local Stellar Standalone Node](#4-start-the-local-stellar-standalone-node)
5. [Fund Accounts with Friendbot](#5-fund-accounts-with-friendbot)
6. [Deploy Contracts to Local Standalone](#6-deploy-contracts-to-local-standalone)
7. [Run the Backend and Frontend Locally](#7-run-the-backend-and-frontend-locally)
8. [Run All Tests](#8-run-all-tests)
9. [Freighter Wallet Setup for Local Testing](#9-freighter-wallet-setup-for-local-testing)
10. [Common Errors and Fixes](#10-common-errors-and-fixes)

---

## 1. Prerequisites

Install the following tools before cloning the repository. Each section below includes the recommended version and the fastest installation path per OS.

### Node.js ≥ 20 LTS

The backend and frontend both require Node.js 20 or 22.

```bash
# Install via nvm (recommended — works on Linux, macOS, WSL2)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.bashrc   # or ~/.zshrc on macOS/zsh
nvm install 20
nvm use 20
node --version     # should print v20.x.x
```

macOS alternative via Homebrew:
```bash
brew install node@20
```

Windows (WSL2): follow the Linux instructions above inside your WSL2 Ubuntu terminal. Do **not** use native Windows Node.js.

### Docker and Docker Compose

Docker runs the full stack (Stellar standalone node, backend, frontend) via a single command.

- **Linux**: follow the [official Docker Engine install](https://docs.docker.com/engine/install/ubuntu/) then add your user to the `docker` group:
  ```bash
  sudo usermod -aG docker $USER && newgrp docker
  ```
- **macOS**: install [Docker Desktop](https://www.docker.com/products/docker-desktop/) or [OrbStack](https://orbstack.dev/) (faster on Apple Silicon).
- **Windows WSL2**: install [Docker Desktop for Windows](https://docs.docker.com/desktop/install/windows-install/) and enable the WSL2 integration in Docker Desktop settings.

Verify:
```bash
docker --version          # Docker version 24.x.x or later
docker compose version    # Docker Compose version v2.x.x or later
```

### Rust and Cargo (for smart contract development)

Required only if you plan to build or modify the Soroban smart contracts.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup target add wasm32-unknown-unknown
cargo --version            # cargo 1.80.0 or later
```

### Stellar CLI (for contract deployment)

Required only for manual contract deployment steps.

```bash
cargo install --locked stellar-cli --features opt
stellar --version          # stellar 21.x.x or later
```

---

## 2. Clone and Install

```bash
# Clone the repository
git clone https://github.com/CodedSceptre/ai-net.git
cd ai-net

# Install all dependencies (root workspace + backend + frontend)
npm install
```

`npm install` at the root installs workspaces for `backend/`, `frontend/`, and shared tooling. You do not need to `cd` into each subdirectory separately.

Verify the install:
```bash
ls backend/node_modules frontend/node_modules   # both directories should exist
```

---

## 3. Environment Variables

The project ships an `.env.example` at the root. Copy it before starting any service:

```bash
cp .env.example .env
```

Open `.env` in your editor. Below is a complete explanation of every variable grouped by concern.

### Runtime and Server

```ini
# Application mode. Use "development" locally, "production" for deploys.
NODE_ENV=development

# HTTP port the backend API listens on.
PORT=3001

# Log verbosity: "trace" | "debug" | "info" | "warn" | "error" | "fatal"
LOG_LEVEL=info

# Application version tag included in /health responses.
NPM_PACKAGE_VERSION=0.1.0

# Seconds to wait for in-flight requests to drain on SIGTERM.
GRACEFUL_SHUTDOWN_TIMEOUT=30
```

### Database

```ini
# SQLite connection string. Path is relative to the backend/ directory.
# Three separate databases are created automatically on first start:
#   backend/data/payments.db  — payment ledger
#   backend/data/agents.db    — agent registry cache
#   backend/data/tasks.db     — task state machine
DATABASE_URL=./data/ai-net.db
```

You never need to create these files manually — they are created and migrated automatically when the backend starts.

### CORS and API Security

```ini
# Comma-separated list of allowed CORS origins. Add your frontend URL here.
ALLOWED_ORIGINS=http://localhost:3000

# Optional comma-separated static API keys for machine-to-machine callers.
# Leave blank for local development.
API_KEYS=

# Optional secret for admin-only endpoints.
ADMIN_API_KEY=
```

### Stellar Network

```ini
# Which Stellar network to target: "standalone" | "testnet" | "mainnet"
STELLAR_NETWORK=testnet

# Horizon REST API base URL.
STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org

# Legacy alias for STELLAR_HORIZON_URL (kept for older scripts).
STELLAR_HORIZON=

# Soroban JSON-RPC endpoint used for contract calls.
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
```

**Local standalone values** (used when running `docker compose up`):
```ini
STELLAR_NETWORK=standalone
STELLAR_HORIZON_URL=http://localhost:8000
SOROBAN_RPC_URL=http://localhost:8000/soroban/rpc
```

### Stellar Signing Keys

> ⚠️ **Never commit real private keys to git.** The `.gitignore` excludes `.env`, but always double-check before committing.

```ini
# The coordinator agent's Ed25519 secret key (starts with S...).
# Generate one: stellar keys generate coordinator --network standalone
STELLAR_COORDINATOR_SECRET=

# Derived public key (starts with G...). Can be left blank — the backend derives
# it automatically from STELLAR_COORDINATOR_SECRET if not set.
STELLAR_COORDINATOR_PUBLIC_KEY=

# Alternative secret key variable used by some test scripts.
STELLAR_TEST_SECRET=

# General-purpose secret key used by smart-contract deploy scripts.
STELLAR_SECRET_KEY=

# Corresponding public key.
STELLAR_PUBLIC_KEY=

# Contract ID of the deployed AgentRegistry Soroban contract (C...).
# Populated after running the deploy script.
REGISTRY_CONTRACT_ID=

# Optional JSON map of all deployed contract IDs, e.g.:
# {"agent_registry":"C...","task_store":"C..."}
DEPLOYED_CONTRACT_IDS=

# Set to true to skip live Stellar account existence checks in tests/dev.
# Useful when running without a funded account.
SKIP_STELLAR_ACCOUNT_VERIFY=false
```

### Venice AI

```ini
# Your Venice AI API key. Get one at https://venice.ai
VENICE_API_KEY=your_venice_api_key_here

# Venice AI base URL. Do not change unless using a private deployment.
VENICE_BASE_URL=https://api.venice.ai/api/v1

# Model API version tag forwarded in requests.
VENICE_MODEL_VERSION=v1

# How long to cache Venice responses (milliseconds).
# 86400000 = 24 hours (default for most responses)
VENICE_CACHE_TTL_MS=86400000

# Shorter TTL for coding-agent responses (1 hour).
VENICE_CACHE_CODING_TTL_MS=3600000

# Cosine similarity threshold (0–1) for cache hit matching against stored prompts.
VENICE_CACHE_SIMILARITY_THRESHOLD=0.8
```

For local development without a Venice API key, the backend falls back to mock responses. Integration tests that actually call Venice require `RUN_VENICE_INTEGRATION_TESTS=true`.

### Cache

```ini
# Cache driver: "lru" (in-process, default) or "redis" (external Redis).
# Use "lru" for local development — no Redis needed.
CACHE_DRIVER=lru

# Redis connection string. Only used when CACHE_DRIVER=redis.
REDIS_URL=redis://localhost:6379

# Maximum number of entries for the LRU in-process cache.
CACHE_LRU_MAX_SIZE=500

# Per-route cache TTLs in seconds.
CACHE_TTL_AGENTS=60    # /agents list
CACHE_TTL_STATS=30     # /stats endpoint
CACHE_TTL_HEALTH=10    # /health endpoint
```

### Rate Limiting and Quotas

```ini
# Maximum prompt characters accepted per request.
MAX_PROMPT_LENGTH=10000

# Maximum tasks a single Stellar wallet address can submit per 24-hour window.
DAILY_TASK_LIMIT_PER_WALLET=100

# Rolling window for general rate limiting (milliseconds). Default: 60 seconds.
RATE_LIMIT_WINDOW_MS=60000

# Max requests per window per IP for general endpoints.
RATE_LIMIT_MAX_REQUESTS=20

# Stricter limit for agent registration endpoint.
REGISTER_RATE_LIMIT_MAX_REQUESTS=10
```

### Agent Heartbeat

```ini
# How often agents should send a heartbeat ping (milliseconds). Default: 5 minutes.
HEARTBEAT_INTERVAL_MS=300000

# Minutes of silence before an agent is marked offline.
HEARTBEAT_STALE_THRESHOLD_MINUTES=5

# Hours of being offline before an agent record is auto-deleted.
AGENT_OFFLINE_DELETE_HOURS=24
```

### Payment Reconciliation

```ini
# Optional webhook URL to POST reconciliation summaries to.
RECONCILIATION_WEBHOOK_URL=

# How often to run the payment reconciliation job (milliseconds). Default: 24 hours.
RECONCILIATION_INTERVAL_MS=86400000
```

### Compression

```ini
# Minimum response size in bytes before compression is applied.
COMPRESSION_THRESHOLD=1024

# zlib compression level (1–9). 6 is a good balance of speed vs. ratio.
COMPRESSION_LEVEL=6

# Enable Brotli compression in addition to gzip.
COMPRESSION_ENABLE_BROTLI=true
```

### API Versioning

```ini
# Current highest API version served.
API_LATEST_VERSION=2.0

# Comma-separated list of all supported versions.
API_SUPPORTED_VERSIONS=1.0,1.1,2.0

# Version returned when no Accept-Version header is provided.
API_DEFAULT_VERSION=1.0

# Optional RFC 7231 date after which v1 is declared sunset in response headers.
API_V1_SUNSET_DATE=
```

### WebSocket

```ini
# Maximum concurrent WebSocket connections per client IP.
WS_MAX_CONNECTIONS_PER_CLIENT=5

# Maximum messages per minute per connection before throttling.
WS_MAX_MESSAGES_PER_MINUTE=100

# Milliseconds of inactivity before closing an idle connection (30 minutes).
WS_INACTIVITY_TIMEOUT_MS=1800000

# Server-to-client ping interval in milliseconds.
WS_HEARTBEAT_INTERVAL_MS=30000

# Milliseconds to wait for a pong reply before closing the connection.
WS_PONG_TIMEOUT_MS=10000
```

### Metrics

```ini
# How long to cache health-dashboard metrics (milliseconds).
METRICS_CACHE_TTL_MS=5000

# Rolling window for per-second metrics aggregation (milliseconds).
METRICS_WINDOW_MS=60000

# Maximum metric sample buffer size.
METRICS_MAX_SAMPLES=1000
```

### Frontend (Vite)

```ini
# The backend URL that the browser-side code calls. Must be publicly reachable
# from the browser, so use localhost when developing locally.
VITE_API_BASE_URL=http://localhost:3001
```

### Integration Test Flags

These flags are all `false` by default. Flip them to `true` only when you want to run live external-network tests.

```ini
# Run the full Stellar E2E market-report pipeline against testnet.
# Requires funded testnet accounts and takes 60–120 seconds.
RUN_STELLAR_E2E_TESTS=false

# Run Stellar integration tests (contract calls against a real node).
RUN_STELLAR_INTEGRATION_TESTS=false

# Run Venice AI integration tests (real API calls, may incur cost).
RUN_VENICE_INTEGRATION_TESTS=false

# Reuse already-deployed contracts across integration test runs.
RUN_SHARED_DEPLOY=false
```

---

## 4. Start the Local Stellar Standalone Node

Docker Compose brings up three services: a Stellar standalone node, the backend API, and the frontend. For local development you can start all of them at once or bring up only the Stellar node.

### Option A — Full stack (recommended for first-time setup)

```bash
docker compose up -d
```

Services started:
| Service | Container | Exposed Port | URL |
|---|---|---|---|
| Stellar standalone node | `ai-net-stellar-standalone` | 8000 | http://localhost:8000 |
| Backend API | `ai-net-backend` | 3000 | http://localhost:3000 |
| Frontend | `ai-net-frontend` | 5173 | http://localhost:5173 |

Wait for all services to be healthy:
```bash
docker compose ps
```

All three should show `healthy` (or `running`) in the STATUS column. This typically takes 20–40 seconds on first run while Docker pulls images.

Verify the Stellar node is up:
```bash
curl http://localhost:8000/
# Should return a JSON object with "network_passphrase" and other fields
```

Verify the backend:
```bash
curl http://localhost:3000/health
# {"status":"ok","version":"0.1.0",...}
```

### Option B — Stellar node only (for manual backend/frontend development)

```bash
docker compose up -d stellar-standalone
```

Then run the backend and frontend manually as described in [Section 7](#7-run-the-backend-and-frontend-locally).

When using the standalone node locally, update your `.env` to point at it:
```ini
STELLAR_NETWORK=standalone
STELLAR_HORIZON_URL=http://localhost:8000
SOROBAN_RPC_URL=http://localhost:8000/soroban/rpc
SKIP_STELLAR_ACCOUNT_VERIFY=true
```

### Stopping and resetting

```bash
# Stop all services
docker compose down

# Stop and wipe all volumes (resets the chain state and SQLite data)
docker compose down -v
```

---

## 5. Fund Accounts with Friendbot

The Stellar standalone node ships with a built-in Friendbot faucet that mints 10,000 XLM to any new address. You need a funded account before deploying contracts or running agent workflows.

### Generate a keypair

```bash
stellar keys generate coordinator --network standalone
stellar keys address coordinator   # prints the G... public key
stellar keys show coordinator      # prints the S... secret key (keep it safe)
```

Store the secret key in your `.env`:
```ini
STELLAR_COORDINATOR_SECRET=S...your-secret-here...
STELLAR_COORDINATOR_PUBLIC_KEY=G...your-public-key-here...
```

### Fund via Friendbot (standalone node)

```bash
COORDINATOR_PUB=$(stellar keys address coordinator)

# Using the standalone Friendbot endpoint
curl "http://localhost:8000/friendbot?addr=${COORDINATOR_PUB}"
# {"hash":"...","result":"..."}
```

Verify the balance:
```bash
curl "http://localhost:8000/accounts/${COORDINATOR_PUB}" | python3 -m json.tool | grep balance
# "balance": "10000.0000000"
```

### Fund via Friendbot (testnet)

When targeting the public Stellar testnet instead of the local node:
```bash
COORDINATOR_PUB=$(stellar keys address coordinator)
curl "https://friendbot.stellar.org?addr=${COORDINATOR_PUB}"
```

---

## 6. Deploy Contracts to Local Standalone

The smart contracts live in `smart-contracts/`. Deployment scripts target the `standalone` network by default when a local Stellar node is running.

### Set up smart-contract environment

```bash
cd smart-contracts
cp .env.example .env
```

Edit `smart-contracts/.env`:
```ini
STELLAR_NETWORK=standalone
STELLAR_HORIZON_URL=http://localhost:8000
SOROBAN_RPC_URL=http://localhost:8000/soroban/rpc
STELLAR_SECRET_KEY=S...your-coordinator-secret...
STELLAR_COORDINATOR_SECRET=S...your-coordinator-secret...
```

### Build the contracts

```bash
# From smart-contracts/
cargo build --target wasm32-unknown-unknown --release
```

Compiled `.wasm` files are placed in `target/wasm32-unknown-unknown/release/`.

### Deploy all contracts

```bash
./scripts/deploy.sh --network standalone
```

The script prints each contract ID as it deploys. Copy the `REGISTRY_CONTRACT_ID` value back into your root `.env`:
```ini
REGISTRY_CONTRACT_ID=C...the-contract-id-printed-by-the-script...
```

### Verify deployment

```bash
./scripts/verify.sh --network standalone
```

Expected output: `✅ All contracts verified.`

Return to the repo root when done:
```bash
cd ..
```

---

## 7. Run the Backend and Frontend Locally

Use this approach when you want fast hot-reload iteration without rebuilding Docker images.

### Prerequisites for manual dev mode

- Stellar standalone node running (via `docker compose up -d stellar-standalone`)
- `.env` populated (see [Section 3](#3-environment-variables))
- `npm install` completed (see [Section 2](#2-clone-and-install))

### Run database migrations

Migrations run automatically on server start, but you can also trigger them manually:

```bash
cd backend
npm run db:migrate
cd ..
```

This applies any pending migrations to all three SQLite databases (`payments.db`, `agents.db`, `tasks.db`) under `backend/data/`.

To seed local development data (sample agents and tasks):
```bash
cd backend
npm run db:seed
cd ..
```

### Start the backend

```bash
cd backend
npm run dev
```

The backend starts on **http://localhost:3001**. Watch for:
```
{"level":"info","msg":"Server listening on port 3001"}
```

Health check:
```bash
curl http://localhost:3001/health
# {"status":"ok","uptime":...}
```

### Start the frontend

Open a second terminal:

```bash
cd frontend
npm run dev
```

Vite starts on **http://localhost:5173**. Open that URL in your browser.

The frontend reads `VITE_API_BASE_URL` from `.env` (or `.env.local`). Make sure it points to the backend:
```ini
VITE_API_BASE_URL=http://localhost:3001
```

---

## 8. Run All Tests

### Full-stack quality gate (run this before every PR)

From the repo root:
```bash
npm run gate
```

This runs formatting checks, linting, type-checking, and coverage thresholds across all packages in one command.

### Backend tests

```bash
cd backend
npm test                   # unit + integration tests
npm run test:coverage      # same with coverage report
```

Coverage report is written to `backend/coverage/`.

### Frontend tests

```bash
cd frontend
npm test                   # Vitest unit tests
npm run test:coverage      # with coverage
```

### Smart contract tests (Rust)

```bash
cd smart-contracts
cargo test
```

### End-to-end pipeline tests (testnet)

These tests fund fresh testnet accounts via Friendbot and execute the full five-agent market report pipeline. They require a Venice API key and take 60–120 seconds.

```bash
cd smart-contracts
cp .env.example .env
# Edit .env and set:
#   RUN_STELLAR_E2E_TESTS=true
#   STELLAR_COORDINATOR_SECRET=S...
#   VENICE_API_KEY=...

npm run test:e2e
```

---

## 9. Freighter Wallet Setup for Local Testing

[Freighter](https://www.freighter.app/) is the official Stellar browser extension wallet used to sign transactions in the frontend.

### Install Freighter

1. Install from the [Chrome Web Store](https://chrome.google.com/webstore/detail/freighter/bcacfldlkkdogcmkkibnjlakofdplcbk) or [Firefox Add-ons](https://addons.mozilla.org/en-US/firefox/addon/freighter/).
2. Follow the prompts to create a new wallet and save your recovery phrase.

### Connect to standalone (local) network

1. Open the Freighter extension and click the network selector at the top.
2. Select **Add Custom Network**.
3. Fill in:
   - **Network Name**: `Standalone Local`
   - **Horizon URL**: `http://localhost:8000`
   - **Soroban RPC URL**: `http://localhost:8000/soroban/rpc`
   - **Passphrase**: `Standalone Network ; February 2017`
   - **Allow HTTP**: ✅ (enable for localhost)
4. Click **Add Network** and then select it.

### Fund your Freighter address

Copy your Freighter public key (the `G...` address shown in the extension) and fund it:

```bash
curl "http://localhost:8000/friendbot?addr=<YOUR_FREIGHTER_G_ADDRESS>"
```

### Connect to testnet instead

1. In Freighter, select the network dropdown and choose **Testnet**.
2. Fund your address via the public Friendbot:
   ```bash
   curl "https://friendbot.stellar.org?addr=<YOUR_FREIGHTER_G_ADDRESS>"
   ```

Your wallet is now ready. Open the frontend at http://localhost:5173 and click **Connect Wallet** to sign in with Freighter.

---

## 10. Common Errors and Fixes

### `EACCES: permission denied` when running Docker commands

**Cause**: your user is not in the `docker` group.

```bash
sudo usermod -aG docker $USER
newgrp docker
# Or log out and back in
```

---

### `ENOSPC: System limit for number of file watchers reached`

**Cause**: Linux inotify watcher limit is too low for the project's file count.

```bash
echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

---

### `Error: connect ECONNREFUSED 127.0.0.1:8000` (Stellar node)

**Cause**: the Stellar standalone Docker container is not running or not yet healthy.

```bash
docker compose ps
docker compose logs stellar-standalone
# If stopped:
docker compose up -d stellar-standalone
```

---

### `error: package 'stellar-cli' not found` (Cargo install)

**Cause**: the `opt` feature requires a recent Cargo version.

```bash
rustup update stable
cargo install --locked stellar-cli --features opt
```

---

### `Cannot find module` errors after `npm install`

**Cause**: workspace symlinks broken, often after switching Node.js versions.

```bash
rm -rf node_modules backend/node_modules frontend/node_modules
npm install
```

---

### Backend crashes with `SQLITE_CANTOPEN`

**Cause**: the `backend/data/` directory does not exist.

```bash
mkdir -p backend/data
cd backend && npm run db:migrate
```

---

### Frontend shows `Failed to fetch` on API calls

**Cause**: `VITE_API_BASE_URL` is not set or points to the wrong port.

Check your `.env`:
```ini
VITE_API_BASE_URL=http://localhost:3001
```

Restart Vite after changing `.env`:
```bash
# Ctrl+C in the frontend terminal, then:
cd frontend && npm run dev
```

---

### `Transaction failed: insufficient balance` during contract deploy

**Cause**: the deployer account was not funded.

```bash
stellar keys address coordinator   # get the public key
curl "http://localhost:8000/friendbot?addr=<PUBLIC_KEY>"
```

---

### Freighter shows `Network mismatch` error

**Cause**: Freighter is set to Testnet but the app is pointing to the local standalone node (or vice versa).

Ensure Freighter's selected network matches the values in your `.env`:
- **Standalone**: use the custom `Standalone Local` network configured in [Section 9](#9-freighter-wallet-setup-for-local-testing).
- **Testnet**: use Freighter's built-in Testnet option.

---

### WSL2: `localhost` not reachable from Windows browser

**Cause**: WSL2 networking sometimes uses a different IP than `127.0.0.1`.

```bash
# Inside WSL2, find the IP:
ip addr show eth0 | grep 'inet '
# Use that IP in your Windows browser instead of localhost
```

Alternatively, enable WSL2 localhost forwarding in `.wslconfig`:
```ini
# %USERPROFILE%\.wslconfig
[wsl2]
localhostForwarding=true
```

---

### macOS: Docker port binding fails on ports < 1024

**Cause**: macOS restricts binding to privileged ports in some Docker setups. All ports used by ai-net (3001, 5173, 8000) are above 1024, so this should not apply. If you still see bind errors, restart Docker Desktop.

---

## Further Reading

- [Architecture Specification](architecture/index.md) — System design, component diagrams, and security model.
- [REST API Reference](API_REFERENCE.md) — Endpoint documentation with curl examples.
- [Smart Contract Deployment Guide](../smart-contracts/docs/DEPLOYMENT_GUIDE.md) — Full deployment and upgrade workflows.
- [AI-Agent Integration Guide](ai-agent-integration-guide.md) — Registering third-party agents, staking bonds, and dispute resolution.
- [CONTRIBUTING.md](../CONTRIBUTING.md) — Branching rules, Conventional Commits, and PR checklists.
