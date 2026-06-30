# Verification and CI Plan

This repository's evaluator hardening depends on keeping debug builds, release
builds, docs, lint policy, dependency checks, fuzzing, and deeper runtime checks
healthy. This document records the verification commands and the CI jobs that
protect the current safety model.

## Required Local Checks

Run these before merging evaluator, parser, resolver, or workflow changes:

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo test --release --workspace --all-targets --all-features
cargo test --doc --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./c-tests/run-smoke.sh
./c-tests/run-packaging-smoke.sh
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

The hostile-input fuzz target lives under `crates/shunting-yard/fuzz/`:

```bash
cd crates/shunting-yard/fuzz
cargo fuzz build evaluate_no_panic
cargo fuzz run evaluate_no_panic
```

GitLab CI has a manual `fuzz:evaluate_no_panic` job in `.gitlab-ci.yml`.
It uses a Rust nightly container and shell commands instead of Node-backed
GitHub Actions, which avoids runner failures caused by older Node.js runtimes.

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
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test -p shunting_yard --lib tests::evaluate_
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
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu --workspace --all-targets --all-features
```

The sanitizer job runs test binaries only. Regular doctests are covered by the
baseline CI workflow; local ASan doctest linking failed with missing sanitizer
runtime symbols from rustdoc-generated binaries.

LeakSanitizer can be added later if ASan remains stable. ThreadSanitizer should
wait until the crate has meaningful concurrency.

## CI Jobs

`.github/workflows/rust-ci.yml` runs on pushes to `main` and pull requests:

- `cargo fmt --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-targets --all-features`
- `cargo test --release --workspace --all-targets --all-features`
- `cargo test --doc --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `./c-tests/run-smoke.sh`
- `./c-tests/run-packaging-smoke.sh`

`.github/workflows/security.yml` runs supply-chain checks:

- `cargo audit`
- `cargo deny check`

`.github/workflows/fuzz.yml` is manual and builds the `evaluate_no_panic` fuzz
target.

`.github/workflows/miri.yml` is manual and scheduled weekly.

`.github/workflows/sanitizers.yml` is manual and scheduled weekly.

## FFI Adapter Checks

- the core crate must remain `unsafe_code = "forbid"`;
- unsafe code must remain isolated to `shunting_yard_ffi`;
- FFI exported functions must not unwind across the ABI boundary;
- C smoke tests must pass in CI;
- Rust-side FFI tests must cover null pointers, invalid UTF-8, success, and
  evaluation failure;
- callback resolver tests must cover null callback pointers, callback-provided
  bool/integer/float values, `user_data`, repeated lookup, callback failure,
  invalid value kinds, and invalid callback-provided floats.
- extended FFI entrypoints must support `out_error = NULL`;
- returned `ShyError` objects must be freed with `shy_error_free`;
- C smoke tests must cover error object allocation, accessors, and free;
- Rust-side FFI tests must cover null error accessor behavior;
- pointers returned by `shy_error_message` are valid only until
  `shy_error_free`;
- C callers must not free Rust-owned FFI error memory directly.
- parse functions must reject null input and null output handle pointers;
- successful FFI parse returns an owned non-null `ShyParsedExpression` handle;
- failed FFI parse leaves the output handle null when possible;
- parsed-expression handles must be freed with `shy_parsed_expression_free`;
- freeing a null parsed-expression handle is allowed;
- parsed no-variable and parsed callback evaluation must support `_ex` error
  reporting;
- repeated evaluation of the same parsed handle must be tested;
- callback-backed parsed evaluation must resolve variables at evaluation time.
- parse-stage errors must retain owned diagnostics inside `ShyError`;
- diagnostic accessors must handle null errors and invalid indexes safely;
- expected-token pointers must be borrowed from `ShyError`;
- expected-token pointers must not be freed by C;
- non-parse errors should report zero indexed diagnostics;
- C smoke tests must cover parse diagnostic iteration.
- options must be initialized through `shy_eval_options_default`;
- with-options entrypoints must reject null option pointers;
- invalid ABI size must be rejected;
- zero limits must be rejected;
- overlarge values that cannot fit Rust `usize` must be rejected;
- tight limits must produce resource-limit errors before evaluation side
  effects;
- callback with-options tests must prove resource limits can prevent callback
  invocation;
- C smoke tests must cover default options, valid options, resource-limit
  failure, and invalid options.
- ABI regression smoke must freeze exported function names, status codes, error
  codes, callback ABI, `ShyValue`, and `ShyEvalOptions`;
- packaging smoke must verify pkg-config flags;
- packaging smoke must build and run CMake consumers;
- packaging smoke must build and run direct shared-library consumers;
- packaging smoke must build and run direct static-library consumers;
- standalone consumer examples must not require Cargo knowledge after the FFI
  package is installed.

## Current Policy Notes

Unsafe code is forbidden in the core crate through Cargo lints. The FFI crate
contains the repository's raw pointer boundary and must keep unsafe operations
small, documented, and covered by Rust-side and C smoke tests. Panic-prone
constructs such as `unwrap`, `expect`, `todo!`, and `unimplemented!` are denied
in hand-written production modules of the core crate. The generated parser
integration has a narrow documented allowance because generated LALRPOP code
uses unwrap internally.

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
- no unsafe code should be introduced outside the FFI adapter crate.

Diagnostic-aware APIs must preserve compatibility:

- detailed APIs must return `Error::Lexical`, `Error::Parse`,
  `Error::ResourceLimit`, or `Error::Eval` according to the failure stage;
- legacy APIs must continue returning their existing `EvalError` variants;
- converting detailed errors into legacy `EvalError` must preserve existing
  public behavior;
- tests should match structured variants and avoid depending on free-form
  display strings.
