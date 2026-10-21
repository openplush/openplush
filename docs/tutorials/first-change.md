# Tutorial: your first engine change, test first

<!-- index: add a method to the engine's millimetre type, test first, and run every engine check. -->

You will add one small method to the engine's millimetre type, writing its test before the code, and run the same checks CI runs. It takes about an hour, most of it waiting for the build, and at the end you have done every step of a real engine change on your own machine.

## You will

- Write a test for a method that does not exist yet, and watch it fail.
- Write the method, and watch the test pass.
- Run every engine check and see it end in `check: ok`.
- Know what turns an exercise like this into a pull request.

## Before you start

- [Set up and run every check](../start/setup.md) once, so the tools are installed and your machine gives the same answers as CI.
- Skim the [crates](../reference/crates/README.md): this tutorial works in `openplush-geom`, the crate at the bottom of the engine, which every other crate builds on. It has no engine dependencies, so a change there builds fast and touches nothing else.

This is an exercise: you throw the change away at the end. A change that is merged starts from a [task](../reference/glossary.md#process), and nothing in the engine needs this method yet.

## Steps

### 1. Make a branch for the exercise

From an up-to-date `main`:

```sh
git switch main && git pull
git switch -c exercise/mm-checked-add
```

### 2. Read the type you are changing

Open [`crates/openplush-geom/src/units.rs`](https://github.com/openplush/openplush/blob/main/crates/openplush-geom/src/units.rs). `Mm` is a length in millimetres, and its only public constructor, `Mm::try_new`, refuses a negative or non-finite value with `GeomError::InvalidLength`. So every `Mm` that exists is finite and not negative, and code that compares lengths, such as the check that both sides of a [seam](../reference/glossary.md#geometry) are equally long, never has to guard against a NaN.

The method you will add, `Mm::checked_add`, puts two lengths end to end, as you would to get the total length of a seam made of two edges. Adding two finite numbers can overflow to infinity, and an infinite `Mm` would break the type's promise. The engine never clamps or quietly fixes up a bad value, so the method returns an error instead.

### 3. Write the test first

Create `crates/openplush-geom/tests/units.rs` with two tests, one for the result you want and one for the overflow:

```rust
use openplush_geom::GeomError;
use openplush_geom::units::Mm;

#[test]
fn two_seam_lengths_add_up() {
    let side = Mm::try_new(120.0).expect("valid length");
    let top = Mm::try_new(35.5).expect("valid length");
    assert_eq!(side.checked_add(top), Mm::try_new(155.5));
}

#[test]
fn a_sum_that_overflows_is_an_error() {
    let huge = Mm::try_new(f64::MAX).expect("valid length");
    assert!(matches!(
        huge.checked_add(huge),
        Err(GeomError::InvalidLength { .. })
    ));
}
```

A file in a crate's `tests/` folder is an integration test: it sees only what the crate makes public, as another crate would. The second test is the one that matters most here, because it pins the failure: a sum too large to hold is a typed error, not infinity and not the largest number.

### 4. Watch it fail

Run only the new test file:

```sh
cargo test -p openplush-geom --test units
```

It does not compile, because the method is missing:

```text
error[E0599]: no method named `checked_add` found for struct `Mm` in the current scope
 --> crates/openplush-geom/tests/units.rs:8:21
  |
8 |     assert_eq!(side.checked_add(top), Mm::try_new(155.5));
  |                     ^^^^^^^^^^^ method not found in `Mm`
```

The same error follows for line 15. A test that fails for the reason you expect proves it tests the thing you are about to write. In a pull request this is the first of two commits, so a reviewer reads what was promised before reading how.

### 5. Write the method

In `crates/openplush-geom/src/units.rs`, add the method inside `impl Mm`, after `get`:

```rust
    /// The two lengths end to end. Errors if the sum overflows to infinity.
    pub fn checked_add(self, other: Self) -> Result<Self, GeomError> {
        Self::try_new(self.0 + other.0)
    }
```

It reuses `Mm::try_new`, so the rule for what a valid length is stays in one place. The doc comment is one line and says only what the signature does not: when it errors.

### 6. Watch it pass

Run the test again:

```sh
cargo test -p openplush-geom --test units
```

```text
running 2 tests
test a_sum_that_overflows_is_an_error ... ok
test two_seam_lengths_add_up ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Drop `--test units` to run every test in the crate.

### 7. Run every engine check

```sh
scripts/check.sh engine
```

This runs what CI runs for the engine: the crate boundary and module-size checks, hygiene, formatting, clippy, every Rust test, the generated TypeScript types, the workflow linter, and a build of every test for WebAssembly, since the engine also runs in the browser. The first run takes several minutes and a few gigabytes of disk for the build output. It ends with:

```text
check: ok (boundary hygiene fmt clippy test types workflows wasm-check)
```

If a step fails, its output names the file and line; fix it and rerun just that step, such as `scripts/check.sh clippy`.

### 8. Throw the exercise away

```sh
git restore crates/openplush-geom/src/units.rs
rm crates/openplush-geom/tests/units.rs
git switch main
git branch -D exercise/mm-checked-add
```

## What you built

You made an engine change the way every one is made here: a failing test that states the promise, the smallest code that keeps it, a typed error where the input can't be honoured, and a clean run of the engine checks.

For a real change, pick a ready task from a feature's `tasks.md` (the [path through this book](../start/README.md#the-path-through-this-book) shows where), commit the test and then the code as two commits, and [scope and open a pull request](../how-to/pull-requests.md). What is built and what is still open is on the [status page](../start/status.md).
