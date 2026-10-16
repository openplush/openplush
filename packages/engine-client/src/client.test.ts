import { describe, expect, it } from "vitest";

import { EngineClient } from "./client";
import type { EngineErrorDto, Op, Progress } from "./index";
import { MockWorker } from "./mock-worker";

const op: Op = { op: "cancel", payload: { target: 99 } };
const cancelled: EngineErrorDto = { code: "Cancelled", message: "cancelled" };

function setup(): { client: EngineClient; workers: MockWorker[] } {
  const workers: MockWorker[] = [];
  const client = new EngineClient(() => {
    const worker = new MockWorker();
    workers.push(worker);
    return worker;
  });
  return { client, workers };
}

function only(workers: readonly MockWorker[]): MockWorker {
  expect(workers).toHaveLength(1);
  const [worker] = workers;
  if (worker === undefined) throw new Error("no worker spawned");
  return worker;
}

describe("EngineClient requests", () => {
  it("posts the op with a fresh id and resolves with the result", async () => {
    const { client, workers } = setup();
    const call = client.request<{ charts: number[] }>(op);
    const worker = only(workers);
    expect(worker.received).toEqual([{ id: call.id, ...op }]);

    worker.send({ id: call.id, ok: true, result: { charts: [1, 2] }, warnings: [] });
    await expect(call.outcome).resolves.toEqual({
      ok: true,
      result: { charts: [1, 2] },
      warnings: [],
    });
  });

  it("resolves a failure with its error", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    only(workers).send({ id: call.id, ok: false, error: cancelled });
    await expect(call.outcome).resolves.toEqual({ ok: false, error: cancelled });
  });

  it("gives every request its own increasing id on one worker", () => {
    const { client, workers } = setup();
    const ids = [client.request(op).id, client.request(op).id, client.request(op).id];
    expect(ids).toEqual([0, 1, 2]);
    expect(only(workers).received.map((r) => r.id)).toEqual(ids);
  });

  it("reports progress only before the terminal message", async () => {
    const { client, workers } = setup();
    const seen: Progress[] = [];
    const call = client.request(op, (p) => seen.push(p));
    const worker = only(workers);
    const working: Progress = { kind: "working", chart: 1, iteration: 2, maxDistortion: 0.5 };

    worker.send({ id: call.id, progress: working });
    worker.send({ id: call.id, ok: true, result: null, warnings: [] });
    worker.send({ id: call.id, progress: { kind: "done" } });
    await call.outcome;
    await worker.drained();
    expect(seen).toEqual([working]);
  });

  it("keeps the first terminal message and ignores a second", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    const worker = only(workers);
    worker.send({ id: call.id, ok: false, error: cancelled });
    worker.send({ id: call.id, ok: true, result: 1, warnings: [] });
    await expect(call.outcome).resolves.toEqual({ ok: false, error: cancelled });
  });

  it("throws on a terminal message without ok, settling nothing", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    const worker = only(workers);
    worker.send({ id: call.id } as never);
    worker.send({ id: call.id, ok: true, result: "later", warnings: [] });
    await expect(call.outcome).resolves.toEqual({ ok: true, result: "later", warnings: [] });
    expect(worker.errors).toEqual([expect.any(TypeError)]);
  });

  it("keeps delivering after a progress listener throws", async () => {
    const { client, workers } = setup();
    const first = client.request(op, () => {
      throw new Error("listener");
    });
    const second = client.request(op);
    const worker = only(workers);
    worker.send({ id: first.id, progress: { kind: "done" } });
    worker.send({ id: second.id, ok: true, result: null, warnings: [] });
    await expect(second.outcome).resolves.toEqual({ ok: true, result: null, warnings: [] });
    expect(worker.errors).toEqual([new Error("listener")]);
  });

  it("ignores a message for an id it never sent", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    const worker = only(workers);
    worker.send({ id: call.id + 1000, ok: true, result: "stray", warnings: [] });
    worker.send({ id: call.id, ok: true, result: "mine", warnings: [] });
    await expect(call.outcome).resolves.toEqual({ ok: true, result: "mine", warnings: [] });
  });
});

describe("EngineClient cancel", () => {
  it("posts a cancel for a pending request", () => {
    const { client, workers } = setup();
    const call = client.request(op);
    call.cancel();
    const [, cancel] = only(workers).received;
    expect(cancel).toEqual({
      id: expect.any(Number),
      op: "cancel",
      payload: { target: call.id },
    });
    expect(cancel?.id).not.toBe(call.id);
  });

  it("does nothing once the request has its terminal message", async () => {
    const { client, workers } = setup();
    const call = client.request(op);
    const worker = only(workers);
    worker.send({ id: call.id, ok: true, result: null, warnings: [] });
    await call.outcome;
    call.cancel();
    expect(worker.received).toHaveLength(1);
  });
});
