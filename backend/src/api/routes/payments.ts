import { Router, type Request, type Response } from "express";
import { getDb, createPaymentDb } from "../../db/index";
import { createTaskDb, getTaskDb } from "../../db/tasks";
import type { Task } from "../../types/task";
import { createLogger } from "../../utils/logger";

const router = Router();
const logger = createLogger({ component: "payments-route" });

/**
 * GET /api/payments
 *
 * Returns all payment records from the local payments.db.
 * Records are serialized with amountStroops as a string to avoid
 * JSON bigint precision issues.
 */
router.get("/", (req: Request, res: Response) => {
  const walletPublicKey = req.header("walletpublickey")?.trim();
  if (!walletPublicKey) {
    res.status(400).json({
      error: { message: "Wallet public key is required", code: "WALLET_REQUIRED" },
    });
    return;
  }

  try {
    const tasksDb = createTaskDb(getTaskDb());
    const taskById = new Map<string, Task>();
    let cursor: string | undefined;

    do {
      const page = tasksDb.listCursor(walletPublicKey, { cursor, limit: 100 });
      for (const task of page.items) taskById.set(task.id, task);
      cursor = page.nextCursor;
    } while (cursor);

    const payments = createPaymentDb(getDb())
      .listAll()
      .filter((payment) => taskById.has(payment.taskId))
      .map((p) => {
        const task = taskById.get(p.taskId);
        if (!task) throw new Error(`Missing task ownership record for payment ${p.taskId}`);
        return {
          taskId: p.taskId,
          nodeId: p.nodeId,
          balanceId: p.balanceId,
          status: p.status,
          amountStroops: p.amountStroops.toString(),
          txHash: p.txHash,
          date: p.status === "locked" ? task.createdAt : task.updatedAt,
        };
      });
    res.json({ payments });
  } catch (err) {
    logger.error({ err }, "Failed to retrieve payments");
    res.status(500).json({
      error: { message: "Failed to retrieve payments", code: "PAYMENT_DB_ERROR" },
    });
  }
});

export { router as paymentsRouter };
