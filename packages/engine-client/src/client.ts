import type { EngineErrorDto } from "./generated/EngineErrorDto";
import type { Op } from "./generated/Op";
import type { Progress } from "./generated/Progress";
import type { RequestId } from "./generated/RequestId";
import type { WarningDto } from "./generated/WarningDto";
import type { WorkerMessage, WorkerPort } from "./port";

/** How a request ended: exactly one per request. */
export type Outcome<T> =
  | { ok: true; result: T; warnings: readonly WarningDto[] }
  | { ok: false; error: EngineErrorDto };

/** A request in flight. */
export interface Call<T> {
  readonly id: RequestId;
  readonly outcome: Promise<Outcome<T>>;
  /** Asks the worker to stop the request; does nothing once its outcome is settled. */
  cancel(): void;
}

type Pending = {
  settle: (outcome: Outcome<unknown>) => void;
  onProgress: ((progress: Progress) => void) | undefined;
};

// Request ids are u32 on the wire.
const MAX_ID = 0xffff_ffff;

/** Sends requests to an engine worker, spawned on first use, and settles each one once. */
export class EngineClient {
  readonly #spawn: () => WorkerPort;
  readonly #pending = new Map<RequestId, Pending>();
  #port: WorkerPort | undefined;
  #nextId = 0;

  constructor(spawn: () => WorkerPort) {
    this.#spawn = spawn;
  }

  /**
   * Sends `op`. The worker decides the result's shape; `T` states the caller's
   * reading of it, which this client cannot check.
   */
  request<T>(op: Op, onProgress?: (progress: Progress) => void): Call<T> {
    const id = this.#allocate();
    // Posted before it is pending, so a post that throws leaves nothing to settle;
    // no reply can arrive before this method returns.
    this.#worker().post({ id, ...op });
    const outcome = new Promise<Outcome<T>>((resolve) => {
      const settle = resolve as (outcome: Outcome<unknown>) => void;
      this.#pending.set(id, { settle, onProgress });
    });
    const cancel = (): void => {
      if (!this.#pending.has(id)) return;
      this.#worker().post({ id: this.#allocate(), op: "cancel", payload: { target: id } });
    };
    return { id, outcome, cancel };
  }

  #allocate(): RequestId {
    if (this.#nextId > MAX_ID) throw new RangeError("engine-client: request ids exhausted");
    return this.#nextId++;
  }

  #worker(): WorkerPort {
    if (this.#port === undefined) {
      const port = this.#spawn();
      port.listen((message) => this.#receive(message));
      this.#port = port;
    }
    return this.#port;
  }

  #receive(message: WorkerMessage): void {
    const { id } = message;
    const pending = this.#pending.get(id);
    if (pending === undefined) return;
    if ("progress" in message) {
      pending.onProgress?.(message.progress);
      return;
    }
    if (message.ok === true) {
      this.#pending.delete(id);
      pending.settle({ ok: true, result: message.result, warnings: message.warnings });
    } else if (message.ok === false) {
      this.#pending.delete(id);
      pending.settle({ ok: false, error: message.error });
    } else {
      throw new TypeError(`engine-client: response ${id} has no progress and no ok`);
    }
  }
}
