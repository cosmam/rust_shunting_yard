# Rust Shunting Yard: FFI Error Reporting Implementation Plan

## Purpose

This document assumes the callback resolver branch has merged successfully.

At that point, the repository should have:

```text
safe Rust core crate
FFI adapter crate
C smoke-test consumer
workspace CI
no-variable FFI evaluation
callback-backed FFI variable resolution
validated C ABI value/status handling
unsafe code isolated to shunting_yard_ffi
```

The next feature is **FFI error reporting**.

The goal is to let C callers retrieve meaningful error information after an FFI call fails, without exposing Rust-owned references, Rust enums with invalid-discriminant risk, or unstable internal diagnostic structures directly across the C ABI.

Recommended branch:

```text
feature/ffi-error-reporting
```

---

## High-Level Goal

Add an FFI-safe error object model:

```text
C caller
    -> calls shy_evaluate_no_vars or shy_evaluate_with_callback
    -> receives ShyStatus
    -> if status != OK, caller can inspect a ShyError object
    -> caller frees error object using a matching free function
```

The new ABI should support at least:

```text
- stage/category of failure;
- status code;
- stable error code;
- human-readable message;
- source span for lexical/parse errors when available;
- diagnostic count where applicable;
- safe allocation and free functions.
```

This feature should **not** try to expose the entire Rust `Error`, `EvalError`, or `ParseDiagnostics` structure directly.

---

# Target API Shape

The safest first version is to add new “extended” entrypoints that write both a value and an optional error handle:

```c
ShyStatus shy_evaluate_no_vars_ex(
    const char *expression,
    ShyValue *out_value,
    ShyError **out_error
);

ShyStatus shy_evaluate_with_callback_ex(
    const char *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value,
    ShyError **out_error
);
```

Where:

```c
typedef struct ShyError ShyError;
```

The error object is opaque to C callers.

C callers inspect it through accessor functions:

```c
void shy_error_free(ShyError *error);

ShyStatus shy_error_status(const ShyError *error);
int32_t shy_error_stage(const ShyError *error);
int32_t shy_error_code(const ShyError *error);
const char *shy_error_message(const ShyError *error);

int32_t shy_error_span_start(const ShyError *error);
int32_t shy_error_span_end(const ShyError *error);
int32_t shy_error_has_span(const ShyError *error);

int32_t shy_error_diagnostic_count(const ShyError *error);
```

The original existing entrypoints should remain:

```c
ShyStatus shy_evaluate_no_vars(...);
ShyStatus shy_evaluate_with_callback(...);
```

They should either continue using the old implementation or delegate to the new extended helpers with `out_error = NULL`.

---

# Design Principles

## 1. Keep the core crate unchanged

Do not add FFI-specific error types to the safe core crate.

The FFI crate should translate from:

```text
shunting_yard::Error
shunting_yard::EvalError
shunting_yard::ParseDiagnostics
```

into FFI-owned error objects.

## 2. Keep all Rust-owned memory behind explicit free functions

If Rust allocates an error object, C must free it with:

```c
shy_error_free(error);
```

No other free function, `free`, `delete`, or caller allocator may be used.

## 3. Do not expose Rust references across the ABI

Accessor functions may return `const char *` pointing into the `ShyError` object, but that pointer is valid only until `shy_error_free`.

Do not return pointers into temporary strings.

## 4. Do not expose Rust enums directly to C

Use raw `int32_t` constants for ABI stages and codes.

Inside Rust, you may still use Rust enums for internal implementation, but exported/imported ABI fields and return values should be raw integer codes.

## 5. Preserve existing status behavior

Existing status codes should remain stable:

```text
OK
NULL_POINTER
INVALID_UTF8
EVALUATION_ERROR
PANIC
RESOLVER_ERROR
INVALID_VALUE
```

Error reporting should add details, not change the meaning of existing statuses.

---

# Proposed C ABI Additions

## Opaque error type

In the public header:

```c
typedef struct ShyError ShyError;
```

## Error stages

Use integer constants:

```c
enum {
    SHY_ERROR_STAGE_NONE = 0,
    SHY_ERROR_STAGE_INPUT = 1,
    SHY_ERROR_STAGE_LEXICAL = 2,
    SHY_ERROR_STAGE_PARSE = 3,
    SHY_ERROR_STAGE_RESOURCE_LIMIT = 4,
    SHY_ERROR_STAGE_EVALUATION = 5,
    SHY_ERROR_STAGE_RESOLVER = 6,
    SHY_ERROR_STAGE_PANIC = 7,
    SHY_ERROR_STAGE_INVALID_VALUE = 8,
};
```

These should be mirrored in Rust as `i32` constants.

## Error codes

Use stable integer constants.

Start small and pragmatic:

