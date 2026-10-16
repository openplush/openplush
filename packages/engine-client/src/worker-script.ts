import { engineCrashed } from "./errors";
import type { Request } from "./generated/Request";
import type { Response } from "./generated/Response";
import type { WorkerMessage } from "./port";

/** Sends one response to the shell. */
export type Reply = (message: Response<unknown>) => void;

/** Hands a request to the engine's exports, which answer through `reply`. */
export type Dispatch = (request: Request, reply: Reply) => void;

/** What `serve` needs of the worker's global scope. */
export type WorkerScope = Pick<EventTarget, "addEventListener" | "removeEventListener"> & {
  postMessage(message: WorkerMessage): void;
};

/**
 * Answers the shell's requests from `scope` with `dispatch`. Any exception from
 * `dispatch`, a trap included, is a crash: the request gets `EngineCrashed`, the
 * shell gets `{ crashed: true }`, and nothing after it is handled or posted, since a
 * trapped WASM instance must never be called again.
 */
export function serve(scope: WorkerScope, dispatch: Dispatch): void {
  let crashed = false;
  const reply: Reply = (message) => {
    if (!crashed) scope.postMessage(message);
  };
  const onMessage = (event: Event): void => {
    const request = (event as MessageEvent<Request>).data;
    try {
      dispatch(request, reply);
    } catch (error) {
      crashed = true;
      scope.removeEventListener("message", onMessage);
      console.error(error);
      scope.postMessage({ id: request.id, ok: false, error: engineCrashed });
      scope.postMessage({ crashed: true });
    }
  };
  scope.addEventListener("message", onMessage);
}
