/**
 * Rate-limiting middleware for the ai-net backend API.
 *
 * Three tiers with distinct limits, all configurable via environment variables:
 *
 *   public       — unauthenticated endpoints (/health, /api/stats, GET /api/agents)
 *                  100 req/min per IP (default). Keyed by client IP.
 *
 *   authed       — authenticated task-creation endpoints (/api/tasks)
 *                  30 req/min per wallet address (default). Falls back to IP when
 *                  no wallet header is present.
 *
 *   veniceProxy  — endpoints that proxy calls to Venice AI (/api/tasks POST, agents)
 *                  10 req/min per wallet address (default). Enforced to respect
 *                  upstream Venice AI quotas.
 *
 * All limiters use an in-process LRU-backed sliding-window algorithm and emit
 * standard rate-limit headers on every response so clients can back off
 * proactively rather than only learning about limits on 429.
 *
 * Violations are logged at the WARN level via the project's structured logger
 * with enough context (key, tier, endpoint) to support alerting.
 */

import { LRUCache } from "lru-cache";
import type { Request, Response, NextFunction } from "express";
import { createLogger } from "../../utils/logger";

const logger = createLogger({ module: "rate-limit" });

// ── Types ─────────────────────────────────────────────────────────────────────

export interface RateLimitOptions {
  /** Rolling window in milliseconds. Default: 60 000 (1 minute). */
  windowMs?: number;
  /** Maximum number of requests allowed within the window. Default: 20. */
  maxRequests?: number;
  /**
   * Maximum number of distinct keys tracked simultaneously per limiter instance.
   * When the limit is reached the least-recently-used key is evicted on the
   * next accepted request. Defaults to 10 000.
   */
  maxEntries?: number;
  /**
   * Label used in log messages and headers for this tier.
   * Defaults to "global".
   */
  tierLabel?: string;
}

/** Sliding-window state stored per rate-limit key. */
interface Window {
  timestamps: number[];
}

export interface RateLimiter {
  middleware: (req: Request, res: Response, next: NextFunction) => void;
  /**
   * Fully clears tracked state. Useful in tests and graceful-shutdown hooks.
   */
  stop: () => void;
  /**
   * Current number of tracked keys. Exposed for tests and operational
   * debugging — not part of the rate-limiting contract.
   * @internal
   */
  size: () => number;
}

// ── Header helpers ────────────────────────────────────────────────────────────

/**
 * Attach standard rate-limit headers to the response.
 *
 * Headers emitted on **every** response so clients can track their quota
 * without waiting for a 429:
 *  - `X-RateLimit-Limit`     — max requests allowed per window
 *  - `X-RateLimit-Remaining` — requests remaining in the current window
 *  - `X-RateLimit-Reset`     — Unix timestamp (seconds) when the window resets
 *
 * On 429 responses `Retry-After` is also set (seconds until reset).
 */
function setRateLimitHeaders(
  res: Response,
  limit: number,
  remaining: number,
  resetAtMs: number,
): void {
  const resetSec = Math.ceil(resetAtMs / 1000);
  res.setHeader("X-RateLimit-Limit", String(limit));
  res.setHeader("X-RateLimit-Remaining", String(Math.max(0, remaining)));
  res.setHeader("X-RateLimit-Reset", String(resetSec));
}

// ── Key extraction ────────────────────────────────────────────────────────────

/**
 * Extract a rate-limit key from the request.
 *
 * For authenticated routes we key by wallet address so that a single wallet
 * cannot circumvent limits by rotating IP addresses (e.g. when behind a NAT
 * or load balancer). For unauthenticated routes we fall back to the client IP.
 *
 * Wallet address is read from the `walletpublickey` request header, which is
 * the convention already used by the rest of the codebase (task routes,
 * idempotency middleware, etc.).
 */
export function extractRateLimitKey(req: Request, useWallet = false): string {
  if (useWallet) {
    const walletKey = req.headers["walletpublickey"];
    if (typeof walletKey === "string" && walletKey.trim().length > 0) {
      return `wallet:${walletKey.trim()}`;
    }
  }
  return `ip:${req.ip ?? "unknown"}`;
}

// ── Core factory ──────────────────────────────────────────────────────────────