```c
enum {
    SHY_ERROR_CODE_NONE = 0,
    SHY_ERROR_CODE_NULL_POINTER = 1,
    SHY_ERROR_CODE_INVALID_UTF8 = 2,
    SHY_ERROR_CODE_PANIC = 3,

    SHY_ERROR_CODE_LEXICAL_ERROR = 100,

    SHY_ERROR_CODE_PARSE_ERROR = 200,
    SHY_ERROR_CODE_PARSE_RECOVERY = 201,

    SHY_ERROR_CODE_RESOURCE_LIMIT = 300,
    SHY_ERROR_CODE_INPUT_TOO_LARGE = 301,
    SHY_ERROR_CODE_TOO_MANY_TOKENS = 302,
    SHY_ERROR_CODE_AST_TOO_LARGE = 303,
    SHY_ERROR_CODE_EXPRESSION_TOO_DEEP = 304,
    SHY_ERROR_CODE_TOO_MANY_FUNCTION_ARGUMENTS = 305,
    SHY_ERROR_CODE_TOO_MANY_PARSER_RECOVERIES = 306,

    SHY_ERROR_CODE_EVAL_ERROR = 400,
    SHY_ERROR_CODE_INVALID_ARITY = 401,
    SHY_ERROR_CODE_INVALID_TYPE = 402,
    SHY_ERROR_CODE_DIVISION_BY_ZERO = 403,
    SHY_ERROR_CODE_INTEGER_OVERFLOW = 404,
    SHY_ERROR_CODE_INVALID_SHIFT_COUNT = 405,
    SHY_ERROR_CODE_INVALID_EXPONENT = 406,
    SHY_ERROR_CODE_INVALID_PRECISION = 407,
    SHY_ERROR_CODE_NON_FINITE_FLOAT = 408,
    SHY_ERROR_CODE_SUBNORMAL_FLOAT = 409,
    SHY_ERROR_CODE_UNEXPECTED_OPCODE = 410,
    SHY_ERROR_CODE_UNKNOWN_VARIABLE = 411,
    SHY_ERROR_CODE_INVALID_EXPRESSION = 412,

    SHY_ERROR_CODE_RESOLVER_ERROR = 500,
    SHY_ERROR_CODE_INVALID_VALUE_KIND = 600,
};
```

Do not worry about perfect mapping in the first commit. It is acceptable to start with broad codes, then refine.

Recommended for this branch:

```text
Map the common high-value cases precisely:
- null pointer
- invalid UTF-8
- lexical error
- parse error / parse recovery
- resource limits
- division by zero
- integer overflow
- non-finite/subnormal float
- unknown variable / resolver error
- invalid callback value kind
- panic
```

## Extended evaluation functions

```c
ShyStatus shy_evaluate_no_vars_ex(
    const char *expression,
    ShyValue *out_value,
    ShyError **out_error
);

ShyStatus shy_evaluate_with_callback_ex(
    const char *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value,
    ShyError **out_error
);
```

Behavior:

```text
- On success:
  - return SHY_STATUS_OK;
  - write out_value;
  - if out_error is non-null, write NULL to *out_error.

- On failure:
  - return non-OK ShyStatus;
  - do not modify out_value;
  - if out_error is non-null, allocate a ShyError and write it to *out_error.
  - if out_error is null, do not allocate an error object.
```

## Error accessors

```c
void shy_error_free(ShyError *error);

ShyStatus shy_error_status(const ShyError *error);
int32_t shy_error_stage(const ShyError *error);
int32_t shy_error_code(const ShyError *error);
const char *shy_error_message(const ShyError *error);

int32_t shy_error_has_span(const ShyError *error);
int32_t shy_error_span_start(const ShyError *error);
int32_t shy_error_span_end(const ShyError *error);

int32_t shy_error_diagnostic_count(const ShyError *error);
```

Accessor behavior for null `error`:

```text
shy_error_status(NULL) -> SHY_STATUS_NULL_POINTER
shy_error_stage(NULL) -> SHY_ERROR_STAGE_NONE
shy_error_code(NULL) -> SHY_ERROR_CODE_NULL_POINTER
shy_error_message(NULL) -> NULL
shy_error_has_span(NULL) -> 0
shy_error_span_start(NULL) -> -1
shy_error_span_end(NULL) -> -1
shy_error_diagnostic_count(NULL) -> 0
```

---

# Rust-Side Design

## Internal error object

In `crates/shunting-yard-ffi/src/lib.rs`:

```rust
#[repr(C)]
pub struct ShyError {
    status: ShyStatusCode,
    stage: i32,
    code: i32,
    message: std::ffi::CString,
    has_span: i32,
    span_start: i32,
    span_end: i32,
    diagnostic_count: i32,
}
```

This struct is opaque to C. It does not need to expose fields in the header.

The `CString` is owned by the error object. `shy_error_message` returns:

```rust
error.message.as_ptr()
```

The pointer is valid until `shy_error_free(error)`.

## Allocation helper

```rust
fn allocate_error(error: ShyError) -> *mut ShyError {
    Box::into_raw(Box::new(error))
}
```

Free function:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_free(error: *mut ShyError) {
    if error.is_null() {
        return;
    }

    unsafe {
        // SAFETY:
        // - error must be a pointer returned by this crate through an out_error parameter.
        // - Box::from_raw takes ownership and drops the allocation exactly once.
        drop(Box::from_raw(error));
    }
}
```

## Message handling

Any string placed into `CString` must not contain interior NUL bytes.

Use a sanitizing helper:

```rust
fn cstring_lossy_no_nul(message: impl Into<String>) -> CString {
    let message = message.into().replace('\0', "\\0");

    match CString::new(message) {
        Ok(value) => value,
        Err(_) => CString::new("error message contained invalid NUL").unwrap_or_else(|_| unreachable!()),
    }
}
```

If `unwrap`/`expect` lints are denied in the FFI crate, avoid them:

```rust
fn cstring_static(message: &'static str) -> CString {
    match CString::new(message) {
        Ok(value) => value,
        Err(_) => {
            // Static messages in this module must not contain NUL bytes.
            let fallback = Vec::from("internal error");
            unsafe {
                // This specific fallback has no interior NUL and no trailing NUL.
                CString::from_vec_unchecked(fallback)
            }
        }
    }
}
```

Simpler safe version:

```rust
fn cstring_lossy_no_nul(message: impl Into<String>) -> CString {
    let sanitized = message.into().replace('\0', "\\0");
    CString::new(sanitized).unwrap_or_else(|_| {
        // If replacement failed for some unexpected reason, use a hand-built
        // CString through bytes that are known not to contain NUL.
        CString::new("error").expect("static string contains no NUL")
    })
}
```

But if `unwrap_used` / `expect_used` are not denied in the FFI crate, this is acceptable. If they are later denied, revisit.

Recommended for consistency:

```text
Avoid unwrap/expect in production FFI helpers.
```

## Error construction helper

```rust
struct ErrorParts {
    status: ShyStatusCode,
    stage: i32,
    code: i32,
    message: String,
    span: Option<(usize, usize)>,
    diagnostic_count: usize,
}

