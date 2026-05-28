# Cargo Mutants Guide

This guide explains how to read `cargo-mutants` results and turn them into useful
test work. This standalone repo is a single Rust crate, so the default config
mutates all Rust implementation files:

```bash
cargo mutants
```

That command reads `.cargo/mutants.toml`, mutates `src/**/*.rs`, and runs the
`shunting_yard` package test suite.

## Fast Sanity Checks

Before running a mutation campaign, confirm the configured scope:

```bash
cargo mutants --list-files
cargo mutants --list
```

`--list-files` should show only files under `src/`. If generated output,
build artifacts, C++, Python, or other non-crate files appear, stop and fix
`.cargo/mutants.toml` before running the campaign.

Also confirm the underlying test package:

```bash
cargo test -p shunting_yard
```

## Output Files

After a run, inspect `mutants.out/`. The most useful files are:

- `missed.txt`: mutants that survived. Start here.
- `caught.txt`: mutants killed by tests. Usually no action needed.
- `unviable.txt`: mutants that could not build or run meaningfully.
- `timeout.txt`: mutants that exceeded the timeout.
- `log/`: per-mutant command output.
- `diff/`: mutated code diffs, useful for understanding exactly what changed.
- `outcomes.json`: machine-readable summary for scripts or CI.

`mutants.out*` is ignored by git and should not be committed.

## Result Meanings

`caught` means the tests failed after the mutation. This is good: the suite noticed the
behavior change.

`missed` means the tests still passed after the mutation. Treat this as a test coverage
question, not automatically as a bug. The mutated behavior may be meaningful, equivalent,
or outside the contract.

`unviable` means the mutation produced code that could not compile or could not be tested.
This usually needs no action.

`timeout` means the mutation made the test run too slowly or hang. Investigate these
carefully because a timeout may reveal an infinite parser/evaluator path or an expression
case that needs a smaller regression test.

## Triage Workflow

For each entry in `missed.txt`:

1. Read the mutant description.
2. Open the matching diff in `mutants.out/diff/`.
3. Decide whether the mutated behavior is observably wrong for expression parsing,
   precedence, type checking, or evaluation.
4. If it is wrong, add or strengthen a focused test.
5. If it is equivalent, leave it alone unless many equivalent mutants point to code that
   can be simplified.
6. If it is intentionally out of scope, consider excluding that item only after
   documenting why.

Prefer the smallest test that kills the mutant. For shunting-yard behavior, use this
order:

1. Existing unit test near the mutated module, when the behavior is internal and precise.
2. Public `evaluate` tests, when the behavior crosses lexing, parsing, and evaluation.
3. A table-driven `rstest`, when one expression pattern should hold across several
   operators, numeric types, or boolean cases.

## When To Add Tests

Add a test when a missed mutant changes behavior users would care about:

- Operator precedence or associativity changes.
- Unary and binary operators bind differently.
- Parentheses, function calls, or comma-separated arguments parse incorrectly.
- Integer, float, boolean, or variable tokens produce the wrong expression.
- Type errors, parser errors, or math errors stop being reported.
- Numeric functions return the wrong value, overflow behavior, or invalid-type result.
- Short boolean and bitwise expression cases evaluate incorrectly.

Good tests usually assert a specific `Value` or `EvalError`. Avoid broad fixtures that
make it unclear which behavior killed the mutant.

## When Not To Add A Test

Do not add a test just to kill an equivalent mutant. Examples:

- Reordered code that produces the same observable result.
- Mutations in diagnostic display text when the exact text is not part of the contract.
- Mutations inside defensive branches that are impossible to reach through public APIs.

If a mutant seems equivalent but hard to reason about, prefer a small refactor that makes
the equivalence obvious over adding a brittle test.

## Updating The Config

Use `examine_globs` to control implementation files under mutation. The current setting is:

```toml
test_package = ["shunting_yard"]
examine_globs = [
    "src/**/*.rs",
]
```

Only add exclusions after checking that a mutant is repeatedly equivalent, untestable, or
outside the expression evaluator contract. Prefer improving code or tests first.

## Local Commands

List the mutation scope:

```bash
cargo mutants --list-files
```

List the exact mutants:

```bash
cargo mutants --list
```

Run the configured mutation campaign:

```bash
cargo mutants
```

Run and keep output visible:

```bash
cargo mutants --no-shuffle
```

Rerun normal checks after changing tests or code:

```bash
cargo test -p shunting_yard
cargo clippy -p shunting_yard -- -D warnings
cargo fmt --check
```

## CI Expectations

The GitHub Actions workflow uploads `mutants.out` as an artifact. When CI fails:

1. Download the artifact.
2. Open `missed.txt`.
3. Inspect the corresponding diff and log.
4. Add the smallest useful test or document why the mutant is equivalent.
5. Rerun `cargo mutants` locally if the campaign is small enough, or at least rerun
   `cargo test -p shunting_yard` before pushing.

Mutation testing is a feedback tool. The goal is not to blindly reach zero survivors at
any cost; the goal is to find meaningful missing assertions and keep the expression
contract explicit.