/**
 * Create a configurable in-memory sliding-window rate limiter.
 *
 * Backing store is `lru-cache`, which provides:
 *  - a hard cap on entry count (`maxEntries`) bounding memory under IP/wallet floods; and
 *  - TTL-based eviction so quiet keys drop out automatically.
 *
 * Standard rate-limit headers are emitted on every response so clients can
 * proactively back off rather than only learning about limits on 429.
 *
 * @param opts          Configuration options (see RateLimitOptions).
 * @param useWalletKey  When true the limiter keys by wallet address extracted
 *                      from the `walletpublickey` header, falling back to IP.
 *                      Defaults to false (IP-only keying).
 */
export function createRateLimiter(opts: RateLimitOptions = {}, useWalletKey = false): RateLimiter {
  const windowMs = opts.windowMs ?? 60_000;
  const maxRequests = opts.maxRequests ?? 20;
  const maxEntries = opts.maxEntries ?? 10_000;
  const tierLabel = opts.tierLabel ?? "global";

  const windows = new LRUCache<string, Window>({
    max: maxEntries,
    ttl: windowMs,
    // Don't refresh age on read: a 429 must not let stale keys linger.
    updateAgeOnGet: false,
  });

  function middleware(req: Request, res: Response, next: NextFunction): void {
    const key = extractRateLimitKey(req, useWalletKey);
    const now = Date.now();
    const cutoff = now - windowMs;

    let win = windows.get(key) ?? { timestamps: [] };
    win.timestamps = win.timestamps.filter((t) => t > cutoff);

    const oldest = win.timestamps[0];
    const resetAtMs = oldest !== undefined ? oldest + windowMs : now + windowMs;
    const remaining = maxRequests - win.timestamps.length;

    if (win.timestamps.length >= maxRequests) {
      const retryAfter = Math.ceil((resetAtMs - now) / 1000);
      setRateLimitHeaders(res, maxRequests, 0, resetAtMs);
      res.setHeader("Retry-After", String(retryAfter));

      // Log violation at WARN so operators can set up alerts.
      logger.warn(
        {
          tier: tierLabel,
          key,
          limit: maxRequests,
          windowMs,
          retryAfterSeconds: retryAfter,
          path: req.path,
          method: req.method,
        },
        `Rate limit exceeded [${tierLabel}]: ${key} on ${req.method} ${req.path}`,
      );

      res
        .status(429)
        .json({ error: { message: "Too many requests", code: "RATE_LIMITED" } });
      return;
    }

    win.timestamps.push(now);
    windows.set(key, win);

    // Emit headers on every allowed response so clients can track quota.
    setRateLimitHeaders(res, maxRequests, remaining - 1, resetAtMs);

    next();
  }

  return {
    middleware,
    stop() {
      windows.clear();
    },
    size() {
      return windows.size;
    },
  };
}

// ── Env helpers ───────────────────────────────────────────────────────────────

