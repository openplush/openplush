# Results: cargo-mutants on openplush-flatten and openplush-pattern

Run 2026-10-23 on `main` at `2ad8d23` (before the flatten oracles of #395 and #396 landed), with cargo-mutants 27.1.0, `--timeout-multiplier 3 -j 2`, on the machine in the [README](README.md). Machine-readable: [results.json](results.json).

## openplush-flatten

| Generated | Caught | Missed | Timeout | Unviable | Wall time |
|---|---|---|---|---|---|
| 723 | 608 | 9 | 34 | 72 | 85 min in two runs (54 min, cut off by a container restart after 514 mutants; then 31 min with `--iterate` for the 242 not yet caught) |

Of the 651 viable mutants, 608 are caught and 34 time out, so 9 (1.4%) survive [solid].

### The nine survivors

Each was rechecked on `main` at `85fd6bc`, after #395 and #396: all nine still survive, and a control mutation in the same function (`Rest::jacobian`, line 26) is caught [solid].

| Where | Mutation | Verdict |
|---|---|---|
| `fallback.rs:58` `dart` | `+` → `*` in the path length | **Gap** [likely]: no test checks that the proposed dart is the shortest path; a chart with two candidate paths of different length would catch it |
| `energy.rs:124` `first_positive_root` | `&&` → `\|\|` in the finiteness guard | Gap in a guard that the session can't reach [likely]: callers pass finite layouts; a unit test of `max_step` with a non-finite coefficient would catch it |
| `energy.rs:134` `first_positive_root` | `>` → `>=` on the root | Same [likely]: a root at exactly 0 means a face already collapsed, which det J > 0 rules out |
| `interior/mod.rs:110` `Solve::iterate` | `<` → `<=` on the 1e-9 gain | Equality at a threshold [likely]: no realistic input lands on it exactly |
| `closure.rs:111` `damped` | `<` → `<=` on the trial error | Equality at a threshold [likely] |
| `closure.rs:62` `gap` | `+` → `-` in the summed end point | Equivalent [solid]: negating a vector keeps its length |
| `energy.rs:25`, `energy.rs:29` `Rest::jacobian` | `+` → `-` on the `inv[1][0]` term | Equivalent [solid]: `rest_shapes` always builds `inv[1][0] = 0` |
| `energy.rs:39` `Rest::operator` | `+` → `-` on `b[0]` | Equivalent [solid]: `b[0]` is `inv[1][0]` |

So one real gap and two unreachable-guard gaps, out of 651 viable mutants.

### Timeouts

34 mutants ran past the limit (48 s against a 16 s baseline), 26 of them in `interior/energy.rs`. They are mostly mutations that keep the line search from ever accepting a step or from settling. cargo-mutants doesn't count a timeout as missed. Each one costs its full limit, so they are about a third of the wall time [likely].

## openplush-pattern

Running.

## Recommendation

Pending the pattern run.
