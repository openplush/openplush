import type { Request } from "./generated/Request";
import type { WorkerMessage, WorkerPort } from "./port";

/**
 * A worker that answers nothing by itself: tests read what it `received` and
 * `send` its replies, which arrive asynchronously, as from a real worker.
 */
export class MockWorker implements WorkerPort {
  readonly received: Request[] = [];
  /** What listeners threw, in delivery order. */
  readonly errors: unknown[] = [];
  #onMessage: ((message: WorkerMessage) => void) | undefined;
  #pending: Promise<void> = Promise.resolve();

  post(request: Request): void {
    this.received.push(request);
  }

  listen(onMessage: (message: WorkerMessage) => void): () => void {
    this.#onMessage = onMessage;
    return () => {
      this.#onMessage = undefined;
    };
  }

  /**
   * Delivers `message` after the current task, in order with earlier sends. A listener
   * that throws fails only this delivery, as with a worker's `message` event; the
   * error lands in `errors`.
   */
  send(message: WorkerMessage): void {
    this.#pending = this.#pending
      .then(() => this.#onMessage?.(message))
      .catch((error: unknown) => {
        this.errors.push(error);
      });
  }

  /** Resolves once every message sent so far has been delivered. */
  drained(): Promise<void> {
    return this.#pending;
  }
}