function readEnvInt(key: string, fallback: number): number {
  const raw = process.env[key];
  if (!raw) return fallback;
  const n = parseInt(raw, 10);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

function readEnvWindowMs(key: string, fallback: number): number {
  const raw = process.env[key];
  if (!raw) return fallback;
  const n = parseInt(raw, 10);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

// ── Per-route-group limiter factories ────────────────────────────────────────
//
// Using factory functions so tests can reset process.env before the limiter
// is instantiated.

/**
 * Public routes: generous limit for read-heavy unauthenticated traffic.
 * 100 req/min per IP (configurable via RATE_LIMIT_PUBLIC_*).
 */
export function createPublicLimiter(): RateLimiter {
  return createRateLimiter(
    {
      windowMs: readEnvWindowMs("RATE_LIMIT_PUBLIC_WINDOW_MS", 60_000),
      maxRequests: readEnvInt("RATE_LIMIT_PUBLIC_MAX_REQUESTS", 100),
      tierLabel: "public",
    },
    false, // keyed by IP
  );
}

/**
 * Authenticated routes: tighter limit keyed per wallet address.
 * 30 req/min per wallet (configurable via RATE_LIMIT_AUTHED_*).
 */
export function createAuthedLimiter(): RateLimiter {
  return createRateLimiter(
    {
      windowMs: readEnvWindowMs("RATE_LIMIT_AUTHED_WINDOW_MS", 60_000),
      maxRequests: readEnvInt("RATE_LIMIT_AUTHED_MAX_REQUESTS", 30),
      tierLabel: "authed",
    },
    true, // keyed by wallet address
  );
}

/**
 * Admin routes: conservative limit for privileged operations.
 * 20 req/min per IP (configurable via RATE_LIMIT_ADMIN_*).
 */
export function createAdminLimiter(): RateLimiter {
  return createRateLimiter(
    {
      windowMs: readEnvWindowMs("RATE_LIMIT_ADMIN_WINDOW_MS", 60_000),
      maxRequests: readEnvInt("RATE_LIMIT_ADMIN_MAX_REQUESTS", 20),
      tierLabel: "admin",
    },
    false, // keyed by IP
  );
}

/**
 * Venice AI proxy routes: strict limit to respect upstream API quotas.
 * 10 req/min per wallet address (configurable via RATE_LIMIT_VENICE_PROXY_*).
 *
 * This limiter applies to any endpoint that ultimately invokes the Venice AI
 * inference API (task creation, agent execution). Keeping this low prevents a
 * single wallet from exhausting the shared Venice quota.
 */
export function createVeniceProxyLimiter(): RateLimiter {
  return createRateLimiter(
    {
      windowMs: readEnvWindowMs("RATE_LIMIT_VENICE_PROXY_WINDOW_MS", 60_000),
      maxRequests: readEnvInt("RATE_LIMIT_VENICE_PROXY_MAX_REQUESTS", 10),
      tierLabel: "venice-proxy",
    },
    true, // keyed by wallet address
  );
}

// ── Module-level singleton instances ─────────────────────────────────────────

/** Public routes (100 req/min per IP). */
export const publicLimiter = createPublicLimiter();

/** Authenticated routes (30 req/min per wallet). */
export const authedLimiter = createAuthedLimiter();

/** Admin routes (20 req/min per IP). */
export const adminLimiter = createAdminLimiter();

/**
 * Venice AI proxy routes (10 req/min per wallet).
 * Apply this in front of any route that triggers a Venice AI call.
 */
export const veniceProxyLimiter = createVeniceProxyLimiter();

// ── Legacy named exports (kept for backward compatibility) ───────────────────

import { getConfig } from "../../config";

/**
 * Default rate limiter used by POST /api/tasks (legacy; prefer authedLimiter).
 * Configured via RATE_LIMIT_WINDOW_MS / RATE_LIMIT_MAX_REQUESTS.
 */
let defaultLimiter: RateLimiter | null = null;

function getDefaultLimiter(): RateLimiter {
  if (!defaultLimiter) {
    const config = getConfig();
    defaultLimiter = createRateLimiter({
      windowMs: config.RATE_LIMIT_WINDOW_MS,
      maxRequests: config.RATE_LIMIT_MAX_REQUESTS,
      tierLabel: "default",
    });
  }
  return defaultLimiter;
}

export const rateLimitMiddleware = (
  req: Request,
  res: Response,
  next: NextFunction,
): void => getDefaultLimiter().middleware(req, res, next);

/**
 * Stricter rate limiter used by POST /api/agents/register.
 * 10 req/min per IP — registration is an expensive operation.
 */
let registerLimiter: RateLimiter | null = null;

function getRegisterLimiter(): RateLimiter {
  if (!registerLimiter) {
    const config = getConfig();
    registerLimiter = createRateLimiter({
      windowMs: config.RATE_LIMIT_WINDOW_MS,
      maxRequests: config.REGISTER_RATE_LIMIT_MAX_REQUESTS,
      tierLabel: "register",
    });
  }
  return registerLimiter;
}

export const registerRateLimitMiddleware = (
  req: Request,
  res: Response,
  next: NextFunction,
): void => getRegisterLimiter().middleware(req, res, next);

/**
 * Rate limiter used by POST /api/agents/:id/heartbeat.
 * 60 req/min per IP to allow periodic agent pings while preventing abuse.
 */
const heartbeatLimiter = createRateLimiter({
  windowMs: 60_000,
  maxRequests: 60,
  tierLabel: "heartbeat",
});
export const heartbeatRateLimitMiddleware = heartbeatLimiter.middleware;
