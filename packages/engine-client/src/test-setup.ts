import { expect } from "vitest";

import { EngineClient } from "./client";
import { MockWorker } from "./mock-worker";

/** A client whose every spawn is a fresh `MockWorker`, kept in `workers`. */
export function setup(): { client: EngineClient; workers: MockWorker[] } {
  const workers: MockWorker[] = [];
  const client = new EngineClient(() => {
    const worker = new MockWorker();
    workers.push(worker);
    return worker;
  });
  return { client, workers };
}

/** The `index`th worker spawned, which must exist. */
export function nth(workers: readonly MockWorker[], index: number): MockWorker {
  const worker = workers[index];
  if (worker === undefined) throw new Error(`worker ${index} was never spawned`);
  return worker;
}

/** The one worker spawned. */
export function only(workers: readonly MockWorker[]): MockWorker {
  expect(workers).toHaveLength(1);
  return nth(workers, 0);
}
