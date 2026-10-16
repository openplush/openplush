import type { Request } from "./generated/Request";
import type { Response } from "./generated/Response";

/**
 * What the worker script sends after the engine traps. The one hand-typed message:
 * the script raises it, not the WASM instance, so no Rust type describes it.
 */
export type Crashed = { crashed: true };

/** Anything a worker sends. */
export type WorkerMessage = Response<unknown> | Crashed;

/** The client's view of a worker; a real `Worker` and the mock both fit behind it. */
export interface WorkerPort {
  post(request: Request): void;
  /**
   * Starts delivering messages, and `onError` for the worker's `error` event;
   * the returned function stops both.
   */
  listen(onMessage: (message: WorkerMessage) => void, onError: () => void): () => void;
  terminate(): void;
}
