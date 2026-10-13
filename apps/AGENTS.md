# Shell Lane — `apps/` and `packages/`

Applies in addition to the root `AGENTS.md`. Work here should never require editing Rust. If a feature needs an engine change, stop and name the missing `openplush-pipeline` API.

## Rules
- Talk to the engine only through `@openplush/engine-client` (`packages/engine-client`). Never import the wasm package from `apps/`.
- The UI thread never runs geometry. Engine calls go through the worker: await results, show progress, support cancel.
- Geometry crosses the worker boundary as transferable typed arrays. Never JSON-encode vertex data.
- TypeScript `strict`. No `any`; no `@ts-ignore` without a linked issue.
- Three.js: dispose geometries, materials, and textures when replaced or unmounted. One renderer per canvas.
- Seam drawing snaps to mesh edges via `three-mesh-bvh` raycasts. Engine seams are edge flags, so free-floating curves are invalid input.
- Heatmaps use viridis or cividis with a numeric legend. Never encode meaning by red/green alone.
- Everything outside the 3D viewport is keyboard-operable with visible focus states.
- No runtime network: no CDN fonts or scripts, no analytics, no remote error reporting. Bundle all assets.
- Display units follow the user's locale setting (mm/cm or in); internal values are always mm.

## Tests
- Vitest for logic.
- Playwright smoke runs the capsule, cube, and sphere fixtures end to end (import → seams → export) and asserts the exported scale square is 50 mm.
- Don't mock the engine in end-to-end tests.
