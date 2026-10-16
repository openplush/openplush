import type { Request } from "./generated/Request";
import type { WorkerMessage, WorkerPort } from "./port";

type Delivery = { message: WorkerMessage } | { error: true };

/**
 * A worker that answers nothing by itself: tests read what it `received` and
 * `send` its replies, which arrive asynchronously, as from a real worker.
 */
export class MockWorker implements WorkerPort {
  readonly received: Request[] = [];
  /** What listeners threw, in delivery order. */
  readonly errors: unknown[] = [];
  terminated = false;
  #onMessage: ((message: WorkerMessage) => void) | undefined;
  #onError: (() => void) | undefined;
  #pending: Promise<void> = Promise.resolve();

  /** Whether a listener is attached. */
  get listening(): boolean {
    return this.#onMessage !== undefined;
  }

  post(request: Request): void {
    if (this.terminated) throw new Error("mock-worker: post after terminate");
    this.received.push(request);
  }

  listen(onMessage: (message: WorkerMessage) => void, onError: () => void): () => void {
    this.#onMessage = onMessage;
    this.#onError = onError;
    return () => {
      this.#onMessage = undefined;
      this.#onError = undefined;
    };
  }

  terminate(): void {
    this.terminated = true;
  }

  /**
   * Delivers `message` after the current task, in order with earlier sends. A listener
   * that throws fails only this delivery, as with a worker's `message` event; the
   * error lands in `errors`.
   */
  send(message: WorkerMessage): void {
    this.#deliver({ message });
  }

  /** Fires the worker's `error` event, in order with sends, as when its script dies. */
  fail(): void {
    this.#deliver({ error: true });
  }

  /** Resolves once every message sent so far has been delivered. */
  drained(): Promise<void> {
    return this.#pending;
  }

  #deliver(delivery: Delivery): void {
    this.#pending = this.#pending
      .then(() => {
        if ("message" in delivery) this.#onMessage?.(delivery.message);
        else this.#onError?.();
      })
      .catch((error: unknown) => {
        this.errors.push(error);
      });
  }
}
