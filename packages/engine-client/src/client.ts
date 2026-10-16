import { engineCrashed, staleHandle } from "./errors";
import type { EngineErrorDto } from "./generated/EngineErrorDto";
import type { MeshHandle } from "./generated/MeshHandle";
import type { Op } from "./generated/Op";
import type { PipelineHandle } from "./generated/PipelineHandle";
import type { Progress } from "./generated/Progress";
import type { RequestId } from "./generated/RequestId";
import type { WarningDto } from "./generated/WarningDto";
import type { WorkerMessage, WorkerPort } from "./port";

/** How a request ended: exactly one per request. */
export type Outcome<T> =
  | { ok: true; result: T; warnings: readonly WarningDto[]; generation: number }
  | { ok: false; error: EngineErrorDto };

/** A request in flight. */
export interface Call<T> {
  readonly id: RequestId;
  readonly outcome: Promise<Outcome<T>>;
  /** Asks the worker to stop the request; does nothing once its outcome is settled. */
  cancel(): void;
}

/**
 * A wire handle with the worker generation that issued it: a success's `generation`,
 * never the current one, which a crash may already have moved past.
 */
export type Issued<H extends MeshHandle | PipelineHandle> = {
  readonly generation: number;
  readonly handle: H;
};

export type RequestOptions = {
  onProgress?: (progress: Progress) => void;
  /** The handles the request refers to; one from an earlier generation is `StaleHandle`. */
  uses?: readonly Issued<MeshHandle | PipelineHandle>[];
};

type Pending = {
  settle: (outcome: Outcome<unknown>) => void;
  onProgress: ((progress: Progress) => void) | undefined;
  generation: number;
};

type Connection = { port: WorkerPort; detach: () => void };

// Request ids are u32 on the wire.
const MAX_ID = 0xffff_ffff;

/**
 * Sends requests to an engine worker, spawned on first use, and settles each one once.
 * A crashed worker is dropped; the next request spawns a new one in a new generation.
 */
export class EngineClient {
  readonly #spawn: () => WorkerPort;
  readonly #pending = new Map<RequestId, Pending>();
  readonly #restarted = new Set<(event: { generation: number }) => void>();
  #connection: Connection | undefined;
  #generation = 0;
  #nextId = 0;

  constructor(spawn: () => WorkerPort) {
    this.#spawn = spawn;
  }

  /** Calls `listener` after each crash, with the new generation; returns an unsubscribe. */
  onRestarted(listener: (event: { generation: number }) => void): () => void {
    this.#restarted.add(listener);
    return () => this.#restarted.delete(listener);
  }

  /**
   * Sends `op`. The worker decides the result's shape; `T` states the caller's
   * reading of it, which this client cannot check.
   */
  request<T>(op: Op, options: RequestOptions = {}): Call<T> {
    const id = this.#allocate();
    const cancel = (): void => {
      if (!this.#pending.has(id)) return;
      this.#port().post({ id: this.#allocate(), op: "cancel", payload: { target: id } });
    };
    if (options.uses?.some((h) => h.generation !== this.#generation)) {
      return { id, outcome: Promise.resolve({ ok: false, error: staleHandle }), cancel };
    }
    // Posted before it is pending, so a post that throws leaves nothing to settle;
    // no reply can arrive before this method returns.
    this.#port().post({ id, ...op });
    const outcome = new Promise<Outcome<T>>((resolve) => {
      const settle = resolve as (outcome: Outcome<unknown>) => void;
      const generation = this.#generation;
      this.#pending.set(id, { settle, onProgress: options.onProgress, generation });
    });
    return { id, outcome, cancel };
  }

  #allocate(): RequestId {
    if (this.#nextId > MAX_ID) throw new RangeError("engine-client: request ids exhausted");
    return this.#nextId++;
  }

  #port(): WorkerPort {
    if (this.#connection === undefined) {
      const port = this.#spawn();
      const detach = port.listen(
        (message) => this.#receive(message),
        () => this.#crash(),
      );
      this.#connection = { port, detach };
    }
    return this.#connection.port;
  }

  #receive(message: WorkerMessage): void {
    if ("crashed" in message) {
      this.#crash();
      return;
    }
    const { id } = message;
    const pending = this.#pending.get(id);
    if (pending === undefined) return;
    if ("progress" in message) {
      pending.onProgress?.(message.progress);
      return;
    }
    if (message.ok === true) {
      this.#pending.delete(id);
      const { result, warnings } = message;
      pending.settle({ ok: true, result, warnings, generation: pending.generation });
    } else if (message.ok === false) {
      this.#pending.delete(id);
      pending.settle({ ok: false, error: message.error });
    } else {
      throw new TypeError(`engine-client: response ${id} has no progress and no ok`);
    }
  }

  // Detaching first means the dead worker's later messages, and a second crash
  // signal from it, never reach this client.
  #crash(): void {
    const connection = this.#connection;
    if (connection === undefined) return;
    connection.detach();
    connection.port.terminate();
    this.#connection = undefined;
    const pending = [...this.#pending.values()];
    this.#pending.clear();
    for (const p of pending) p.settle({ ok: false, error: engineCrashed });
    this.#generation += 1;
    this.#emitRestarted({ generation: this.#generation });
  }

  // Every listener runs even if one throws; a listener added meanwhile waits for the next crash.
  #emitRestarted(event: { generation: number }): void {
    const failures: unknown[] = [];
    for (const listener of [...this.#restarted]) {
      try {
        listener(event);
      } catch (error) {
        failures.push(error);
      }
    }
    if (failures.length > 0) throw new AggregateError(failures, "engine-client: onRestarted");
  }
}
