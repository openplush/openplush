import { engineCrashed } from "./errors";
import type { Request } from "./generated/Request";
import type { RequestId } from "./generated/RequestId";
import type { Response } from "./generated/Response";
import type { WorkerMessage } from "./port";

/** Sends one response to the shell. */
export type Reply = (message: Response<unknown>) => void;

/**
 * Hands a request to the engine's exports, which answer through `reply`. Returns
 * `undefined`, not a promise: a rejection would escape the crash rule.
 */
export type Dispatch = (request: Request, reply: Reply) => undefined;

/** What `serve` needs of the worker's global scope. */
export type WorkerScope = Pick<EventTarget, "addEventListener" | "removeEventListener"> & {
  postMessage(message: WorkerMessage): void;
};

/**
 * Answers the shell's requests from `scope` with `dispatch`. Any exception from
 * `dispatch` or from posting a reply, a trap included, is a crash: the request gets
 * `EngineCrashed`, the shell gets `{ crashed: true }`, and nothing after it is handled
 * or posted, since a trapped WASM instance must never be called again.
 */
export function serve(scope: WorkerScope, dispatch: Dispatch): void {
  let crashed = false;
  const crash = (id: RequestId, error: unknown): void => {
    if (crashed) return;
    crashed = true;
    scope.removeEventListener("message", onMessage);
    console.error(error);
    scope.postMessage({ id, ok: false, error: engineCrashed });
    scope.postMessage({ crashed: true });
  };
  const onMessage = (event: Event): void => {
    const { id } = (event as MessageEvent<Request>).data;
    // Built per request, so a reply that fails after `dispatch` returns still crashes for its own id.
    const reply: Reply = (message) => {
      if (crashed) return;
      try {
        scope.postMessage(message);
      } catch (error) {
        crash(id, error);
      }
    };
    try {
      dispatch((event as MessageEvent<Request>).data, reply);
    } catch (error) {
      crash(id, error);
    }
  };
  scope.addEventListener("message", onMessage);
}
