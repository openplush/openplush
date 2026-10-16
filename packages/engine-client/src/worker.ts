import type { WorkerMessage, WorkerPort } from "./port";

/** What `fromWorker` needs of a `Worker`. */
export type WorkerLike = Pick<
  Worker,
  "postMessage" | "addEventListener" | "removeEventListener" | "terminate"
>;

/**
 * A `Worker` behind the client's port. `messageerror` counts as an error: a message
 * that could not be deserialized may have been a request's only terminal message.
 */
export function fromWorker(worker: WorkerLike): WorkerPort {
  return {
    post: (request) => worker.postMessage(request),
    listen(onMessage, onError) {
      const message = (event: MessageEvent<WorkerMessage>): void => onMessage(event.data);
      const error = (): void => onError();
      worker.addEventListener("message", message);
      worker.addEventListener("error", error);
      worker.addEventListener("messageerror", error);
      return () => {
        worker.removeEventListener("message", message);
        worker.removeEventListener("error", error);
        worker.removeEventListener("messageerror", error);
      };
    },
    terminate: () => worker.terminate(),
  };
}
