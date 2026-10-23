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

| Generated | Caught | Missed | Timeout | Unviable | Wall time |
|---|---|---|---|---|---|
| 274 | 222 | 22 | 1 | 29 | 10 min, one run |

Of the 245 viable mutants, 22 (9.0%) survive [solid]. Pattern's tests are mostly end-to-end oracles over whole outlines, which say a refusal happened but rarely which edges it names or which branch of a predicate decided it.

### The 22 survivors

| Where | Mutations | Verdict |
|---|---|---|
| `outline.rs:97–98` `turns_cleanly` | always `Ok(true)`; drop the `Sign::Zero` arm; `\|\|` → `&&` | **Gap** [likely]: no test has an outline whose corner folds back along its own edge (a collinear spike), which only this check refuses, since `check_simple` skips adjacent edges |
| `outline.rs:122` `segments_meet` | `\|\|` → `&&` in the collinear-touch case | **Gap** [likely]: no test has a vertex lying on a non-adjacent edge |
| `outline.rs:59` `check_simple` | four mutations of `(a + 1) % n` in the refusal | **Gap** [solid]: no test asserts which edges a corner refusal names |
| `allowance.rs:153` `join` | `/ cross` → `* cross` | **Gap** [likely]: every allowance test has right-angle corners, where `cross = ±1` and the two agree |
| `cut.rs:31`, `cut.rs:36` | `as_printed`, `flipped` → `1` | **Gap** [solid]: the one test uses a count of one of each |
| `allowance.rs:56`, `allowance.rs:109` | `>` → `>=`, `<` → `<=` | Equality at a threshold [likely] |
| `allowance.rs:126` `strip` | `>` → `>=` on the width | Unreachable until zero-allowance edges exist (001 T152) [likely] |
| `allowance.rs:116` | `j < i` → `j <= i` | Equivalent [solid]: `j == i` needs a one-edge outline, and the loop runs only above three |
| `outline.rs:107–109` `segments_meet` | `\|\|` → `&&` in the box test | Equivalent [solid]: the box test only skips the exact predicates, which give the same answer |
| `outline.rs:79` `check_counter_clockwise` | four comparisons in the lowest-vertex choice | Equivalent [likely]: the mutated choice is still an extreme vertex, which is convex, so its turn is still the orientation |

So 12 survivors are test gaps: the fold-back corner (3), the collinear touch (1), the refusal's edge index (4), the non-right miter (1), the cut-count accessors (2) and, once 001 T152 lands, the zero-width strip (1).

## Cost

| | flatten | pattern |
|---|---|---|
| Mutants | 723 | 274 |
| Wall time, `-j 2` on 4 vCPUs | 85 min | 10 min |
| Per viable mutant | 7.8 s | 2.4 s |

Flatten's test suite takes about 16 s and its 34 timeouts each cost 48 s, so it dominates. A full run of both crates doesn't fit in a per-PR check [solid]. `--in-diff` limits a run to the mutants on lines a PR touches, which would cost a few minutes for a typical engine PR [likely; not measured here].

## Recommendation

For the maintainer's adoption decision; nothing here is adopted.

1. **Per PR:** `cargo mutants --in-diff` on `crates/` changes, reporting survivors as a review comment without blocking, in place of the `mutation-auditor` agent's hand-picked mutations or beside it. It is mechanical and repeatable where the agent is not, but it only mutates operators and return values, which misses the semantic mutations the agent tried on #395 (moving a statement, perturbing a constant) [likely].
2. **Weekly:** a full run of every engine crate, its survivors filed as test tasks.
3. **Not a gate yet.** Equivalent mutants (12 of the 31 survivors here, 8 of them certain) would need an exclusion list (`mutants.toml`) before a missed mutant could fail CI.

The 12 pattern gaps and the 3 flatten gaps above (the dart's shortest path, and the two `first_positive_root` guards) are candidate test tasks; this spike adds none.
