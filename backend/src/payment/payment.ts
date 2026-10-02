import {
  Keypair,
  Operation,
  Asset,
  Claimant,
} from "@stellar/stellar-sdk";
import type { PaymentDb, PaymentRecord } from "../db/index";
import {
  PaymentAlreadyReleasedError,
  xlmToStroops,
  stroopsToXlm,
} from "./utils";
import { tracingService } from "../services/tracing";
import { currentTraceId } from "../services/traceContext";
import { StellarService } from "../services/stellarService";

export interface PaymentServiceHooks {
  /**
   * Invoked after a payment is successfully released on-chain so callers can
   * trigger a reconciliation check (e.g. `ReconciliationService.run('release')`).
   */
  reconciliationHook?: (record: PaymentRecord) => void;
}

export class PaymentService {
  private readonly stellarService: StellarService;

  constructor(
    private db: PaymentDb,
    private hooks: PaymentServiceHooks = {},
    stellarService?: StellarService,
  ) {
    this.stellarService = stellarService ?? new StellarService();
  }

  /**
   * Enumerate all local payment records — the reconciliation source of truth.
   */
  listLocalRecords(): PaymentRecord[] {
    return this.db.listAll();
  }

  async lock(
    taskId: string,
    nodeId: string,
    coordinatorKeypair: Keypair,
    agentPublicKey: string,
    amountXLM: number,
    correlationId?: string
  ): Promise<string> {
    const traceId = correlationId ?? currentTraceId();
    const span = traceId
      ? tracingService.startSpan(traceId, 'payment', 'lock', { taskId, nodeId, amountXLM })
      : null;

    try {
      const amountStroops = xlmToStroops(amountXLM);
      const amountStr = stroopsToXlm(amountStroops);

      const tx = await this.stellarService.buildTransaction(coordinatorKeypair, [
          Operation.createClaimableBalance({
            asset: Asset.native(),
            amount: amountStr,
            claimants: [
              new Claimant(agentPublicKey, Claimant.predicateUnconditional()),
              new Claimant(coordinatorKeypair.publicKey(), Claimant.predicateUnconditional()),
            ],
          }),
        ]);

      // Derive balance ID before signing — deterministic from the operation
      const balanceId = tx.getClaimableBalanceId(0);

      await this.stellarService.submitTransaction(tx.toXDR());

      this.db.insert({
        taskId,
        nodeId,
        balanceId,
        status: "locked",
        amountStroops,
        txHash: null,
      });

      if (span) tracingService.endSpan(span.spanId, 'completed', { balanceId });
      return balanceId;
    } catch (err) {
      if (span) tracingService.endSpan(span.spanId, 'failed', { error: String(err) });
      throw err;
    }
  }

  async release(
    taskId: string,
    nodeId: string,
    coordinatorKeypair: Keypair,
    correlationId?: string
  ): Promise<string> {
    const record = this.db.findByKey(taskId, nodeId);
    if (!record) throw new Error(`No payment record for task=${taskId} node=${nodeId}`);

    // Idempotency: return existing hash without a second Stellar tx
    if (record.status === "released" && record.txHash) {
      return record.txHash;
    }

    const traceId = correlationId ?? currentTraceId();
    const span = traceId
      ? tracingService.startSpan(traceId, 'payment', 'release', { taskId, nodeId })
      : null;

    try {
      const tx = await this.stellarService.buildTransaction(coordinatorKeypair, [
        Operation.claimClaimableBalance({ balanceId: record.balanceId }),
      ]);

      const { hash: txHash } = await this.stellarService.submitTransaction(tx.toXDR());

      this.db.updateStatus(taskId, nodeId, "released", txHash);

      this.hooks.reconciliationHook?.({
        ...record,
        status: "released",
        txHash,
      });

      if (span) tracingService.endSpan(span.spanId, 'completed', { txHash });
      return txHash;
    } catch (err) {
      if (span) tracingService.endSpan(span.spanId, 'failed', { error: String(err) });
      throw err;
    }
  }

  async refund(
    taskId: string,
    nodeId: string,
    coordinatorKeypair: Keypair
  ): Promise<string> {
    const record = this.db.findByKey(taskId, nodeId);
    if (!record) throw new Error(`No payment record for task=${taskId} node=${nodeId}`);

    if (record.status === "released") {
      throw new PaymentAlreadyReleasedError(taskId, nodeId);
    }

    const tx = await this.stellarService.buildTransaction(coordinatorKeypair, [
      Operation.claimClaimableBalance({ balanceId: record.balanceId }),
    ]);

    const { hash: txHash } = await this.stellarService.submitTransaction(tx.toXDR());

    this.db.updateStatus(taskId, nodeId, "refunded", txHash);
    return txHash;
  }

  getPaymentStatus(taskId: string, nodeId: string): PaymentRecord | undefined {
    return this.db.findByKey(taskId, nodeId);
  }
}
