# FFI Adapter Plan

## Scope

The FFI adapter crate exposes a C ABI for the safe `shunting_yard` core crate.
The core crate remains unsafe-free.

## Crates

- `shunting_yard`: safe Rust core.
- `shunting_yard_ffi`: C ABI adapter.

## Current ABI

- `shy_eval_options_default`
- `shy_evaluate_no_vars`
- `shy_evaluate_no_vars_ex`
- `shy_evaluate_no_vars_with_options`
- `shy_evaluate_no_vars_with_options_ex`
- `shy_evaluate_with_callback`
- `shy_evaluate_with_callback_ex`
- `shy_evaluate_with_callback_with_options`
- `shy_evaluate_with_callback_with_options_ex`
- `shy_parse_expression`
- `shy_parse_expression_ex`
- `shy_parse_expression_with_options`
- `shy_parse_expression_with_options_ex`
- `shy_parsed_expression_free`
- `shy_evaluate_parsed_no_vars`
- `shy_evaluate_parsed_no_vars_ex`
- `shy_evaluate_parsed_with_callback`
- `shy_evaluate_parsed_with_callback_ex`
- `shy_error_free`
- `shy_error_status`
- `shy_error_stage`
- `shy_error_code`
- `shy_error_message`
- `shy_error_has_span`
- `shy_error_span_start`
- `shy_error_span_end`
- `shy_error_diagnostic_count`
- `shy_error_diagnostic_kind`
- `shy_error_diagnostic_has_span`
- `shy_error_diagnostic_span_start`
- `shy_error_diagnostic_span_end`
- `shy_error_diagnostic_expected_count`
- `shy_error_diagnostic_expected_token`

## Safety Policy

- Unsafe code is isolated to the FFI crate.
- Unsafe blocks must be small and documented with `SAFETY:` comments.
- No Rust panic may cross the C ABI boundary.
- Exported functions return status codes instead of panicking.
- C variable resolver callbacks must not unwind across the ABI boundary.
- C callers own all input pointers.
- Rust does not retain C input pointers after the call returns.
- Rust does not retain callback variable-name pointers or `user_data` after the
  call returns.
- Status-only evaluation APIs do not allocate Rust-owned error objects.
- Extended evaluation APIs may allocate Rust-owned `ShyError` objects.
- C callers must free returned error objects with `shy_error_free`.
- Pointers returned by `shy_error_message` are borrowed from the error object
  and remain valid only until `shy_error_free`.
- Future APIs that allocate memory must provide matching free functions.

## Current Limitations

- Diagnostic iteration exposes kind, span, and expected tokens, but not the
  full nested recovery structure yet.
- FFI error messages are human-readable and are not a stable machine-readable
  format.
- C smoke testing is Linux-first.

## Error Reporting

The `_ex` entrypoints can return an owned `ShyError` object through an optional
`ShyError **out_error` argument. If `out_error` is `NULL`, no error object is
allocated and callers receive only the `ShyStatus`.

On success, `_ex` entrypoints write `NULL` to `*out_error` when `out_error` is
provided. On failure, they leave `out_value` unchanged and write a newly
allocated error object to `*out_error` when requested.

The error object exposes stable integer stage and code values through accessors
rather than exposing Rust enums or core diagnostic structures directly. Source
spans are available for lexical and parse failures when the core parser reports
them.

## EvalOptions

The FFI adapter exposes `ShyEvalOptions`, a C-compatible resource-limit
configuration struct. Callers initialize it with `shy_eval_options_default`,
optionally adjust limits, and pass it to `_with_options` entrypoints.

The struct uses fixed-width integer fields rather than Rust `usize`. The
adapter validates ABI size, nonzero values, and conversion into Rust `usize`
before calling the core crate.

Invalid options return `SHY_STATUS_INVALID_OPTIONS` and, for `_ex` entrypoints,
an input-stage `ShyError` with `SHY_ERROR_CODE_INVALID_OPTIONS`.

## Parse Diagnostic Iteration

Parse-stage `ShyError` objects retain owned diagnostic records. C callers can
inspect them by index through accessor functions.

Diagnostic indexes are zero-based. Invalid indexes return safe defaults.
Expected-token strings are borrowed from `ShyError` and remain valid only until
`shy_error_free`.

Expected-token strings are intended for diagnostics and display. They should not
be treated as a stable machine-readable grammar schema.

## Parsed Expression Handles

`ShyParsedExpression` is an opaque handle owned by C callers after a successful
parse call. It must be released with `shy_parsed_expression_free`.

Handles are immutable and may be evaluated repeatedly. Variable values are
resolved at evaluation time, not parse time.

C callers must not use a handle after freeing it.

## C Smoke Test

The initial C smoke test is Linux-first and exercises the dynamic library path
by linking with `-lshunting_yard_ffi` and setting `LD_LIBRARY_PATH` to
`target/debug`. Static-library smoke testing can be added as a separate
follow-up.

## Callback Resolver Shape

`shy_evaluate_with_callback` routes variable lookup through the caller's C
callback:

```text
C callback
    -> unsafe extern "C" trampoline
    -> safe FfiResolver
    -> VariableResolver
    -> evaluate_with_resolver / evaluate_parsed
```

The callback receives a NUL-terminated variable name, caller-owned `user_data`,
and writable `ShyValue` output storage. Non-OK callback statuses are returned
from `shy_evaluate_with_callback`. Unknown `ShyValue.kind` values are rejected
as `SHY_STATUS_INVALID_VALUE`; non-finite and subnormal floats are rejected by
the core evaluator and reported through the FFI as evaluation errors.

## Next Planned FFI Step

Add C packaging guidance and smoke coverage for CMake, pkg-config, and static
library consumers.
