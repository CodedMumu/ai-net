import { Router, Request, Response, RequestHandler } from "express";
import { getConfig, ttlForRoute } from "../../config";
import { adminAuthMiddleware } from "../middleware/auth";
import { metricsService } from "../../services/metrics";
import { tracingService } from "../../services/tracing";
import { cacheMiddleware } from "../middleware/cache";

const router = Router();
const startTime = Date.now();

/** Lazily initialised cache middleware per route-group. */
function cachedRoute(group: "health"): RequestHandler {
  let middleware: RequestHandler | null = null;
  return (req, res, next) => {
    if (!middleware) {
      middleware = cacheMiddleware({ ttl: ttlForRoute(group) });
    }
    return middleware(req, res, next);
  };
}

// ─── Types ────────────────────────────────────────────────────────────────────

type HealthStatus = "ok" | "error" | "degraded" | "unreachable";

interface DependencyResult {
  status: HealthStatus;
  responseTimeMs: number;
  message?: string;
}

interface DetailedHealthResponse {
  status: HealthStatus;
  uptime: number;
  version: string;
  stellarNetwork: string;
  timestamp: string;
  responseTimeMs: number;
  dependencies: {
    sqlite: {
      agents: DependencyResult;
      tasks: DependencyResult;
      payments: DependencyResult;
    };
    venice: DependencyResult;
    horizon: DependencyResult;
    queue: DependencyResult;
  };
}

// ─── Dependency checks ────────────────────────────────────────────────────────

async function checkSqliteDb(
  importFn: () => Promise<{ getDb?: () => any; getAgentDb?: () => any; getTaskDb?: () => any; closeDb?: () => void; closeAgentDb?: () => void; closeTaskDb?: () => void }>,
  getterKey: "getDb" | "getAgentDb" | "getTaskDb",
  closerKey: "closeDb" | "closeAgentDb" | "closeTaskDb",
): Promise<DependencyResult> {
  const t0 = Date.now();
  try {
    const mod = await importFn();
    const getter = mod[getterKey] as (() => any) | undefined;
    const closer = mod[closerKey] as (() => void) | undefined;
    if (!getter) {
      return { status: "error", responseTimeMs: Date.now() - t0, message: "DB getter not found" };
    }
    const db = getter();
    db.prepare("SELECT 1").get();
    closer?.();
    return { status: "ok", responseTimeMs: Date.now() - t0 };
  } catch (err) {
    return {
      status: "error",
      responseTimeMs: Date.now() - t0,
      message: err instanceof Error ? err.message : String(err),
    };
  }
}

async function checkVenice(apiKey: string, timeoutMs = 5_000): Promise<DependencyResult> {
  const t0 = Date.now();
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    const response = await fetch("https://api.venice.ai/api/v1/models", {
      headers: { Authorization: `Bearer ${apiKey}` },
      signal: controller.signal,
    });
    clearTimeout(timer);
    // 401 means the key is wrong but the API is reachable
    const ok = response.ok || response.status === 401;
    return {
      status: ok ? "ok" : "unreachable",
      responseTimeMs: Date.now() - t0,
      message: ok ? undefined : `HTTP ${response.status}`,
    };
  } catch (err) {
    return {
      status: "unreachable",
      responseTimeMs: Date.now() - t0,
      message: err instanceof Error ? err.message : String(err),
    };
  }
}

async function checkHorizon(horizonUrl: string, timeoutMs = 5_000): Promise<DependencyResult> {
  const t0 = Date.now();
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    // /fee_stats is a lightweight, unauthenticated Horizon endpoint
    const url = `${horizonUrl.replace(/\/$/, "")}/fee_stats`;
    const response = await fetch(url, { signal: controller.signal });
    clearTimeout(timer);
    return {
      status: response.ok ? "ok" : "unreachable",
      responseTimeMs: Date.now() - t0,
      message: response.ok ? undefined : `HTTP ${response.status}`,
    };
  } catch (err) {
    return {
      status: "unreachable",
      responseTimeMs: Date.now() - t0,
      message: err instanceof Error ? err.message : String(err),
    };
  }
}

