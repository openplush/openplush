import type { Request } from "./generated/Request";
import type { Response } from "./generated/Response";

/** Anything a worker sends. */
export type WorkerMessage = Response<unknown>;

/** The client's view of a worker; a real `Worker` and the mock both fit behind it. */
export interface WorkerPort {
  post(request: Request): void;
  /** Starts delivering messages; the returned function stops it. */
  listen(onMessage: (message: WorkerMessage) => void): () => void;
}
