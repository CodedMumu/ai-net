# 🚀 Node Operators Guide

> Everything you need to provision, configure, deploy, and operate an **ai-net** node from scratch — with all commands copy-paste ready.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Hardware and OS Requirements](#2-hardware-and-os-requirements)
3. [Installing Dependencies](#3-installing-dependencies)
4. [Configuring Secrets and Environment Variables](#4-configuring-secrets-and-environment-variables)
5. [Deploying Smart Contracts to Testnet and Mainnet](#5-deploying-smart-contracts-to-testnet-and-mainnet)
6. [Funding Stellar Accounts](#6-funding-stellar-accounts)
7. [Starting and Stopping the Node](#7-starting-and-stopping-the-node)
8. [Health Check and Monitoring](#8-health-check-and-monitoring)
9. [Upgrading the Node to a New Version](#9-upgrading-the-node-to-a-new-version)
10. [Troubleshooting Common Errors](#10-troubleshooting-common-errors)

---

## 1. Overview

An **ai-net node** runs the backend API server that coordinates AI agents, manages tasks, and bridges off-chain inference with the Stellar blockchain. The stack consists of:

```
┌──────────────────────────────────────────────────────────┐
│                       ai-net Node                        │
│                                                          │
│   ┌────────────────┐   ┌────────────────┐   ┌─────────┐  │
│   │ Coordinator    │──►│ Payment Layer  │──►│ Stellar │  │
│   │ & Agent Daemon │   │ (Stellar SDK)  │   │ Network │  │
│   └────────────────┘   └────────────────┘   └─────────┘  │
│           │                     │                        │
│           ▼                     ▼                        │
│   ┌────────────────┐   ┌────────────────┐                │
│   │ SQLite DBs     │   │ Venice AI API  │                │
│   │ (agents/tasks/ │   │ (Inference)    │                │
│   │  payments)     │   │                │                │
│   └────────────────┘   └────────────────┘                │
└──────────────────────────────────────────────────────────┘
```

### Node Roles

- **Coordinator**: Decomposes user requests into task DAGs and orchestrates agent assignment.
- **Registry Sync**: Reads the on-chain `agent_registry` Soroban contract and caches state locally in SQLite.
- **Payment Relay**: Signs Stellar transactions to release escrow payments on task completion.
- **Agent Workers**: Runs specialized agent types (Research, Coding, Design, Risk, Report) via Venice AI.

---

## 2. Hardware and OS Requirements

### 2.1 Recommended Specifications

| Component | Testnet / Dev | Production / Mainnet |
|---|---|---|
| **CPU** | 2 vCPUs (x86_64 or ARM64) | 4+ vCPUs |
| **RAM** | 4 GB | 8 GB+ |
| **Disk** | 20 GB SSD | 80 GB+ NVMe SSD |
| **Network** | 10 Mbps outbound | 100 Mbps redundant |
| **OS** | Ubuntu 22.04/24.04 LTS · Debian 12 | Ubuntu 22.04/24.04 LTS · Debian 12 |

### 2.2 Supported Operating Systems

- Ubuntu 22.04 LTS (Jammy) — **recommended**
- Ubuntu 24.04 LTS (Noble)
- Debian 12 (Bookworm)
- macOS 14+ (local development only)

### 2.3 Firewall and Port Configuration

| Port | Protocol | Direction | Purpose |
|---|---|---|---|
| `3000` | TCP | Inbound | Backend REST API (reverse proxy target) |
| `5173` | TCP | Inbound | Frontend dashboard (optional; dev only) |
| `8000` | TCP | Inbound | Stellar standalone RPC (Docker dev only) |
| `9090` | TCP | Private | Prometheus metrics scrape endpoint |
| `443` | TCP | Inbound | HTTPS via Nginx/Cloudflare (production) |

```bash
# Ubuntu UFW example — allow only API port from reverse proxy
sudo ufw allow 22/tcp        # SSH
sudo ufw allow 443/tcp       # HTTPS (Nginx)
sudo ufw allow from 10.0.0.0/8 to any port 9090  # Prometheus (internal only)
sudo ufw enable
sudo ufw status
```

> ⚠️ Never expose port `9090` (Prometheus metrics) to the public internet. It contains operational details about your node.

---

## 3. Installing Dependencies

### 3.1 System Packages

```bash
# Update package index
sudo apt update && sudo apt upgrade -y

# Install essentials
sudo apt install -y \
  curl \
  git \
  jq \
  build-essential \
  ca-certificates \
  gnupg \
  unzip \
  wget
```

### 3.2 Node.js 20 LTS

```bash
# Add NodeSource repository for Node.js 20 LTS
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -

# Install Node.js
sudo apt install -y nodejs

# Verify
node --version   # Expected: v20.x.x
npm --version    # Expected: 10.x.x
```

### 3.3 Docker and Docker Compose

```bash
# Add Docker's official GPG key
curl -fsSL https://download.docker.com/linux/ubuntu/gpg \
  | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

# Add Docker repository
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
  https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" \
  | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Install Docker Engine and Compose plugin
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Add your user to the docker group (avoids needing sudo)
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker --version           # Expected: Docker version 26.x.x
docker compose version     # Expected: Docker Compose version v2.x.x
```

### 3.4 Rust and Stellar CLI

Rust is required for building Soroban smart contracts and running the Stellar CLI.

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Add wasm32 target for Soroban contract compilation
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Verify
rustc --version      # Expected: rustc 1.78.x or newer
stellar --version    # Expected: stellar 21.x.x or newer
```

### 3.5 Clone the Repository

```bash
git clone https://github.com/Epta-Node/ai-net.git
cd ai-net

# Install root-level npm dependencies
npm install
```

---

## 4. Configuring Secrets and Environment Variables

### 4.1 Generate Node Keypairs

Create dedicated Stellar keypairs for your node. Never reuse development keys in production.

```bash
# Generate coordinator keypair for testnet
stellar keys generate coordinator-testnet --network testnet

# View the public address
stellar keys address coordinator-testnet

# View the secret key (copy to a secure location)
stellar keys show coordinator-testnet
```

**Role separation:**

| Key | Purpose | Security |
|---|---|---|
| `STELLAR_COORDINATOR_SECRET` | Hot wallet — signs payment transactions | In `.env`, never commit |
| Admin Key | Contract upgrades and governance | Cold wallet / hardware device |

### 4.2 Backend Environment File

```bash
# Copy the example file
cp backend/.env.example backend/.env

# Restrict permissions
chmod 600 backend/.env
```

Edit `backend/.env` with your configuration:

```ini
# ── Server ────────────────────────────────────────────────────────────────────
NODE_ENV=production
PORT=3000
LOG_LEVEL=info
GRACEFUL_SHUTDOWN_TIMEOUT=30

# ── Database (SQLite — three separate databases) ──────────────────────────────
# Each database file is auto-created and migrated on first start.
DATABASE_URL=./data/ai-net.db

# ── CORS ──────────────────────────────────────────────────────────────────────
ALLOWED_ORIGINS=https://your-domain.com

# ── Security ──────────────────────────────────────────────────────────────────
# Optional: comma-separated bearer keys for protected endpoints
API_KEYS=

# Optional: shared secret for /health/dashboard admin endpoint
ADMIN_API_KEY=

# ── Stellar Network ───────────────────────────────────────────────────────────
STELLAR_NETWORK=testnet
STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org

# Hot wallet for transaction signing
STELLAR_COORDINATOR_SECRET=SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Optional: deployed AgentRegistry contract ID
REGISTRY_CONTRACT_ID=

# Set true only in local/test to skip Horizon account validation
SKIP_STELLAR_ACCOUNT_VERIFY=false

# ── Venice AI ─────────────────────────────────────────────────────────────────
VENICE_API_KEY=your_venice_api_key_here
VENICE_BASE_URL=https://api.venice.ai/api/v1

# ── Caching ───────────────────────────────────────────────────────────────────
CACHE_DRIVER=lru
CACHE_LRU_MAX_SIZE=500
CACHE_TTL_AGENTS=60
CACHE_TTL_STATS=30
CACHE_TTL_HEALTH=10

# ── Rate Limiting ─────────────────────────────────────────────────────────────
RATE_LIMIT_WINDOW_MS=60000
RATE_LIMIT_MAX_REQUESTS=20
DAILY_TASK_LIMIT_PER_WALLET=100

# ── Agent Lifecycle ───────────────────────────────────────────────────────────
HEARTBEAT_INTERVAL_MS=300000
HEARTBEAT_STALE_THRESHOLD_MINUTES=5
AGENT_OFFLINE_DELETE_HOURS=24

# ── Reconciliation ────────────────────────────────────────────────────────────
RECONCILIATION_INTERVAL_MS=86400000
```

> Full variable reference: see `backend/.env.example` in the repository.

### 4.3 Frontend Environment File (Optional)

```bash
cp frontend/.env.example frontend/.env
```

```ini
VITE_API_BASE_URL=http://localhost:3000
VITE_STELLAR_NETWORK=testnet
VITE_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
```

### 4.4 Smart Contracts Environment File

```bash
cp smart-contracts/.env.example smart-contracts/.env
```

```ini
STELLAR_SECRET_KEY=SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
VENICE_API_KEY=your_venice_api_key_here
STELLAR_NETWORK=testnet
```

### 4.5 Secrets Management Best Practices

- **Never commit `.env` files.** They are listed in `.gitignore` but always double-check before pushing.
- **Production:** Use a secrets manager such as AWS Secrets Manager, HashiCorp Vault, or GCP Secret Manager and inject values at runtime.
- **Rotate keys regularly.** Especially after any suspected exposure.
- **Separate testnet and mainnet keypairs.** Never use the same secret key on both networks.

---

## 5. Deploying Smart Contracts to Testnet and Mainnet

### 5.1 Build Contracts

```bash
cd smart-contracts

# Build all Soroban contracts to optimized Wasm
cargo build --target wasm32-unknown-unknown --release

# Verify Wasm artifacts exist
ls target/wasm32-unknown-unknown/release/*.wasm
```

### 5.2 Initialize Stellar CLI Network Configuration

```bash
# Configure testnet profile
stellar network add \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  testnet

# Verify configuration
stellar network ls
```

### 5.3 Deploy to Testnet

```bash
cd smart-contracts

# Initialize deployment environment (creates deployment state file)
./scripts/manage.sh init

# Deploy all contracts to testnet
./scripts/deploy.sh --network testnet

# Verify deployment
./scripts/verify.sh --network testnet
```

Deployment addresses are saved to `smart-contracts/deployments/testnet.json`:

```json
{
  "network": "testnet",
  "deployedAt": "2026-09-29T13:00:00.000Z",
  "contracts": {
    "agent_registry": "CCXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "payment_escrow": "CDXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "dispute_resolution": "CEXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "error_registry": "CFXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
  }
}
```

Copy the `agent_registry` contract ID into `backend/.env`:

```bash
# Extract and set registry contract ID
REGISTRY_ID=$(jq -r '.contracts.agent_registry' smart-contracts/deployments/testnet.json)
sed -i "s/^REGISTRY_CONTRACT_ID=.*/REGISTRY_CONTRACT_ID=${REGISTRY_ID}/" backend/.env
echo "Registry Contract ID set: $REGISTRY_ID"
```

### 5.4 Deploy to Mainnet

> ⚠️ Mainnet deployment uses real XLM. Ensure your keypair is funded with at least **100 XLM** before proceeding.

```bash
cd smart-contracts

# Dry run first — review what will be deployed
./scripts/deploy.sh --network mainnet --dry-run

# Deploy to mainnet (requires confirmation)
./scripts/deploy.sh --network mainnet

# Verify mainnet deployment
./scripts/verify.sh --network mainnet
```

Mainnet addresses are saved to `smart-contracts/deployments/mainnet.json`. Update `backend/.env` accordingly:

```bash
# Switch to mainnet in backend env
sed -i 's/STELLAR_NETWORK=testnet/STELLAR_NETWORK=mainnet/' backend/.env
sed -i 's|STELLAR_HORIZON_URL=.*|STELLAR_HORIZON_URL=https://horizon.stellar.org|' backend/.env
sed -i 's|SOROBAN_RPC_URL=.*|SOROBAN_RPC_URL=https://soroban-mainnet.stellar.org|' backend/.env
```

### 5.5 Smart Contract Upgrades

To upgrade deployed contracts to a new version:

```bash
cd smart-contracts

# Dry run — preview what would be upgraded
./scripts/upgrade.sh --network testnet --dry-run

# Upgrade a specific contract using the upgrade manager
./scripts/upgrade.sh --network testnet --use-upgrade-manager agent_registry

# Upgrade all contracts
./scripts/upgrade.sh --network testnet --use-upgrade-manager

# Upgrade to a specific version
./scripts/upgrade.sh --network testnet --use-upgrade-manager --version "1.2.0" agent_registry
```

> The upgrade system provides a **48-hour rollback window**. See [smart-contracts/docs/UPGRADE_GUIDE.md](../smart-contracts/docs/UPGRADE_GUIDE.md) for full procedures.

---

## 6. Funding Stellar Accounts

### 6.1 Testnet — Friendbot

```bash
# Get your coordinator's public key
PUBLIC_KEY=$(stellar keys address coordinator-testnet)
echo "Public Key: $PUBLIC_KEY"

# Fund via Stellar CLI
stellar keys fund coordinator-testnet --network testnet

# Or via curl
curl -X POST "https://friendbot.stellar.org?addr=${PUBLIC_KEY}"

# Verify balance (should show ~10,000 XLM)
curl -s "https://horizon-testnet.stellar.org/accounts/${PUBLIC_KEY}" \
  | jq '.balances[] | select(.asset_type == "native") | .balance'
```

### 6.2 Mainnet — Transfer XLM

On mainnet, fund your coordinator wallet by transferring XLM from an exchange or another wallet:

```bash
# Show your coordinator's mainnet public key
stellar keys address coordinator-mainnet

# After funding, verify the balance
PUBLIC_KEY=$(stellar keys address coordinator-mainnet)
curl -s "https://horizon.stellar.org/accounts/${PUBLIC_KEY}" \
  | jq '.balances[] | select(.asset_type == "native") | .balance'
```

### 6.3 Minimum Balance Requirements

| Account | Minimum XLM | Purpose |
|---|---|---|
| Coordinator (hot wallet) | 10 XLM | Base reserve + transaction fees |
| Escrow operational buffer | 50 XLM | Ongoing escrow funding |
| Agent registration bond | 10 XLM per agent | Per-agent stake |

> Stellar requires a **base reserve of 1 XLM** per account plus **0.5 XLM** per trustline or subentry. Keep at least 5 XLM above your operational minimum as a safety margin.

### 6.4 Set Up a Balance Alert

```bash
# Save this script as /opt/ai-net/scripts/check-balance.sh
cat > /opt/ai-net/scripts/check-balance.sh << 'EOF'
#!/bin/bash
HORIZON="${STELLAR_HORIZON_URL:-https://horizon-testnet.stellar.org}"
PUBLIC_KEY="${STELLAR_PUBLIC_KEY}"
THRESHOLD=20  # Alert if below 20 XLM

BALANCE=$(curl -s "${HORIZON}/accounts/${PUBLIC_KEY}" \
  | jq -r '.balances[] | select(.asset_type == "native") | .balance')

echo "Current balance: ${BALANCE} XLM"

if (( $(echo "$BALANCE < $THRESHOLD" | bc -l) )); then
  echo "WARNING: Balance below ${THRESHOLD} XLM threshold!"
  # Add your alerting command here (e.g., send Slack/email notification)
fi
EOF
chmod +x /opt/ai-net/scripts/check-balance.sh

# Run every hour via cron
(crontab -l 2>/dev/null; echo "0 * * * * /opt/ai-net/scripts/check-balance.sh >> /var/log/ai-net-balance.log 2>&1") | crontab -
```

---

## 7. Starting and Stopping the Node

### Option A: Docker Compose (Recommended)

Docker Compose runs the full stack — Stellar standalone node, backend API, and frontend dashboard — with one command.

```bash
cd /path/to/ai-net

# Copy environment variables
cp .env.example .env
# Edit .env with your VENICE_API_KEY and Stellar keys

# Start all services in detached mode
docker compose up -d

# Check service status
docker compose ps

# Stream live logs
docker compose logs -f backend

# Check health
curl -s http://localhost:3000/health | jq
```

**Services started by Docker Compose:**

| Service | Port | Description |
|---|---|---|
| `stellar-standalone` | `8000` | Local Stellar node with Soroban RPC |
| `backend` | `3000` | ai-net REST API |
| `frontend` | `5173` | Web dashboard |

**Stop all services:**

```bash
# Graceful stop (waits for in-flight requests)
docker compose stop

# Remove containers (data volumes preserved)
docker compose down

# Remove containers AND volumes (destroys SQLite data — use with caution)
docker compose down -v
```

### Option B: Bare Metal / Systemd

For production deployments on a dedicated server:

#### Step 1: Create a system user

```bash
sudo useradd -r -s /bin/false -d /opt/ai-net ainet
sudo mkdir -p /opt/ai-net
sudo chown -R ainet:ainet /opt/ai-net
```

#### Step 2: Clone and install

```bash
sudo -u ainet git clone https://github.com/Epta-Node/ai-net.git /opt/ai-net
cd /opt/ai-net
sudo -u ainet npm install
```

#### Step 3: Configure environment

```bash
sudo -u ainet cp backend/.env.example backend/.env
sudo -u ainet chmod 600 backend/.env
sudo -u ainet nano backend/.env  # Fill in your secrets
```

#### Step 4: Run database migrations

```bash
cd /opt/ai-net/backend
sudo -u ainet npm run db:migrate
```

#### Step 5: Build the backend

```bash
cd /opt/ai-net/backend
sudo -u ainet npm run build
```

#### Step 6: Create the systemd service

```bash
sudo tee /etc/systemd/system/ainet-node.service > /dev/null << 'EOF'
[Unit]
Description=ai-net Coordinator and Payment Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=ainet
Group=ainet
WorkingDirectory=/opt/ai-net/backend
EnvironmentFile=/opt/ai-net/backend/.env
ExecStart=/usr/bin/node dist/index.js
Restart=always
RestartSec=5s
LimitNOFILE=65536

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/ai-net/backend/data

[Install]
WantedBy=multi-user.target
EOF
```

#### Step 7: Enable and start

```bash
sudo systemctl daemon-reload
sudo systemctl enable ainet-node
sudo systemctl start ainet-node

# Check status
sudo systemctl status ainet-node

# Follow logs
sudo journalctl -u ainet-node -f
```

#### Stopping and restarting

```bash
# Graceful stop (respects GRACEFUL_SHUTDOWN_TIMEOUT)
sudo systemctl stop ainet-node

# Restart
sudo systemctl restart ainet-node

# Reload without downtime (if supported)
sudo systemctl reload ainet-node
```

### Database Migrations

Migrations run automatically on start. To run them manually:

```bash
cd /opt/ai-net/backend

# Apply all pending migrations (agents.db, tasks.db, payments.db)
npm run db:migrate

# Roll back the most recent migration
npm run db:rollback

# Seed with local dev sample data
npm run db:seed
```

---

## 8. Health Check and Monitoring

### 8.1 HTTP Health Endpoints

#### `GET /health` — Overall system health

```bash
curl -s http://localhost:3000/health | jq
```

**Response (`200 OK`):**

```json
{
  "status": "healthy",
  "timestamp": "2026-09-29T13:00:00.000Z",
  "services": {
    "database": "connected",
    "stellarRpc": "connected"
  }
}
```

#### `GET /health/ready` — Kubernetes readiness probe

```bash
curl -s http://localhost:3000/health/ready
```

```json
{ "ready": true }
```

#### `GET /health/live` — Kubernetes liveness probe

```bash
curl -s http://localhost:3000/health/live
```

```json
{ "live": true }
```

### 8.2 Node Verification Checklist

Run these checks after every start or restart:

```bash
# 1. Health endpoint responds 200
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/health
# Expected: 200

# 2. Agents endpoint lists registered agents
curl -s "http://localhost:3000/api/v1/agents?limit=5" | jq '.agents | length'
# Expected: a non-negative integer

# 3. Stats endpoint is reachable
curl -s http://localhost:3000/api/v1/stats | jq '{totalAgents, activeAgents}'

# 4. Stellar RPC is reachable from the backend
curl -s "${SOROBAN_RPC_URL}" -d '{"jsonrpc":"2.0","id":0,"method":"getHealth"}' \
  -H "Content-Type: application/json" | jq '.result.status'
# Expected: "healthy"
```

### 8.3 Prometheus Metrics

The node exposes Prometheus metrics at `GET /metrics`:

```bash
curl -s http://localhost:3000/metrics | grep "^ainet_"
```

**Key metrics:**

| Metric | Description |
|---|---|
| `ainet_tasks_total{status="completed"}` | Total tasks completed |
| `ainet_tasks_total{status="failed"}` | Total tasks failed |
| `ainet_task_duration_seconds` | Task execution latency histogram |
| `ainet_active_agents` | Current active agent count |
| `ainet_escrow_settlements_total` | On-chain escrow settlements |
| `ainet_stellar_rpc_latency_seconds` | Stellar RPC call latency |
| `ainet_heartbeat_stale_total` | Agents marked stale (missed heartbeat) |

**Prometheus scrape configuration:**

```yaml
# Add to /etc/prometheus/prometheus.yml
scrape_configs:
  - job_name: 'ainet_node'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:3000']
```

### 8.4 Grafana Dashboard

The repository ships a pre-built Grafana dashboard at `backend/grafana/ai-net-dashboard.json`.

```bash
# Import the dashboard into Grafana
curl -s -X POST http://localhost:3030/api/dashboards/import \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${GRAFANA_API_KEY}" \
  -d "{\"dashboard\": $(cat backend/grafana/ai-net-dashboard.json), \"overwrite\": true}"
```

### 8.5 Log Aggregation

Logs are structured JSON (via Pino). Configure your log pipeline:

```bash
# View logs in human-readable format with pino-pretty
cd /opt/ai-net/backend
npm run dev 2>&1 | npx pino-pretty

# Or follow systemd logs with JSON output
journalctl -u ainet-node -f -o cat | jq
```

**For production log forwarding to a centralized system (e.g., Loki, Datadog, CloudWatch):**

```bash
# Example: forward logs to a Loki instance via promtail
# Configure promtail to scrape /var/log/syslog filtered by "ainet-node"
```

---

## 9. Upgrading the Node to a New Version

### 9.1 Docker Compose Upgrade

```bash
cd /path/to/ai-net

# Pull latest code
git pull origin main

# Rebuild containers with no cache
docker compose build --no-cache

# Rolling restart (stops and starts services one at a time)
docker compose up -d --force-recreate

# Verify health after upgrade
sleep 5
curl -s http://localhost:3000/health | jq '.status'
# Expected: "healthy"
```

### 9.2 Bare Metal / Systemd Upgrade

```bash
cd /opt/ai-net

# Step 1: Pull the latest code
sudo -u ainet git fetch origin
sudo -u ainet git pull origin main

# Step 2: Install any new dependencies
cd backend
sudo -u ainet npm ci

# Step 3: Rebuild
sudo -u ainet npm run build

# Step 4: Apply database migrations (safe to run multiple times)
sudo -u ainet npm run db:migrate

# Step 5: Restart the service (graceful shutdown + start)
sudo systemctl restart ainet-node

# Step 6: Verify
sudo systemctl status ainet-node
curl -s http://localhost:3000/health | jq '.status'
```

### 9.3 Smart Contract Upgrades

See [Deploying Smart Contracts — §5.5](#55-smart-contract-upgrades) for the full upgrade workflow.

### 9.4 Rollback Procedure

If the new version is faulty:

```bash
# Find the last working commit
git log --oneline -10

# Roll back to a specific commit
sudo -u ainet git checkout <LAST_GOOD_COMMIT>

# Rebuild and restart
cd backend && sudo -u ainet npm ci && sudo -u ainet npm run build
sudo systemctl restart ainet-node

# Roll back database if needed
sudo -u ainet npm run db:rollback
```

---

## 10. Troubleshooting Common Errors

### Error 1: `tx_bad_seq` — Stellar Transaction Bad Sequence

**Symptom:** Transactions fail with Horizon error code `tx_bad_seq`.

**Cause:** Concurrent transactions from the same keypair caused a sequence number collision.

**Resolution:**
```bash
# Check current sequence number
PUBLIC_KEY=$(stellar keys address coordinator-testnet)
curl -s "https://horizon-testnet.stellar.org/accounts/${PUBLIC_KEY}" | jq '.sequence'

# The node auto-retries with the correct sequence number.
# If the error persists, restart the backend to flush the in-memory sequence cache.
sudo systemctl restart ainet-node
```

---

### Error 2: `insufficient_balance` / `op_underfunded`

**Symptom:** On-chain escrow or payment transfer fails with `insufficient_balance`.

**Cause:** Coordinator hot wallet does not have enough XLM to cover the transaction amount plus base reserve.

**Resolution:**
```bash
# Check balance
PUBLIC_KEY=$(stellar keys address coordinator-testnet)
curl -s "https://horizon-testnet.stellar.org/accounts/${PUBLIC_KEY}" \
  | jq '.balances[] | select(.asset_type == "native") | .balance'

# Testnet: refill via Friendbot
stellar keys fund coordinator-testnet --network testnet

# Mainnet: transfer XLM from your funding wallet
```

---

### Error 3: `429 Too Many Requests` — Horizon Rate Limit

**Symptom:** Backend logs show `429` responses from Horizon.

**Cause:** The public SDF Horizon rate limit has been exceeded.

**Resolution:**
```bash
# Option 1: Switch to a dedicated Horizon provider
# In backend/.env:
STELLAR_HORIZON_URL=https://horizon.blockdaemon.com/stellar/testnet

# Option 2: Run your own Horizon instance (advanced)
docker run stellar/quickstart:testing --testnet

# Restart backend after changing config
sudo systemctl restart ainet-node
```

---

### Error 4: `CircuitBreakerOpenError` — Venice AI Unreachable

**Symptom:** Agent task assignments fail with `CircuitBreakerOpenError`.

**Cause:** Consecutive Venice AI API timeouts or 5xx errors tripped the circuit breaker.

**Resolution:**
```bash
# 1. Verify your API key is valid
curl -s https://api.venice.ai/api/v1/models \
  -H "Authorization: Bearer ${VENICE_API_KEY}" | jq '.data[0].id'

# 2. Check Venice AI status at https://status.venice.ai

# 3. The circuit breaker auto-resets after 60 seconds.
#    If it doesn't, restart the backend:
sudo systemctl restart ainet-node
```

---

### Error 5: Agent Marked `offline` — Heartbeat Timeout

**Symptom:** An agent's status changes to `offline` and it stops receiving tasks.

**Cause:** Agent worker process crashed, or network latency prevented heartbeats within `HEARTBEAT_STALE_THRESHOLD_MINUTES` (default: 5 minutes).

**Resolution:**
```bash
# Check agent status
curl -s "http://localhost:3000/api/v1/agents/AGENT_ID" | jq '{status, lastHeartbeat}'

# View agent worker logs
sudo journalctl -u ainet-node -f --grep="heartbeat"

# Restart the service to re-register all agent workers
sudo systemctl restart ainet-node

# Or send a manual heartbeat via API
curl -s -X POST "http://localhost:3000/api/v1/agents/AGENT_ID/heartbeat" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ${AGENT_API_KEY}" \
  -d '{"status": "idle", "activeJobs": 0}'
```

---

### Error 6: Database Migration Fails on Startup

**Symptom:** Backend crashes with `migration error` or `SQLITE_LOCKED`.

**Cause:** A previous migration was interrupted, leaving the database in a partial state.

**Resolution:**
```bash
cd /opt/ai-net/backend

# Check migration status
npm run db:migrate -- --status

# Roll back the failed migration
npm run db:rollback

# Re-apply migrations
npm run db:migrate

# If the database is corrupted, restore from backup (see §11 Backup section below)
```

---

### Error 7: `EADDRINUSE` — Port Already in Use

**Symptom:** Backend fails to start with `Error: listen EADDRINUSE :::3000`.

**Cause:** Another process is already using port 3000.

**Resolution:**
```bash
# Find what's using port 3000
sudo lsof -i :3000
# or
sudo ss -tlnp | grep :3000

# Kill the conflicting process (replace PID)
sudo kill -9 <PID>

# Or change the PORT in backend/.env
PORT=3001
sudo systemctl restart ainet-node
```

---

### Error 8: Docker Compose — `unhealthy` or `Exit 1`

**Symptom:** `docker compose ps` shows a service as `unhealthy` or `Exit 1`.

**Resolution:**
```bash
# View detailed container logs
docker compose logs backend --tail=100

# Check exit code
docker compose ps

# Inspect health check output
docker inspect ai-net-backend | jq '.[0].State.Health.Log[-1]'

# Common causes:
# - Missing .env values (check that VENICE_API_KEY is set)
# - Stellar standalone not ready yet (backend depends on it)
# - Port conflict with another container

# Full reset if needed
docker compose down
docker compose up -d
```

---

### Error 9: `STELLAR_COORDINATOR_SECRET` Not Set

**Symptom:** Backend logs `STELLAR_COORDINATOR_SECRET is required` or payment endpoints return 500.

**Resolution:**
```bash
# Verify the variable is set
grep STELLAR_COORDINATOR_SECRET backend/.env

# If missing, generate a keypair and set it
stellar keys generate coordinator --network testnet
stellar keys show coordinator  # Copy the secret key (starts with S)

# Add to backend/.env
echo "STELLAR_COORDINATOR_SECRET=SBXXXXX..." >> backend/.env
sudo systemctl restart ainet-node
```

---

### Error 10: Smart Contract Deploy Fails — `WasmNotFound`

**Symptom:** `./scripts/deploy.sh` fails with `WasmNotFound` or missing artifact.

**Cause:** Soroban Wasm artifacts were not built before deploying.

**Resolution:**
```bash
cd smart-contracts

# Build contracts first
cargo build --target wasm32-unknown-unknown --release

# Verify artifacts exist
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Now deploy
./scripts/deploy.sh --network testnet
```

---

### Error 11: `CORS` Policy Error in Browser

**Symptom:** Frontend shows `CORS policy` error when connecting to the backend.

**Resolution:**
```bash
# In backend/.env, add your frontend origin
ALLOWED_ORIGINS=http://localhost:5173,https://your-domain.com

# Restart backend
sudo systemctl restart ainet-node
```

---

### Error 12: Node Runs Out of Disk Space

**Symptom:** Backend crashes with `SQLITE_FULL` or log files fill the disk.

**Resolution:**
```bash
# Check disk usage
df -h /opt/ai-net

# Check SQLite database sizes
ls -lh /opt/ai-net/backend/data/*.db

# Compact SQLite databases
sqlite3 /opt/ai-net/backend/data/tasks.db "VACUUM;"
sqlite3 /opt/ai-net/backend/data/payments.db "VACUUM;"
sqlite3 /opt/ai-net/backend/data/agents.db "VACUUM;"

# Rotate logs
sudo journalctl --vacuum-size=500M
```

---

### General Debugging Commands

```bash
# View recent backend logs (systemd)
sudo journalctl -u ainet-node -n 200 --no-pager

# View backend logs (Docker Compose)
docker compose logs backend --tail=200

# Check all environment variables loaded by the service
sudo systemctl show ainet-node --property=Environment

# Test Stellar RPC connectivity
curl -s https://soroban-testnet.stellar.org \
  -d '{"jsonrpc":"2.0","id":0,"method":"getHealth"}' \
  -H "Content-Type: application/json" | jq

# Test Venice AI API key validity
curl -s https://api.venice.ai/api/v1/models \
  -H "Authorization: Bearer ${VENICE_API_KEY}" | jq '.data | length'
# Expected: a positive integer (number of available models)
```

---

## Appendix: Backup and Disaster Recovery

### SQLite Database Backup

```bash
#!/bin/bash
# /opt/ai-net/scripts/backup-db.sh
BACKUP_DIR="/var/backups/ainet"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
DB_DIR="/opt/ai-net/backend/data"

mkdir -p "$BACKUP_DIR"

# Backup each database using SQLite's online backup API
for db in agents tasks payments; do
  sqlite3 "${DB_DIR}/${db}.db" ".backup '${BACKUP_DIR}/${db}_${TIMESTAMP}.db'"
  echo "Backed up ${db}.db → ${BACKUP_DIR}/${db}_${TIMESTAMP}.db"
done

# Retain last 14 days of backups
find "$BACKUP_DIR" -name "*.db" -mtime +14 -delete
echo "Backup complete at $TIMESTAMP"
```

Add to crontab:

```bash
chmod +x /opt/ai-net/scripts/backup-db.sh
(crontab -l 2>/dev/null; echo "0 2 * * * /opt/ai-net/scripts/backup-db.sh >> /var/log/ainet-backup.log 2>&1") | crontab -
```

### Database Restore

```bash
# Stop the node
sudo systemctl stop ainet-node

# Restore a database from backup
cp /var/backups/ainet/tasks_20260929_020000.db /opt/ai-net/backend/data/tasks.db
chown ainet:ainet /opt/ai-net/backend/data/tasks.db

# Restart the node
sudo systemctl start ainet-node
```

---

## Additional Resources

- [REST API Reference](API_REFERENCE.md) — All endpoints with request/response examples
- [AI Agent Integration Guide](ai-agent-integration-guide.md) — Guide for third-party agent operators
- [Developer Setup Guide](DEVELOPER_SETUP.md) — Local development setup
- [Smart Contract Deployment Guide](../smart-contracts/docs/DEPLOYMENT_GUIDE.md) — Soroban deployment workflows
- [Upgrade Guide](../smart-contracts/docs/UPGRADE_GUIDE.md) — Smart contract upgrade procedures
- [Architecture Overview](architecture/index.md) — System architecture and component diagrams
- [CONTRIBUTING.md](../CONTRIBUTING.md) — How to contribute to the project
- **GitHub Issues**: [github.com/Epta-Node/ai-net/issues](https://github.com/Epta-Node/ai-net/issues)

---

*Last updated: September 2026 · Issue [#62](https://github.com/Epta-Node/ai-net/issues/62)*