impl ErrorParts {
    fn into_ffi_error(self) -> ShyError {
        let (has_span, span_start, span_end) = match self.span {
            Some((start, end)) => (
                1,
                usize_to_i32_saturating(start),
                usize_to_i32_saturating(end),
            ),
            None => (0, -1, -1),
        };

        ShyError {
            status: self.status,
            stage: self.stage,
            code: self.code,
            message: cstring_lossy_no_nul(self.message),
            has_span,
            span_start,
            span_end,
            diagnostic_count: usize_to_i32_saturating(self.diagnostic_count),
        }
    }
}
```

Saturating helper:

```rust
fn usize_to_i32_saturating(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
```

Or avoid `unwrap_or` if lint policy requires.

## Writing optional errors

```rust
fn write_optional_error(out_error: *mut *mut ShyError, error: ShyError) {
    if out_error.is_null() {
        return;
    }

    let error = allocate_error(error);

    unsafe {
        // SAFETY:
        // - out_error was provided by the caller.
        // - caller must provide writable storage for one ShyError pointer.
        out_error.write(error);
    }
}
```

On success:

```rust
fn clear_optional_error(out_error: *mut *mut ShyError) {
    if out_error.is_null() {
        return;
    }

    unsafe {
        // SAFETY:
        // - out_error was provided by the caller.
        // - caller must provide writable storage for one ShyError pointer.
        out_error.write(std::ptr::null_mut());
    }
}
```

Important:

```text
If out_error is non-null but invalid, Rust cannot detect that safely.
The safety contract must require writable storage for one ShyError*.
```

---

# Phase 0: Baseline Verification

Start from merged `main`:

```bash
git checkout main
git pull
git checkout -b feature/ffi-error-reporting
```

Run:

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo test --release --workspace --all-targets --all-features
cargo test --doc --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo audit
cargo deny check
```

Run fuzz build:

```bash
cd crates/shunting-yard/fuzz
cargo fuzz build evaluate_no_panic
cd ../../..
```

Run C smoke:

```bash
bash ./c-tests/run-smoke.sh
```

Acceptance criteria:

```text
- Existing no-variable FFI tests pass.
- Existing callback FFI tests pass.
- Existing C smoke test passes.
- Main is green before error reporting work begins.
```

---

# Phase 1: Add Error Constants and Opaque Type

## Step 1.1: Add error stage constants

In Rust:

```rust
pub const SHY_ERROR_STAGE_NONE: i32 = 0;
pub const SHY_ERROR_STAGE_INPUT: i32 = 1;
pub const SHY_ERROR_STAGE_LEXICAL: i32 = 2;
pub const SHY_ERROR_STAGE_PARSE: i32 = 3;
pub const SHY_ERROR_STAGE_RESOURCE_LIMIT: i32 = 4;
pub const SHY_ERROR_STAGE_EVALUATION: i32 = 5;
pub const SHY_ERROR_STAGE_RESOLVER: i32 = 6;
pub const SHY_ERROR_STAGE_PANIC: i32 = 7;
pub const SHY_ERROR_STAGE_INVALID_VALUE: i32 = 8;
```

In C header:

```c
enum {
    SHY_ERROR_STAGE_NONE = 0,
    SHY_ERROR_STAGE_INPUT = 1,
    SHY_ERROR_STAGE_LEXICAL = 2,
    SHY_ERROR_STAGE_PARSE = 3,
    SHY_ERROR_STAGE_RESOURCE_LIMIT = 4,
    SHY_ERROR_STAGE_EVALUATION = 5,
    SHY_ERROR_STAGE_RESOLVER = 6,
    SHY_ERROR_STAGE_PANIC = 7,
    SHY_ERROR_STAGE_INVALID_VALUE = 8,
};
```

## Step 1.2: Add error code constants

Add the first stable set listed above.

In Rust, prefer `pub const` integers.

In C, use an anonymous enum.

## Step 1.3: Add opaque type to header

```c
typedef struct ShyError ShyError;
```

In Rust:

```rust
#[repr(C)]
pub struct ShyError {
    status: ShyStatusCode,
    stage: i32,
    code: i32,
    message: CString,
    has_span: i32,
    span_start: i32,
    span_end: i32,
    diagnostic_count: i32,
}
```

Acceptance criteria:

```text
- Header exposes ShyError only as an opaque type.
- Error stage constants exist in Rust and C.
- Error code constants exist in Rust and C.
- No fields of ShyError are exposed in the C header.
```

Recommended commit:

```text
Add opaque FFI error type and error constants
```

---

# Phase 2: Add Error Accessors and Free Function

## Step 2.1: Add free function

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_free(error: *mut ShyError) {
    if error.is_null() {
        return;
    }

    unsafe {
        // SAFETY:
        // - error must have been returned by this crate through an out_error parameter.
        // - this function takes ownership and frees it exactly once.
        drop(Box::from_raw(error));
    }
}
```

Header:

```c
void shy_error_free(ShyError *error);
```

## Step 2.2: Add accessors

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_status(error: *const ShyError) -> ShyStatusCode { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_stage(error: *const ShyError) -> int32_t { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_code(error: *const ShyError) -> int32_t { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_message(error: *const ShyError) -> *const c_char { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_has_span(error: *const ShyError) -> i32 { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_span_start(error: *const ShyError) -> i32 { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_span_end(error: *const ShyError) -> i32 { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_diagnostic_count(error: *const ShyError) -> i32 { ... }
```

For each non-null accessor:

```rust
let error = unsafe {
    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a pointer returned by this crate that has not been freed.
    &*error
};
```

Null behavior:

```text
shy_error_status(NULL) -> SHY_STATUS_NULL_POINTER
shy_error_stage(NULL) -> SHY_ERROR_STAGE_NONE
shy_error_code(NULL) -> SHY_ERROR_CODE_NULL_POINTER
shy_error_message(NULL) -> NULL
shy_error_has_span(NULL) -> 0
shy_error_span_start(NULL) -> -1
shy_error_span_end(NULL) -> -1
shy_error_diagnostic_count(NULL) -> 0
```

Acceptance criteria:

```text
- shy_error_free(NULL) is safe and no-op.
- Accessors handle NULL deterministically.
- Accessors do not allocate.
- shy_error_message returns a pointer valid until shy_error_free.
- Every unsafe block has a SAFETY comment.
```

Recommended commit:

```text
Add FFI error accessors and free function
```

---

# Phase 3: Build Error Mapping Helpers

## Step 3.1: Map `shunting_yard::Error`

Add:

```rust
fn error_parts_from_core_error(error: shunting_yard::Error) -> ErrorParts {
    match error {
        shunting_yard::Error::ResourceLimit(error) => resource_limit_error_parts(error),
        shunting_yard::Error::Lexical { span, error } => lexical_error_parts(span, error),
        shunting_yard::Error::Parse(diagnostics) => parse_error_parts(diagnostics),
        shunting_yard::Error::Eval(error) => eval_error_parts(error),
    }
}
```

## Step 3.2: Map lexical errors

```rust
fn lexical_error_parts(span: shunting_yard::SourceSpan, error: shunting_yard::LexicalError) -> ErrorParts {
    ErrorParts {
        status: SHY_STATUS_EVALUATION_ERROR,
        stage: SHY_ERROR_STAGE_LEXICAL,
        code: SHY_ERROR_CODE_LEXICAL_ERROR,
        message: format!("lexical error: {error}"),
        span: Some((span.start, span.end)),
        diagnostic_count: 1,
    }
}
```

## Step 3.3: Map parse diagnostics

For parse errors:

```rust
fn parse_error_parts(diagnostics: shunting_yard::ParseDiagnostics) -> ErrorParts {
    let diagnostic_count = diagnostics.len();

    let first_span = diagnostics
        .diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic.span.map(|span| (span.start, span.end)));

    let code = if diagnostics.recovery_count() > 0 {
        SHY_ERROR_CODE_PARSE_RECOVERY
    } else {
        SHY_ERROR_CODE_PARSE_ERROR
    };

    ErrorParts {
        status: SHY_STATUS_EVALUATION_ERROR,
        stage: SHY_ERROR_STAGE_PARSE,
        code,
        message: format!("parse failed with {diagnostic_count} diagnostic(s)"),
        span: first_span,
        diagnostic_count,
    }
}
```

Do not expose the full diagnostic vector in this branch. That can come later.

## Step 3.4: Map resource limits

Map variants precisely:

```text
InputTooLarge -> INPUT_TOO_LARGE
TooManyTokens -> TOO_MANY_TOKENS
AstTooLarge -> AST_TOO_LARGE
ExpressionTooDeep -> EXPRESSION_TOO_DEEP
TooManyFunctionArguments -> TOO_MANY_FUNCTION_ARGUMENTS
TooManyParserRecoveries -> TOO_MANY_PARSER_RECOVERIES
```

Status:

```text
SHY_STATUS_EVALUATION_ERROR
```

Stage:

```text
SHY_ERROR_STAGE_RESOURCE_LIMIT
```

## Step 3.5: Map eval errors

Map variants precisely where useful:

```text
InvalidArity -> INVALID_ARITY
InvalidType -> INVALID_TYPE
DivisionByZero -> DIVISION_BY_ZERO
IntegerOverflow -> INTEGER_OVERFLOW
InvalidShiftCount -> INVALID_SHIFT_COUNT
InvalidExponent -> INVALID_EXPONENT
InvalidPrecision -> INVALID_PRECISION
NonFiniteFloat -> NON_FINITE_FLOAT
SubnormalFloat -> SUBNORMAL_FLOAT
UnexpectedOpcode -> UNEXPECTED_OPCODE
UnknownVariable -> UNKNOWN_VARIABLE
InvalidExpression -> INVALID_EXPRESSION
```

If `EvalError::ResourceLimit` still exists, map it to the resource-limit stage.

If `EvalError::LexicalError`, `ParserError`, or `ParserRecovery` still appear through a legacy path, map them consistently.

## Step 3.6: Map callback-specific errors

For callback resolver failures:

```rust
fn resolver_error_parts(status: ShyStatusCode) -> ErrorParts {
    ErrorParts {
        status,
        stage: SHY_ERROR_STAGE_RESOLVER,
        code: SHY_ERROR_CODE_RESOLVER_ERROR,
        message: "variable resolver callback failed".to_owned(),
        span: None,
        diagnostic_count: 0,
    }
}
```

For invalid callback values:

```rust
fn invalid_value_error_parts() -> ErrorParts {
    ErrorParts {
        status: SHY_STATUS_INVALID_VALUE,
        stage: SHY_ERROR_STAGE_INVALID_VALUE,
        code: SHY_ERROR_CODE_INVALID_VALUE_KIND,
        message: "callback returned unknown ShyValue kind".to_owned(),
        span: None,
        diagnostic_count: 0,
    }
}
```

Acceptance criteria:

```text
- Resource errors map to resource-limit stage.
- Lexical errors include source span.
- Parse errors include diagnostic count and first span if available.
- Eval errors map to evaluation stage.
- Callback resolver errors map to resolver stage.
- Invalid callback value kinds map to invalid-value stage.
- Messages are human-readable and contain no interior NUL bytes.
```

Recommended commit:

```text
Map core and callback failures to FFI error objects
```

---

# Phase 4: Add Extended No-Variable Evaluation

## Step 4.1: Add internal helper returning richer failure

Current no-variable implementation likely returns only:

```rust
Result<ShyValue, ShyStatusCode>
```

Add a richer internal result type:

```rust
struct FfiFailure {
    status: ShyStatusCode,
    error: ShyError,
}
```

Then:

```rust
fn evaluate_no_vars_ex_impl(expression: &str) -> Result<ShyValue, FfiFailure> {
    let variables = HashMap::new();

    match shunting_yard::evaluate_detailed(expression, &variables) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(error) => {
            let parts = error_parts_from_core_error(error);
            Err(FfiFailure {
                status: parts.status,
                error: parts.into_ffi_error(),
            })
        }
    }
}
```

## Step 4.2: Add exported extended function

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_no_vars_ex(
    expression: *const c_char,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatusCode {
    ffi_boundary_code(|| {
        clear_optional_error(out_error);

        if expression.is_null() || out_value.is_null() {
            let error = null_pointer_error();
            write_optional_error(out_error, error);
            return SHY_STATUS_NULL_POINTER;
        }

        let expression = unsafe { CStr::from_ptr(expression) };

        let expression = match expression.to_str() {
            Ok(expression) => expression,
            Err(_) => {
                write_optional_error(out_error, invalid_utf8_error());
                return SHY_STATUS_INVALID_UTF8;
            }
        };

        match evaluate_no_vars_ex_impl(expression) {
            Ok(value) => {
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(failure) => {
                write_optional_error(out_error, failure.error);
                failure.status
            }
        }
    })
}
```

Note:

```text
Use raw status code return type if the branch has already converted FFI statuses
to int32_t. If not, do that before this feature.
```

## Step 4.3: Preserve old function

Update old function to delegate:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_no_vars(
    expression: *const c_char,
    out_value: *mut ShyValue,
) -> ShyStatusCode {
    unsafe { shy_evaluate_no_vars_ex(expression, out_value, std::ptr::null_mut()) }
}
```

Acceptance criteria:

```text
- Existing shy_evaluate_no_vars behavior is preserved.
- New shy_evaluate_no_vars_ex returns detailed errors through out_error.
- out_error may be NULL.
- On success, out_error is set to NULL when provided.
- On failure, out_value is not modified.
- On failure, out_error receives an allocated ShyError when provided.
- Caller can free the error with shy_error_free.
```

Recommended commit:

```text
Add extended no-variable FFI evaluation with error objects
```

---

# Phase 5: Add Extended Callback Evaluation

## Step 5.1: Extend callback implementation to return `FfiFailure`

Current callback evaluation probably tracks:

```rust
resolver.last_status
```

Extend it to track:

```rust
last_error_parts: Option<ErrorParts>
```

For callback non-OK:

```rust
self.last_error_parts = Some(resolver_error_parts(status));
```

For invalid value kind:

```rust
self.last_error_parts = Some(invalid_value_error_parts());
```

Then:

```rust
fn evaluate_with_callback_ex_impl(...) -> Result<ShyValue, FfiFailure> {
    let mut resolver = FfiResolver { ... };

    match shunting_yard::evaluate_with_resolver_detailed(expression, &mut resolver) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(error) => {
            let parts = resolver
                .last_error_parts
                .unwrap_or_else(|| error_parts_from_core_error(error));

            let status = parts.status;

            Err(FfiFailure {
                status,
                error: parts.into_ffi_error(),
            })
        }
    }
}
```

## Step 5.2: Add exported extended callback function

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_with_callback_ex(
    expression: *const c_char,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatusCode {
    ffi_boundary_code(|| {
        clear_optional_error(out_error);

        if expression.is_null() || out_value.is_null() {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        }

        let Some(resolver) = resolver else {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        };

        ...
    })
}
```

## Step 5.3: Preserve old callback function

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_with_callback(
    expression: *const c_char,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatusCode {
    unsafe {
        shy_evaluate_with_callback_ex(
            expression,
            resolver,
            user_data,
            out_value,
            std::ptr::null_mut(),
        )
    }
}
```

Acceptance criteria:

```text
- Existing shy_evaluate_with_callback behavior is preserved.
- New extended function returns error details through out_error.
- Null callback produces an error object when requested.
- Callback failure produces resolver-stage error.
- Invalid callback value kind produces invalid-value-stage error.
- Core evaluation errors still produce evaluation-stage error.
- Callback-provided non-finite/subnormal floats produce evaluation-stage error.
```

Recommended commit:

```text
Add extended callback FFI evaluation with error objects
```

---

# Phase 6: Rust-Side Tests

Add tests to:

```text
crates/shunting-yard-ffi/tests/ffi_api.rs
```

## Helper

Create RAII helper for tests:

```rust
struct ErrorHandle(*mut ShyError);

impl Drop for ErrorHandle {
    fn drop(&mut self) {
        unsafe { shy_error_free(self.0) };
    }
}
```

Or manually call `shy_error_free`.

## Required tests

### Error accessors handle NULL

```rust
#[test]
fn error_accessors_handle_null() {
    assert_eq!(unsafe { shy_error_status(ptr::null()) }, SHY_STATUS_NULL_POINTER);
    assert_eq!(unsafe { shy_error_stage(ptr::null()) }, SHY_ERROR_STAGE_NONE);
    assert_eq!(unsafe { shy_error_code(ptr::null()) }, SHY_ERROR_CODE_NULL_POINTER);
    assert!(unsafe { shy_error_message(ptr::null()) }.is_null());
    assert_eq!(unsafe { shy_error_has_span(ptr::null()) }, 0);
    assert_eq!(unsafe { shy_error_span_start(ptr::null()) }, -1);
    assert_eq!(unsafe { shy_error_span_end(ptr::null()) }, -1);
    assert_eq!(unsafe { shy_error_diagnostic_count(ptr::null()) }, 0);
}
```

### Success clears out_error

```rust
#[test]
fn no_vars_ex_success_clears_error() {
    let expression = c_string("1 + 2");
    let mut value = default_test_value();
    let mut error = std::ptr::dangling_mut();

    let status = unsafe {
        shy_evaluate_no_vars_ex(expression.as_ptr(), &mut value, &mut error)
    };

    assert_eq!(status, SHY_STATUS_OK);
    assert!(error.is_null());
}
```

Avoid `dangling_mut` if lint/toolchain dislikes it; initialize to a non-null sentinel carefully or just initialize to null and assert it remains null.

### Lexical error produces span

```rust
#[test]
fn no_vars_ex_lexical_error_has_span() {
    let expression = c_string("$");
    let mut value = default_test_value();
    let mut error = ptr::null_mut();

    let status = unsafe {
        shy_evaluate_no_vars_ex(expression.as_ptr(), &mut value, &mut error)
    };

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert!(!error.is_null());

    assert_eq!(unsafe { shy_error_stage(error) }, SHY_ERROR_STAGE_LEXICAL);
    assert_eq!(unsafe { shy_error_code(error) }, SHY_ERROR_CODE_LEXICAL_ERROR);
    assert_eq!(unsafe { shy_error_has_span(error) }, 1);
    assert_eq!(unsafe { shy_error_span_start(error) }, 0);
    assert_eq!(unsafe { shy_error_span_end(error) }, 1);

    unsafe { shy_error_free(error) };
}
```

### Parse error diagnostic count

Use:

```text
1 +
```

Assert:

```text
stage == PARSE
diagnostic_count >= 1
```

### Resource limit error

Call extended function through an internal helper if public FFI does not expose options.

If public FFI has no options, skip resource-limit FFI test for now or create a very long input that exceeds default `max_input_bytes`.

Example:

```rust
let expression = "1".repeat(16 * 1024 + 1);
```

Assert:

```text
stage == RESOURCE_LIMIT
code == INPUT_TOO_LARGE
```

### Evaluation error: division by zero

```rust
stage == EVALUATION
code == DIVISION_BY_ZERO
message != NULL
```

### Callback resolver error

```rust
stage == RESOLVER
code == RESOLVER_ERROR
status == RESOLVER_ERROR
```

### Invalid callback value kind

```rust
stage == INVALID_VALUE
code == INVALID_VALUE_KIND
status == INVALID_VALUE
```

### Callback non-finite float

```rust
stage == EVALUATION
code == NON_FINITE_FLOAT
```

### Null callback produces error object

```text
status == NULL_POINTER
stage == INPUT
code == NULL_POINTER
```

### out_error may be NULL

Call failure with `out_error = NULL` and assert:

```text
- correct status returned;
- no crash.
```

### Message pointer lifetime

```rust
let message = unsafe { shy_error_message(error) };
assert!(!message.is_null());
let message = unsafe { CStr::from_ptr(message) };
assert!(!message.to_bytes().is_empty());
```

Then free. Do not use message after free.

Acceptance criteria:

```text
- Rust tests cover null accessors.
- Rust tests cover success clearing out_error.
- Rust tests cover lexical span.
- Rust tests cover parse diagnostic count.
- Rust tests cover input-too-large resource error if practical.
- Rust tests cover division by zero.
- Rust tests cover resolver error.
- Rust tests cover invalid callback value kind.
- Rust tests cover non-finite callback float.
- Rust tests cover out_error = NULL failure path.
- Rust tests free every allocated error exactly once.
```

Recommended commit:

```text
Test FFI error objects from Rust
```

---

# Phase 7: C Header and C Smoke Tests

## Step 7.1: Update C header

Add:

```c
typedef struct ShyError ShyError;
```

Add constants.

Add function declarations:

```c
ShyStatus shy_evaluate_no_vars_ex(
    const char *expression,
    ShyValue *out_value,
    ShyError **out_error
);

ShyStatus shy_evaluate_with_callback_ex(
    const char *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value,
    ShyError **out_error
);

void shy_error_free(ShyError *error);

ShyStatus shy_error_status(const ShyError *error);
int32_t shy_error_stage(const ShyError *error);
int32_t shy_error_code(const ShyError *error);
const char *shy_error_message(const ShyError *error);
int32_t shy_error_has_span(const ShyError *error);
int32_t shy_error_span_start(const ShyError *error);
int32_t shy_error_span_end(const ShyError *error);
int32_t shy_error_diagnostic_count(const ShyError *error);
```

Document:

```text
- error object ownership;
- error pointer lifetime;
- message pointer lifetime;
- caller must free error with shy_error_free;
- out_error may be NULL;
- on success, *out_error is set to NULL when out_error is provided.
```

## Step 7.2: Update C smoke test

Add tests:

### No-vars error object

```c
static void test_no_vars_ex_reports_division_by_zero(void) {
    ShyValue value = {0};
    ShyError *error = NULL;

    ShyStatus status = shy_evaluate_no_vars_ex("1 / 0", &value, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_EVALUATION);
    assert(shy_error_code(error) == SHY_ERROR_CODE_DIVISION_BY_ZERO);
    assert(shy_error_message(error) != NULL);

    shy_error_free(error);
}
```

### Lexical span

```c
static void test_no_vars_ex_reports_lexical_span(void) {
    ShyValue value = {0};
    ShyError *error = NULL;

    ShyStatus status = shy_evaluate_no_vars_ex("$", &value, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_LEXICAL);
    assert(shy_error_has_span(error) == 1);
    assert(shy_error_span_start(error) == 0);
    assert(shy_error_span_end(error) == 1);

    shy_error_free(error);
}
```

### Callback resolver error

```c
static void test_callback_ex_reports_resolver_error(void) {
    ShyValue value = {0};
    ShyError *error = NULL;

    ShyStatus status = shy_evaluate_with_callback_ex(
        "x",
        failing_resolver,
        NULL,
        &value,
        &error
    );

    assert(status == SHY_STATUS_RESOLVER_ERROR);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_RESOLVER);
    assert(shy_error_code(error) == SHY_ERROR_CODE_RESOLVER_ERROR);

    shy_error_free(error);
}
```

### out_error may be NULL

```c
static void test_ex_error_output_may_be_null(void) {
    ShyValue value = {0};

    ShyStatus status = shy_evaluate_no_vars_ex("1 / 0", &value, NULL);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
}
```

### Success clears error

```c
static void test_ex_success_clears_error(void) {
    ShyValue value = {0};
    ShyError *error = (ShyError *)1;

    ShyStatus status = shy_evaluate_no_vars_ex("1 + 2", &value, &error);

    assert(status == SHY_STATUS_OK);
    assert(error == NULL);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 3);
}
```

Be careful with `(ShyError *)1` if sanitizers complain. If so, initialize to `NULL` and only assert it is `NULL`.

Acceptance criteria:

```text
- C smoke test exercises error object allocation.
- C smoke test exercises error accessors.
- C smoke test frees every allocated error.
- C smoke test checks lexical span.
- C smoke test checks evaluation error code.
- C smoke test checks callback resolver error code.
- C smoke test checks out_error = NULL.
- C smoke test compiles with -Wall -Wextra -Werror.
```

Recommended commit:

```text
Expose FFI error reporting in C smoke test
```

---

# Phase 8: Documentation

## Update `guidelines/ffi_adapter_plan.md`

Add:

```markdown
## Error Reporting

Extended FFI entrypoints with `_ex` suffix can return an owned `ShyError`
object through `ShyError **out_error`.

If `out_error` is `NULL`, no error object is allocated.

If an error object is returned, the caller owns it and must release it with
`shy_error_free`.

Pointers returned by `shy_error_message` are borrowed from the error object and
remain valid only until `shy_error_free`.

The ABI exposes stable integer stage and code constants rather than Rust enums.
```

Update current ABI list:

```markdown
- `shy_evaluate_no_vars`
- `shy_evaluate_no_vars_ex`
- `shy_evaluate_with_callback`
- `shy_evaluate_with_callback_ex`
- `shy_error_*`
```

Update limitations:

```markdown
- Full parse diagnostic iteration is not exposed yet.
- Error messages are human-readable, not a stable machine-readable format.
- Error object allocation uses Rust's allocator and must be freed by Rust.
```

## Update `guidelines/verification_and_ci_plan.md`

Add:

```markdown
FFI error reporting checks:

- extended entrypoints must support `out_error = NULL`;
- error objects returned to C must be freed with `shy_error_free`;
- C smoke tests must cover allocation, accessors, and free;
- Rust tests must cover null accessor behavior;
- message pointers are valid only until `shy_error_free`;
- no Rust-owned error memory may be freed by C directly.
```

## Update README

Add a brief FFI section or update the workspace layout note:

```markdown
The FFI crate exposes simple status-only entrypoints and extended `_ex`
entrypoints that can return an owned `ShyError`. C callers must release returned
errors with `shy_error_free`.
```

Acceptance criteria:

```text
- FFI error ownership is documented.
- Message pointer lifetime is documented.
- `out_error = NULL` behavior is documented.
- Existing simple APIs and new extended APIs are both documented.
```

Recommended commit:

```text
Document FFI error object ownership
```

---

# Phase 9: Verification

Run:

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo test --release --workspace --all-targets --all-features
cargo test --doc --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo audit
cargo deny check
```

Run fuzz build:

```bash
cd crates/shunting-yard/fuzz
cargo fuzz build evaluate_no_panic
cd ../../..
```

Run C smoke:

```bash
bash ./c-tests/run-smoke.sh
```

Optional smoke fuzz:

```bash
cd crates/shunting-yard/fuzz
cargo fuzz run evaluate_no_panic -- -max_total_time=60
cd ../../..
```

If ASan is available locally:

```bash
RUSTFLAGS="-Zsanitizer=address" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu --workspace --all-targets --all-features
```

Acceptance criteria:

```text
- All workspace checks pass.
- All FFI Rust tests pass.
- C smoke test passes.
- Fuzz target builds.
- No unsafe code is added to the core crate.
- New unsafe blocks remain in the FFI crate and have SAFETY comments.
- No memory leaks are detected in CI-relevant tests where tooling supports it.
```

---

# Suggested Commit Breakdown

## Commit 1: Add opaque FFI error type and constants

```text
Add opaque FFI error type and error constants
```

## Commit 2: Add error accessors and free function

```text
Add FFI error accessors and free function
```

## Commit 3: Map core errors into FFI error objects

```text
Map core errors into FFI error objects
```

## Commit 4: Add extended no-variable evaluation

```text
Add extended no-variable FFI evaluation with error reporting
```

## Commit 5: Add extended callback evaluation

```text
Add extended callback FFI evaluation with error reporting
```

## Commit 6: Test FFI error objects from Rust

```text
Test FFI error objects from Rust
```

## Commit 7: Extend C smoke tests for error reporting

```text
Test FFI error reporting from C
```

## Commit 8: Document FFI error ownership

```text
Document FFI error object ownership
```

---

# Full Definition of Done

This feature is complete when:

```text
ABI:
- ShyError is opaque in the public C header.
- Extended _ex functions exist for no-variable evaluation.
- Extended _ex functions exist for callback-backed evaluation.
- Existing non-_ex functions remain available.
- Existing non-_ex behavior is preserved.
- Error stage constants are exposed.
- Error code constants are exposed.
- Error status/code/stage values are raw integer ABI values, not Rust enum fields.

Ownership:
- ShyError objects are allocated by Rust.
- ShyError objects are freed only by shy_error_free.
- shy_error_free(NULL) is safe.
- Message pointers are borrowed from ShyError.
- Message pointers are valid only until shy_error_free.
- out_error may be NULL.
- On success, *out_error is set to NULL when provided.
- On failure, *out_error receives an allocated error when provided.
- On failure with out_error = NULL, no error object is allocated.

Accessors:
- shy_error_status exists.
- shy_error_stage exists.
- shy_error_code exists.
- shy_error_message exists.
- shy_error_has_span exists.
- shy_error_span_start exists.
- shy_error_span_end exists.
- shy_error_diagnostic_count exists.
- Accessors handle NULL deterministically.
- Accessors do not allocate.
- Accessors do not panic.

Error mapping:
- Null pointer maps to input/null-pointer error.
- Invalid UTF-8 maps to input/invalid-UTF8 error.
- Lexical errors map to lexical stage and include span.
- Parse errors map to parse stage and include diagnostic count.
- Resource limits map to resource-limit stage.
- Division by zero maps to evaluation/division-by-zero.
- Integer overflow maps to evaluation/integer-overflow.
- Non-finite float maps to evaluation/non-finite-float.
- Subnormal float maps to evaluation/subnormal-float.
- Unknown variable maps to evaluation/unknown-variable unless caused by callback failure.
- Callback failure maps to resolver stage.
- Invalid callback value kind maps to invalid-value stage.
- Panic maps to panic stage.

Rust tests:
- Null accessor behavior tested.
- Success clears out_error.
- out_error = NULL tested.
- Lexical span tested.
- Parse diagnostic count tested.
- Resource-limit error tested if practical.
- Division by zero tested.
- Callback resolver error tested.
- Invalid callback value kind tested.
- Non-finite callback float tested.
- Error message pointer tested.
- Every allocated error in tests is freed.

C smoke tests:
- Extended no-variable function tested.
- Extended callback function tested.
- Error allocation tested.
- Error accessors tested.
- Error freeing tested.
- Lexical span tested.
- Evaluation error code tested.
- Callback resolver error code tested.
- out_error = NULL tested.
- Smoke test compiles with -Wall -Wextra -Werror.

Safety:
- No unsafe code added to the core crate.
- Unsafe code remains isolated to shunting_yard_ffi.
- Every unsafe block has a SAFETY comment.
- No panic crosses the ABI boundary.
- Rust-owned memory is not freed by C directly.
- C receives no borrowed Rust data except pointers explicitly tied to ShyError lifetime.

Verification:
- cargo fmt passes.
- cargo check --workspace passes.
- cargo test --workspace passes.
- cargo test --release --workspace passes.
- cargo test --doc --workspace passes.
- cargo clippy --workspace --all-targets --all-features -- -D warnings passes.
- cargo audit passes.
- cargo deny check passes.
- fuzz target builds.
- C smoke test passes.
```

---

# Recommended Next Step After This Feature

After FFI error reporting lands, the next likely FFI feature is:

```text
feature/ffi-parsed-expression-handles
```

Goal:

```text
Allow C callers to parse once, hold an opaque parsed-expression handle, evaluate
it many times with no variables or callback-backed variables, and free it
explicitly.
```

That future feature will need:

```text
- ShyParsedExpression opaque handle;
- parse functions returning ShyParsedExpression*;
- evaluate parsed no-vars;
- evaluate parsed with callback;
- ShyParsedExpression free function;
- handle ownership tests;
- C smoke tests for parse once/evaluate many times.
```
