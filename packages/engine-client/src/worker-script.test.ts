import { describe, expect, it, vi } from "vitest";

import { EngineClient } from "./client";
import type { Request } from "./generated/Request";
import type { Response } from "./generated/Response";
import type { Op } from "./index";
import { fromWorker } from "./worker";
import { type Dispatch, serve } from "./worker-script";

class FakeScope extends EventTarget {
  readonly posted: unknown[] = [];

  // Clones as a real postMessage does, so an uncloneable message throws DataCloneError.
  postMessage(message: unknown): void {
    this.posted.push(structuredClone(message));
  }

  receive(request: Request): void {
    this.dispatchEvent(new MessageEvent("message", { data: request }));
  }
}

const op: Op = { op: "cancel", payload: { target: 0 } };
const request = (id: number): Request => ({ id, ...op });
const crashedFor = (id: number): unknown => ({
  id,
  ok: false,
  error: { code: "EngineCrashed", message: expect.any(String) as unknown },
});

describe("serve", () => {
  it("dispatches each request and posts its replies in order", () => {
    const scope = new FakeScope();
    serve(scope, (req, reply) => {
      reply({ id: req.id, progress: { kind: "done" } });
      reply({ id: req.id, ok: true, result: null, warnings: [] });
    });
    scope.receive(request(1));
    expect(scope.posted).toEqual([
      { id: 1, progress: { kind: "done" } },
      { id: 1, ok: true, result: null, warnings: [] },
    ]);
  });

  it("turns an exception from the engine into EngineCrashed, then crashed", () => {
    const scope = new FakeScope();
    serve(scope, () => {
      throw new WebAssembly.RuntimeError("unreachable");
    });
    scope.receive(request(4));
    expect(scope.posted).toEqual([crashedFor(4), { crashed: true }]);
  });

  it("handles nothing more after a crash", () => {
    const scope = new FakeScope();
    let calls = 0;
    serve(scope, () => {
      calls += 1;
      throw new Error("trap");
    });
    scope.receive(request(1));
    scope.receive(request(2));
    expect(calls).toBe(1);
    expect(scope.posted).toHaveLength(2);
  });

  it("drops replies to an earlier request once crashed", () => {
    const scope = new FakeScope();
    const replies: ((message: Response<unknown>) => void)[] = [];
    serve(scope, (req, reply) => {
      replies.push(reply);
      if (req.id === 2) throw new Error("trap");
    });
    scope.receive(request(1));
    scope.receive(request(2));
    replies[0]?.({ id: 1, ok: true, result: null, warnings: [] });
    expect(scope.posted).toEqual([crashedFor(2), { crashed: true }]);
  });

  it("treats a reply that cannot be posted as a crash", () => {
    const scope = new FakeScope();
    serve(scope, (req, reply) => {
      reply({ id: req.id, ok: true, result: () => {}, warnings: [] });
    });
    scope.receive(request(5));
    expect(scope.posted).toEqual([crashedFor(5), { crashed: true }]);
  });

  it("treats a reply that cannot be posted after dispatch returned as a crash", () => {
    const scope = new FakeScope();
    let late: ((message: Response<unknown>) => void) | undefined;
    let calls = 0;
    serve(scope, (req, reply) => {
      calls += 1;
      if (req.id === 2) late = reply;
    });
    scope.receive(request(2));
    late?.({ id: 2, ok: true, result: () => {}, warnings: [] });
    scope.receive(request(3));
    expect(scope.posted).toEqual([crashedFor(2), { crashed: true }]);
    expect(calls).toBe(1);
  });

  it("logs the exception to the worker console", () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    const trap = new WebAssembly.RuntimeError("unreachable");
    const scope = new FakeScope();
    serve(scope, () => {
      throw trap;
    });
    scope.receive(request(1));
    expect(log).toHaveBeenCalledWith(trap);
    log.mockRestore();
  });

  it("does not accept an async dispatch, whose rejection no crash path would see", () => {
    // @ts-expect-error a promise-returning dispatch
    const dispatch: Dispatch = async () => {};
    expect(dispatch).toBeTypeOf("function");
  });
});

// A worker and its scope, each delivering to the other after the current task.
function pair(dispatch: Dispatch): { client: EngineClient; spawned: () => number } {
  let spawned = 0;
  const client = new EngineClient(() => {
    spawned += 1;
    const host = new EventTarget();
    const scope = new EventTarget();
    const deliver = (target: EventTarget, data: unknown): void =>
      queueMicrotask(() => target.dispatchEvent(new MessageEvent("message", { data })));
    serve({ ...on(scope), postMessage: (m) => deliver(host, m) }, dispatch);
    return fromWorker({ ...on(host), postMessage: (m: unknown) => deliver(scope, m), terminate: () => {} });
  });
  return { client, spawned: () => spawned };
}

function on(target: EventTarget): Pick<EventTarget, "addEventListener" | "removeEventListener"> {
  return {
    addEventListener: target.addEventListener.bind(target),
    removeEventListener: target.removeEventListener.bind(target),
  };
}

describe("EngineClient over a served worker", () => {
  it("settles a trapped request with EngineCrashed and restarts once", async () => {
    const { client, spawned } = pair((req, reply) => {
      if (req.id === 0) throw new WebAssembly.RuntimeError("unreachable");
      reply({ id: req.id, ok: true, result: "fine", warnings: [] });
    });
    const generations: number[] = [];
    client.onRestarted(({ generation }) => generations.push(generation));

    const trapped = await client.request(op).outcome;
    expect(trapped).toMatchObject({ ok: false, error: { code: "EngineCrashed" } });
    expect(generations).toEqual([1]);

    const next = await client.request(op).outcome;
    expect(next).toEqual({ ok: true, result: "fine", warnings: [], generation: 1 });
    expect(spawned()).toBe(2);
  });
});
