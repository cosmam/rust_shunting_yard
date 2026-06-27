# Verification and CI Plan

This repository's evaluator hardening depends on keeping debug builds, release
builds, docs, lint policy, dependency checks, fuzzing, and deeper runtime checks
healthy. This document records the verification commands and the CI jobs that
protect the current safety model.

## Required Local Checks

Run these before merging evaluator, parser, resolver, or workflow changes:

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo test --release --all-targets --all-features
cargo test --doc --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

The release test run matters because arithmetic overflow behavior must not
split between debug and release builds.

## Supply-Chain Checks

Run these after dependency, lockfile, or supply-chain policy changes:

```bash
cargo audit
cargo deny check
```

`deny.toml` intentionally keeps the policy compact: advisories are checked,
unknown registries and git sources are denied, wildcard dependencies are denied,
license allow-listing is explicit, and duplicate dependencies are reported.

## Fuzzing

The hostile-input fuzz target lives under `fuzz/`:

```bash
cd fuzz
cargo fuzz build evaluate_no_panic
cargo fuzz run evaluate_no_panic
```

The target accepts arbitrary bytes, ignores invalid UTF-8, and passes valid
UTF-8 into `evaluate` with an empty variable map. Its purpose is crash and panic
detection, not semantic correctness.

Seed corpus entries cover overflow boundaries, divide/modulo/remainder by zero,
bad shifts, non-finite float generation, parser garbage, nested input, and large
argument lists.

## Miri

Miri is available as a manual and weekly scheduled workflow. The local command
matching CI is:

```bash
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test --lib tests::evaluate_
```

The scope is deliberately narrower than the full test suite. The public
evaluator hardening tests run well under Miri, while the full property suite is
too slow under interpretation and spends most of its time in proptest machinery.
If Miri reports a failure, investigate it rather than treating the job as a
best-effort signal.

## Sanitizers

AddressSanitizer is available as a manual and weekly scheduled workflow. The
local command matching CI is:

```bash
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu --all-targets --all-features
```

The sanitizer job runs test binaries only. Regular doctests are covered by the
baseline CI workflow; local ASan doctest linking failed with missing sanitizer
runtime symbols from rustdoc-generated binaries.

LeakSanitizer can be added later if ASan remains stable. ThreadSanitizer should
wait until the crate has meaningful concurrency.

## CI Jobs

`.github/workflows/rust-ci.yml` runs on pushes to `main` and pull requests:

- `cargo fmt --check`
- `cargo test --all-targets --all-features`
- `cargo test --release --all-targets --all-features`
- `cargo test --doc --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`

`.github/workflows/security.yml` runs supply-chain checks:

- `cargo audit`
- `cargo deny check`

`.github/workflows/fuzz.yml` is manual and builds the `evaluate_no_panic` fuzz
target.

`.github/workflows/miri.yml` is manual and scheduled weekly.

`.github/workflows/sanitizers.yml` is manual and scheduled weekly.

## Current Policy Notes

Unsafe code is forbidden through Cargo lints. Panic-prone constructs such as
`unwrap`, `expect`, `todo!`, and `unimplemented!` are denied in hand-written
production modules. The generated parser integration has a narrow documented
allowance because generated LALRPOP code uses unwrap internally.

All evaluation paths must continue to reject malformed parser recovery, invalid
variable floats, non-finite float operation results, subnormal floats, checked
integer overflows, divide/modulo/remainder by zero, invalid shifts, and resource
limit violations with typed errors rather than panics.

Parse/evaluate separation must preserve the existing safety model:

- direct `evaluate` and `parse` plus `evaluate_parsed` must produce equivalent
  results for successful inputs;
- parse-only APIs must enforce lexical, parser, and resource-limit failures
  before evaluation;
- evaluate-parsed APIs must still validate resolver-returned values;
- no new unsafe code should be introduced.
