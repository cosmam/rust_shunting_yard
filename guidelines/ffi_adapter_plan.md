# FFI Adapter Plan

## Scope

The FFI adapter crate exposes a C ABI for the safe `shunting_yard` core crate.
The core crate remains unsafe-free.

## Crates

- `shunting_yard`: safe Rust core.
- `shunting_yard_ffi`: C ABI adapter.

## Current ABI

- `shy_evaluate_no_vars`

## Safety Policy

- Unsafe code is isolated to the FFI crate.
- Unsafe blocks must be small and documented with `SAFETY:` comments.
- No Rust panic may cross the C ABI boundary.
- Exported functions return status codes instead of panicking.
- C callers own all input pointers.
- Rust does not retain C input pointers after the call returns.
- The first FFI API does not allocate Rust-owned output strings.
- Future APIs that allocate memory must provide matching free functions.

## Current Limitations

- No variable callback resolver yet.
- No structured diagnostic buffer yet.
- No parse/evaluate object handles yet.
- C smoke testing is Linux-first.

## C Smoke Test

The initial C smoke test is Linux-first and exercises the dynamic library path
by linking with `-lshunting_yard_ffi` and setting `LD_LIBRARY_PATH` to
`target/debug`. Static-library smoke testing can be added as a separate
follow-up.

## Next Planned FFI Step

Add C callback resolver support:

```text
C callback
    -> unsafe extern "C" trampoline
    -> safe FfiResolver
    -> VariableResolver
    -> evaluate_with_resolver / evaluate_parsed
```
