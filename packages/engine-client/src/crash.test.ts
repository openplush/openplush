import { describe, expect, it } from "vitest";

import type { EngineClient } from "./client";
import type { EngineErrorDto, Issued, MeshHandle, Op, Outcome, Progress } from "./index";
import type { MockWorker } from "./mock-worker";
import { nth, only, setup } from "./test-setup";

const op: Op = { op: "cancel", payload: { target: 99 } };
const mesh = { n: 7 } as MeshHandle;

function code(outcome: Outcome<unknown>): string | undefined {
  return outcome.ok ? undefined : outcome.error.code;
}

describe("EngineClient on a worker crash", () => {
  it("answers every pending request with EngineCrashed, once", async () => {
    const { client, workers } = setup();
    const calls = [client.request(op), client.request(op), client.request(op)];
    const worker = only(workers);
    const [first, second] = calls;
    if (first === undefined || second === undefined) throw new Error("no calls");
    const crashed: EngineErrorDto = { code: "EngineCrashed", message: "trapped" };

    worker.send({ id: first.id, ok: false, error: crashed });
    worker.send({ crashed: true });
    worker.send({ id: second.id, ok: true, result: "too late", warnings: [] });
    const outcomes = await Promise.all(calls.map((c) => c.outcome));
    expect(outcomes.map(code)).toEqual(["EngineCrashed", "EngineCrashed", "EngineCrashed"]);
    expect(outcomes[0]).toEqual({ ok: false, error: crashed });
  });

  it("treats the worker's error event as a crash", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    only(workers).fail();
    expect(code(await call.outcome)).toBe("EngineCrashed");
  });

  it("detaches from and terminates the crashed worker", async () => {
    const { client, workers } = setup();
    const seen: Progress[] = [];
    client.request(op, { onProgress: (p) => seen.push(p) });
    const worker = only(workers);
    worker.send({ crashed: true });
    worker.send({ id: 0, progress: { kind: "done" } });
    await worker.drained();
    expect(worker.terminated).toBe(true);
    expect(worker.listening).toBe(false);
    expect(seen).toEqual([]);
  });

  it("fires restarted once with the new generation, even after both signals", async () => {
    const { client, workers } = setup();
    const generations: number[] = [];
    client.onRestarted(({ generation }) => generations.push(generation));
    client.request(op);
    const worker = only(workers);
    worker.send({ crashed: true });
    worker.fail();
    await worker.drained();
    expect(generations).toEqual([1]);
  });

  it("stops calling a restarted listener once unsubscribed", async () => {
    const { client, workers } = setup();
    const generations: number[] = [];
    const unsubscribe = client.onRestarted(({ generation }) => generations.push(generation));
    unsubscribe();
    client.request(op);
    only(workers).send({ crashed: true });
    await nth(workers, 0).drained();
    expect(generations).toEqual([]);
  });

  it("treats a cancel racing the crash as a no-op", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    const worker = only(workers);
    worker.send({ crashed: true });
    await call.outcome;
    call.cancel();
    expect(workers).toHaveLength(1);
    expect(worker.received).toHaveLength(1);
  });

  it("runs the next request on a new worker", async () => {
    const { client, workers } = setup();
    const before = client.request(op);
    only(workers).send({ crashed: true });
    await before.outcome;

    const after = client.request(op);
    expect(workers).toHaveLength(2);
    const fresh = nth(workers, 1);
    expect(fresh.received.map((r) => r.id)).toEqual([after.id]);
    fresh.send({ id: after.id, ok: true, result: "fresh", warnings: [] });
    await expect(after.outcome).resolves.toEqual({ ok: true, result: "fresh", warnings: [], generation: 1 });
  });
});

describe("EngineClient handles", () => {
  async function issuedMesh(client: EngineClient, worker: MockWorker): Promise<Issued<MeshHandle>> {
    const call = client.request<MeshHandle>(op);
    worker.send({ id: call.id, ok: true, result: mesh, warnings: [] });
    const outcome = await call.outcome;
    if (!outcome.ok) throw new Error("no mesh");
    return { generation: outcome.generation, handle: outcome.result };
  }

  it("sends a request whose handles are from this generation", async () => {
    const { client, workers } = setup();
    client.request(op);
    const worker = only(workers);
    const issued = await issuedMesh(client, worker);
    expect(issued).toEqual({ generation: 0, handle: mesh });
    client.request(op, { uses: [issued] });
    expect(worker.received).toHaveLength(3);
  });

  it("answers StaleHandle for a handle from an earlier generation, posting nothing", async () => {
    const { client, workers } = setup();
    client.request(op);
    const worker = only(workers);
    const issued = await issuedMesh(client, worker);
    worker.send({ crashed: true });
    await worker.drained();

    const stale = client.request(op, { uses: [issued] });
    expect(code(await stale.outcome)).toBe("StaleHandle");
    expect(workers).toHaveLength(1);
  });

  it("stamps a result with the generation that answered it, even after a crash", async () => {
    const { client, workers } = setup();
    const call = client.request<MeshHandle>(op);
    const worker = only(workers);
    worker.send({ id: call.id, ok: true, result: mesh, warnings: [] });
    worker.send({ crashed: true });
    await worker.drained();
    const outcome = await call.outcome;
    expect(outcome.ok && outcome.generation).toBe(0);
  });
});

describe("EngineClient restarted listeners", () => {
  it("runs every listener even when one throws, then reports the throw", async () => {
    const { client, workers } = setup();
    const seen: string[] = [];
    client.onRestarted(() => {
      throw new Error("first");
    });
    client.onRestarted(() => seen.push("second"));
    client.request(op);
    const worker = only(workers);
    worker.send({ crashed: true });
    await worker.drained();
    expect(seen).toEqual(["second"]);
    expect(worker.errors).toEqual([expect.any(AggregateError)]);
  });

  it("does not call a listener added during the same crash", async () => {
    const { client, workers } = setup();
    const late: number[] = [];
    client.onRestarted(() => {
      client.onRestarted(({ generation }) => late.push(generation));
    });
    client.request(op);
    only(workers).send({ crashed: true });
    await nth(workers, 0).drained();
    expect(late).toEqual([]);
  });
});