async function checkQueue(): Promise<DependencyResult> {
  const t0 = Date.now();
  try {
    const config = getConfig();
    if (config.CACHE_DRIVER === "redis") {
      // Redis/Bull queue: attempt a quick ping via node-redis or ioredis if available
      try {
        const { createClient } = await import("redis" as any);
        const client = createClient({ url: config.REDIS_URL });
        await client.connect();
        await client.ping();
        await client.disconnect();
        return { status: "ok", responseTimeMs: Date.now() - t0 };
      } catch (err) {
        return {
          status: "error",
          responseTimeMs: Date.now() - t0,
          message: err instanceof Error ? err.message : String(err),
        };
      }
    }
    // SQLite-backed job store (default)
    const queueModule = await import("../../queue/jobStore.js");
    const getJobDb = (queueModule as any).getJobDb as (() => any) | undefined;
    const closeJobDb = (queueModule as any).closeJobDb as (() => void) | undefined;
    if (!getJobDb) {
      return { status: "ok", responseTimeMs: Date.now() - t0, message: "SQLite queue (no health fn)" };
    }
    const db = getJobDb();
    db.prepare("SELECT 1").get();
    closeJobDb?.();
    return { status: "ok", responseTimeMs: Date.now() - t0 };
  } catch (err) {
    return {
      status: "error",
      responseTimeMs: Date.now() - t0,
      message: err instanceof Error ? err.message : String(err),
    };
  }
}

// ─── Liveness handler (shared by GET / and GET /live) ─────────────────────────

function livenessHandler(_req: Request, res: Response): void {
  const config = getConfig();
  res.json({
    status: "ok",
    uptime: Math.floor((Date.now() - startTime) / 1000),
    version: config.NPM_PACKAGE_VERSION,
    stellarNetwork: config.STELLAR_NETWORK,
  });
}

// ─── Routes ───────────────────────────────────────────────────────────────────

/**
 * @openapi
 * /health:
 *   get:
 *     summary: Basic liveness check
 *     operationId: getHealth
 *     description: Returns service status, uptime, version, and Stellar network. No dependency checks.
 *     tags: [Health]
 *     security: []
 *     responses:
 *       200:
 *         description: Service is up
 */
router.get("/", cachedRoute("health"), livenessHandler);

/**
 * @openapi
 * /health/live:
 *   get:
 *     summary: Basic liveness check (alias)
 *     operationId: getLive
 *     description: Alias for `GET /health` — process-only liveness, no dependency checks.
 *     tags: [Health]
 *     security: []
 *     responses:
 *       200:
 *         description: Service is up
 */
router.get("/live", livenessHandler);

/**
 * @openapi
 * /health/deep:
 *   get:
 *     summary: Deep dependency health check
 *     operationId: getDeepHealth
 *     description: |
 *       Checks all service dependencies (SQLite databases, Venice AI, Stellar Horizon, queue)
 *       and returns per-dependency status with response times.
 *       Returns 200 if all critical dependencies are healthy, 503 otherwise.
 *     tags: [Health]
 *     security: []
 *     responses:
 *       200:
 *         description: All dependencies healthy
 *       503:
 *         description: One or more critical dependencies are down
 */
router.get("/deep", async (_req: Request, res: Response) => {
  const overallStart = Date.now();
  const config = getConfig();
  const timeoutMs = config.HEALTH_PROBE_TIMEOUT_MS;

  // Run all dependency checks in parallel
  const [agentsResult, tasksResult, paymentsResult, veniceResult, horizonResult, queueResult] =
    await Promise.all([
      checkSqliteDb(
        () => import("../../db/agents.js"),
        "getAgentDb",
        "closeAgentDb",
      ),
      checkSqliteDb(
        () => import("../../db/tasks.js"),
        "getTaskDb",
        "closeTaskDb",
      ),
      checkSqliteDb(
        () => import("../../db/index.js"),
        "getDb",
        "closeDb",
      ),
      checkVenice(config.VENICE_API_KEY, timeoutMs),
      checkHorizon(config.STELLAR_HORIZON_URL, timeoutMs),
      checkQueue(),
    ]);

  const criticalDeps: HealthStatus[] = [
    agentsResult.status,
    tasksResult.status,
    paymentsResult.status,
    veniceResult.status,
    horizonResult.status,
  ];

  const hasError = criticalDeps.some((s) => s === "error" || s === "unreachable");
  const overallStatus: HealthStatus = hasError ? "degraded" : "ok";

  const body: DetailedHealthResponse = {
    status: overallStatus,
    uptime: Math.floor((Date.now() - startTime) / 1000),
    version: config.NPM_PACKAGE_VERSION,
    stellarNetwork: config.STELLAR_NETWORK,
    timestamp: new Date().toISOString(),
    responseTimeMs: Date.now() - overallStart,
    dependencies: {
      sqlite: {
        agents: agentsResult,
        tasks: tasksResult,
        payments: paymentsResult,
      },
      venice: veniceResult,
      horizon: horizonResult,
      queue: queueResult,
    },
  };

  // Legacy flat fields for backward compatibility
  const legacyBody = {
    ...body,
    venice: veniceResult.status === "ok" ? "ok" : "unreachable",
    horizon: horizonResult.status === "ok" ? "ok" : "unreachable",
    services: {
      venice: veniceResult.status === "ok" ? "ok" : "unreachable",
      horizon: horizonResult.status === "ok" ? "ok" : "unreachable",
    },
  };

  res.status(hasError ? 503 : 200).json(legacyBody);
});

