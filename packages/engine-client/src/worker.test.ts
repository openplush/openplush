import { describe, expect, it } from "vitest";

import type { Request } from "./generated/Request";
import type { WorkerMessage } from "./port";
import { fromWorker } from "./worker";

class FakeWorker extends EventTarget {
  readonly posted: unknown[] = [];
  terminated = false;

  postMessage(message: unknown): void {
    this.posted.push(message);
  }

  terminate(): void {
    this.terminated = true;
  }
}

const request: Request = { id: 3, op: "cancel", payload: { target: 1 } };

function attach(worker: FakeWorker): { messages: WorkerMessage[]; errors: number; detach: () => void } {
  const seen = { messages: [] as WorkerMessage[], errors: 0, detach: () => {} };
  seen.detach = fromWorker(worker).listen(
    (message) => seen.messages.push(message),
    () => (seen.errors += 1),
  );
  return seen;
}

describe("fromWorker", () => {
  it("posts requests to the worker", () => {
    const worker = new FakeWorker();
    fromWorker(worker).post(request);
    expect(worker.posted).toEqual([request]);
  });

  it("delivers each message event's data", () => {
    const worker = new FakeWorker();
    const seen = attach(worker);
    worker.dispatchEvent(new MessageEvent("message", { data: { crashed: true } }));
    expect(seen.messages).toEqual([{ crashed: true }]);
  });

  it("reports the error and messageerror events as errors", () => {
    const worker = new FakeWorker();
    const seen = attach(worker);
    worker.dispatchEvent(new Event("error"));
    worker.dispatchEvent(new MessageEvent("messageerror"));
    expect(seen.errors).toBe(2);
    expect(seen.messages).toEqual([]);
  });

  it("stops delivering every event once detached", () => {
    const worker = new FakeWorker();
    const seen = attach(worker);
    seen.detach();
    worker.dispatchEvent(new MessageEvent("message", { data: { crashed: true } }));
    worker.dispatchEvent(new Event("error"));
    worker.dispatchEvent(new MessageEvent("messageerror"));
    expect(seen.messages).toEqual([]);
    expect(seen.errors).toBe(0);
  });

  it("terminates the worker", () => {
    const worker = new FakeWorker();
    fromWorker(worker).terminate();
    expect(worker.terminated).toBe(true);
  });
});
