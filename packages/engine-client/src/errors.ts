import type { EngineErrorDto } from "./generated/EngineErrorDto";

/** The terminal error of every request a worker crash cut short. */
export const engineCrashed: EngineErrorDto = {
  code: "EngineCrashed",
  message: "The engine stopped unexpectedly, so this action did not finish.",
};

/** The error of a request that names an earlier worker generation's handle. */
export const staleHandle: EngineErrorDto = {
  code: "StaleHandle",
  message: "This model was loaded before the engine restarted. Load it again.",
};