/**
 * @openapi
 * /health/ready:
 *   get:
 *     summary: Readiness probe
 *     operationId: getReady
 *     description: |
 *       Kubernetes-style readiness probe. Checks all dependencies are responsive
 *       before allowing traffic. Returns 200 when ready, 500 when not.
 *     tags: [Health]
 *     security: []
 *     responses:
 *       200:
 *         description: Service is ready to receive traffic
 *       500:
 *         description: Service is not ready
 */
router.get("/ready", async (_req: Request, res: Response) => {
  const checks: Record<string, "ok" | "error" | "unknown"> = {
    tasks: "ok",
    payments: "ok",
    queue: "ok",
    venice: "ok",
    horizon: "ok",
    websocket: "ok",
  };

  try {
    const tasksModule = await import("../../db/tasks.js");
    const paymentsModule = await import("../../db/index.js");
    const queueModule = await import("../../queue/jobStore.js");

    try {
      const taskDb = (tasksModule.getTaskDb as Function)();
      taskDb.prepare("SELECT 1").get();
    } catch {
      checks.tasks = "error";
    } finally {
      (tasksModule.closeTaskDb as Function)();
    }

    try {
      const paymentDb = (paymentsModule.getDb as Function)();
      paymentDb.prepare("SELECT 1").get();
    } catch {
      checks.payments = "error";
    } finally {
      (paymentsModule.closeDb as Function)();
    }

    try {
      const jobDb = (queueModule.getJobDb as Function)();
      jobDb.prepare("SELECT 1").get();
    } catch {
      checks.queue = "error";
    } finally {
      (queueModule.closeJobDb as Function)();
    }
  } catch (error) {
    res.status(500).json({ status: "error", checks, error: String(error) });
    return;
  }

  const config = getConfig();
  const timeoutMs = config.HEALTH_PROBE_TIMEOUT_MS;
  const [veniceResult, horizonResult] = await Promise.all([
    checkVenice(config.VENICE_API_KEY, timeoutMs),
    checkHorizon(config.STELLAR_HORIZON_URL, timeoutMs),
  ]);
  checks.venice = veniceResult.status === "ok" ? "ok" : "error";
  checks.horizon = horizonResult.status === "ok" ? "ok" : "error";

  const websocketStatus = metricsService.getWebSocketStatus();
  checks.websocket =
    websocketStatus.status === "unknown"
      ? "unknown"
      : websocketStatus.status === "ok"
        ? "ok"
        : "error";

  const failing = Object.values(checks).filter((status) => status === "error");
  const ready = failing.length === 0;
  res.status(ready ? 200 : 500).json({ status: ready ? "ok" : "error", checks });
});

router.get("/dashboard", adminAuthMiddleware, async (req: Request, res: Response) => {
  try {
    const dashboard = await metricsService.getDashboard(req.query.refresh === "true");
    res.json(dashboard);
  } catch (error) {
    res.status(500).json({
      status: "unhealthy",
      error: "Failed to collect metrics",
      message: error instanceof Error ? error.message : String(error),
    });
  }
});

router.get("/traces/:traceId", (req: Request, res: Response) => {
  const trace = tracingService.getTrace(req.params.traceId);
  if (!trace) {
    res.status(404).json({
      error: "Trace not found",
      traceId: req.params.traceId,
      correlationId: req.params.traceId,
    });
    return;
  }
  res.json(trace);
});

export { router as healthRouter };
export default router;
