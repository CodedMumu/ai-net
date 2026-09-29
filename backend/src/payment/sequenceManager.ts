/**
 * SequenceNumberManager — per-account promise-chain mutex for Stellar submissions.
 *
 * Stellar accounts have a monotonically-incrementing sequence number. When two
 * transactions are built from the same account concurrently they both read the
 * same sequence number and one of them will fail with `tx_bad_seq`. Serialising
 * all submissions for a given public key through a promise chain solves this at
 * zero CPU cost: there are no timers, no polling, and no blocking — each caller
 * simply awaits the previous caller's promise before proceeding.
 *
 * Usage:
 *   const mgr = new SequenceNumberManager();
 *   const result = await mgr.withAccount(publicKey, async () => {
 *     const account = await server.loadAccount(publicKey);
 *     const tx = buildTx(account);
 *     return server.submitTransaction(tx);
 *   });
 */
export class SequenceNumberManager {
  /**
   * Tracks the tail of each account's execution queue.
   * Key   : Stellar public key (G…)
   * Value : promise that resolves when the current head of the queue finishes.
   */
  private locks = new Map<string, Promise<void>>();

  /**
   * Enqueue `fn` for `publicKey`.  `fn` will not start until every previously
   * enqueued function for the same key has settled.
   *
   * @param publicKey  Stellar public key identifying the source account.
   * @param fn         Async work to serialize (load account → build → submit).
   * @returns          Whatever `fn` resolves with.
   */
  async withAccount<T>(publicKey: string, fn: () => Promise<T>): Promise<T> {
    // Grab whatever is currently at the tail of this account's queue, or an
    // already-resolved promise if the queue is empty.
    const current = this.locks.get(publicKey) ?? Promise.resolve();

    // Create the next tail: a promise whose resolver we'll call when fn settles.
    let resolve!: () => void;
    const next = new Promise<void>((r) => {
      resolve = r;
    });

    // Publish our slot as the new tail so the next caller will wait for us.
    this.locks.set(publicKey, next);

    // Wait for everything that was enqueued before us.
    await current;

    try {
      return await fn();
    } finally {
      // Signal the next waiter that it may proceed.
      resolve();

      // Clean up the map entry if nothing else has enqueued behind us (i.e. we
      // are still the tail).  This prevents unbounded map growth during idle
      // periods while keeping the map alive as long as there are waiters.
      if (this.locks.get(publicKey) === next) {
        this.locks.delete(publicKey);
      }
    }
  }
}
