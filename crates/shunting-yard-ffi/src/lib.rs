//! C ABI adapter for the shunting_yard crate.

#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// Status code returned by FFI functions.
pub type ShyStatus = i32;

/// Operation completed successfully.
pub const SHY_STATUS_OK: ShyStatus = 0;
/// A required pointer argument was null.
pub const SHY_STATUS_NULL_POINTER: ShyStatus = 1;
/// The input C string was not valid UTF-8.
pub const SHY_STATUS_INVALID_UTF8: ShyStatus = 2;
/// Parsing or evaluation failed.
pub const SHY_STATUS_EVALUATION_ERROR: ShyStatus = 3;
/// A Rust panic was caught before crossing the ABI boundary.
pub const SHY_STATUS_PANIC: ShyStatus = 4;
/// A variable resolver callback failed to provide a value.
pub const SHY_STATUS_RESOLVER_ERROR: ShyStatus = 5;
/// The callback returned a value kind that is not recognized.
pub const SHY_STATUS_INVALID_VALUE: ShyStatus = 6;

/// Boolean value kind at the C ABI boundary.
pub const SHY_VALUE_BOOL: i32 = 0;
/// Signed integer value kind at the C ABI boundary.
pub const SHY_VALUE_INTEGER: i32 = 1;
/// Floating-point value kind at the C ABI boundary.
pub const SHY_VALUE_FLOAT: i32 = 2;

/// No error stage is available.
pub const SHY_ERROR_STAGE_NONE: i32 = 0;
/// Failure happened while validating FFI input.
pub const SHY_ERROR_STAGE_INPUT: i32 = 1;
/// Failure happened during lexical analysis.
pub const SHY_ERROR_STAGE_LEXICAL: i32 = 2;
/// Failure happened during parsing.
pub const SHY_ERROR_STAGE_PARSE: i32 = 3;
/// A configured resource limit was exceeded.
pub const SHY_ERROR_STAGE_RESOURCE_LIMIT: i32 = 4;
/// Failure happened during evaluation.
pub const SHY_ERROR_STAGE_EVALUATION: i32 = 5;
/// Failure came from a variable resolver callback.
pub const SHY_ERROR_STAGE_RESOLVER: i32 = 6;
/// A Rust panic was caught at the FFI boundary.
pub const SHY_ERROR_STAGE_PANIC: i32 = 7;
/// A callback returned an invalid FFI value.
pub const SHY_ERROR_STAGE_INVALID_VALUE: i32 = 8;

/// No error code is available.
pub const SHY_ERROR_CODE_NONE: i32 = 0;
/// A required pointer argument was null.
pub const SHY_ERROR_CODE_NULL_POINTER: i32 = 1;
/// The input C string was not valid UTF-8.
pub const SHY_ERROR_CODE_INVALID_UTF8: i32 = 2;
/// A Rust panic was caught at the FFI boundary.
pub const SHY_ERROR_CODE_PANIC: i32 = 3;
/// The lexer rejected the source text.
pub const SHY_ERROR_CODE_LEXICAL_ERROR: i32 = 100;
/// The parser rejected the source text.
pub const SHY_ERROR_CODE_PARSE_ERROR: i32 = 200;
/// The parser recovered from malformed source text.
pub const SHY_ERROR_CODE_PARSE_RECOVERY: i32 = 201;
/// A resource limit was exceeded.
pub const SHY_ERROR_CODE_RESOURCE_LIMIT: i32 = 300;
/// Input exceeded the maximum byte length.
pub const SHY_ERROR_CODE_INPUT_TOO_LARGE: i32 = 301;
/// Token count exceeded the configured maximum.
pub const SHY_ERROR_CODE_TOO_MANY_TOKENS: i32 = 302;
/// AST node count exceeded the configured maximum.
pub const SHY_ERROR_CODE_AST_TOO_LARGE: i32 = 303;
/// AST nesting depth exceeded the configured maximum.
pub const SHY_ERROR_CODE_EXPRESSION_TOO_DEEP: i32 = 304;
/// A function call had too many arguments.
pub const SHY_ERROR_CODE_TOO_MANY_FUNCTION_ARGUMENTS: i32 = 305;
/// Parser recovery count exceeded the configured maximum.
pub const SHY_ERROR_CODE_TOO_MANY_PARSER_RECOVERIES: i32 = 306;
/// Evaluation failed without a more specific stable code.
pub const SHY_ERROR_CODE_EVAL_ERROR: i32 = 400;
/// An operator or function had an invalid operand count.
pub const SHY_ERROR_CODE_INVALID_ARITY: i32 = 401;
/// An operator or function received an invalid value type.
pub const SHY_ERROR_CODE_INVALID_TYPE: i32 = 402;
/// Division, modulo, or remainder by zero.
pub const SHY_ERROR_CODE_DIVISION_BY_ZERO: i32 = 403;
/// Checked integer arithmetic overflowed.
pub const SHY_ERROR_CODE_INTEGER_OVERFLOW: i32 = 404;
/// A shift count was negative or too large.
pub const SHY_ERROR_CODE_INVALID_SHIFT_COUNT: i32 = 405;
/// An exponent was invalid.
pub const SHY_ERROR_CODE_INVALID_EXPONENT: i32 = 406;
/// A rounding precision was invalid.
pub const SHY_ERROR_CODE_INVALID_PRECISION: i32 = 407;
/// A floating-point result was NaN or infinite.
pub const SHY_ERROR_CODE_NON_FINITE_FLOAT: i32 = 408;
/// A floating-point result was subnormal.
pub const SHY_ERROR_CODE_SUBNORMAL_FLOAT: i32 = 409;
/// Evaluation encountered an unexpected opcode.
pub const SHY_ERROR_CODE_UNEXPECTED_OPCODE: i32 = 410;
/// A variable name could not be resolved.
pub const SHY_ERROR_CODE_UNKNOWN_VARIABLE: i32 = 411;
/// The expression tree was invalid.
pub const SHY_ERROR_CODE_INVALID_EXPRESSION: i32 = 412;
/// A variable resolver callback failed.
pub const SHY_ERROR_CODE_RESOLVER_ERROR: i32 = 500;
/// A callback returned an unknown ShyValue kind.
pub const SHY_ERROR_CODE_INVALID_VALUE_KIND: i32 = 600;

/// No diagnostic is available for the requested index.
pub const SHY_DIAGNOSTIC_KIND_NONE: i32 = 0;
/// The parser received an invalid token.
pub const SHY_DIAGNOSTIC_KIND_INVALID_TOKEN: i32 = 1;
/// The input ended before the parser could finish an expression.
pub const SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_EOF: i32 = 2;
/// The parser found an unexpected token.
pub const SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_TOKEN: i32 = 3;
/// The parser found extra input after a complete expression.
pub const SHY_DIAGNOSTIC_KIND_EXTRA_TOKEN: i32 = 4;
/// The parser reported a user diagnostic.
pub const SHY_DIAGNOSTIC_KIND_USER: i32 = 5;
/// The parser recovered from malformed source text.
pub const SHY_DIAGNOSTIC_KIND_RECOVERY: i32 = 6;

/// Runtime value kind returned through the C ABI.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShyValueKind {
    /// Boolean value.
    Bool = SHY_VALUE_BOOL,
    /// Signed integer value.
    Integer = SHY_VALUE_INTEGER,
    /// Floating-point value.
    Float = SHY_VALUE_FLOAT,
}

/// Runtime value returned through the C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShyValue {
    /// Active value field.
    pub kind: i32,
    /// Boolean payload. Zero is false; nonzero is true.
    pub bool_value: u8,
    /// Integer payload.
    pub integer_value: i64,
    /// Floating-point payload.
    pub float_value: f64,
}

/// Opaque error object returned by extended FFI entrypoints.
///
/// C callers must release pointers to this type with [`shy_error_free`].
#[repr(C)]
pub struct ShyError {
    status: ShyStatus,
    stage: i32,
    code: i32,
    message: CString,
    has_span: i32,
    span_start: i32,
    span_end: i32,
    diagnostic_count: i32,
}

/// Opaque parsed-expression handle returned by FFI parse entrypoints.
///
/// C callers must release pointers to this type with
/// [`shy_parsed_expression_free`].
#[repr(C)]
pub struct ShyParsedExpression {
    parsed: shunting_yard::ParsedExpression<'static>,
}

/// C callback used to resolve variable names during evaluation.
///
/// The callback receives a NUL-terminated variable name that is valid only for
/// the duration of the call, the caller-provided `user_data` pointer, and a
/// writable output slot. Callback implementations must not unwind across the C
/// ABI boundary.
pub type ShyVariableResolver = Option<
    unsafe extern "C" fn(
        name: *const c_char,
        user_data: *mut c_void,
        out_value: *mut ShyValue,
    ) -> ShyStatus,
>;

type ShyVariableResolverCallback = unsafe extern "C" fn(
    name: *const c_char,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus;

fn status_from_code(code: ShyStatus) -> ShyStatus {
    match code {
        SHY_STATUS_OK
        | SHY_STATUS_NULL_POINTER
        | SHY_STATUS_INVALID_UTF8
        | SHY_STATUS_EVALUATION_ERROR
        | SHY_STATUS_PANIC
        | SHY_STATUS_RESOLVER_ERROR
        | SHY_STATUS_INVALID_VALUE => code,
        _ => SHY_STATUS_RESOLVER_ERROR,
    }
}

fn cstring_lossy_no_nul(message: impl Into<String>) -> CString {
    let sanitized = message.into().replace('\0', "\\0");

    match CString::new(sanitized) {
        Ok(message) => message,
        Err(_) => match CString::new("error") {
            Ok(message) => message,
            Err(_) => {
                // SAFETY: the byte string is static ASCII with no interior NUL.
                unsafe { CString::from_vec_unchecked(b"error".to_vec()) }
            }
        },
    }
}

fn usize_to_i32_saturating(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

struct ErrorParts {
    status: ShyStatus,
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

struct FfiFailure {
    status: ShyStatus,
    error: ShyError,
}

fn allocate_error(error: ShyError) -> *mut ShyError {
    Box::into_raw(Box::new(error))
}

fn allocate_parsed_expression(expression: ShyParsedExpression) -> *mut ShyParsedExpression {
    Box::into_raw(Box::new(expression))
}

fn clear_optional_error(out_error: *mut *mut ShyError) {
    if out_error.is_null() {
        return;
    }

    // SAFETY:
    // - caller must provide writable storage for one ShyError pointer.
    // - out_error was checked for null above.
    unsafe { out_error.write(ptr::null_mut()) };
}

fn write_optional_error(out_error: *mut *mut ShyError, error: ShyError) {
    if out_error.is_null() {
        return;
    }

    let error = allocate_error(error);

    // SAFETY:
    // - caller must provide writable storage for one ShyError pointer.
    // - out_error was checked for null above.
    unsafe { out_error.write(error) };
}

fn clear_parsed_expression_output(out_expression: *mut *mut ShyParsedExpression) {
    if out_expression.is_null() {
        return;
    }

    // SAFETY:
    // - caller must provide writable storage for one ShyParsedExpression pointer.
    // - out_expression was checked for null above.
    unsafe { out_expression.write(ptr::null_mut()) };
}

fn input_error_parts(status: ShyStatus, code: i32, message: &'static str) -> ErrorParts {
    ErrorParts {
        status,
        stage: SHY_ERROR_STAGE_INPUT,
        code,
        message: message.to_owned(),
        span: None,
        diagnostic_count: 0,
    }
}

fn null_pointer_error() -> ShyError {
    input_error_parts(
        SHY_STATUS_NULL_POINTER,
        SHY_ERROR_CODE_NULL_POINTER,
        "required pointer argument was null",
    )
    .into_ffi_error()
}

fn invalid_utf8_error() -> ShyError {
    input_error_parts(
        SHY_STATUS_INVALID_UTF8,
        SHY_ERROR_CODE_INVALID_UTF8,
        "expression was not valid UTF-8",
    )
    .into_ffi_error()
}

fn panic_error() -> ShyError {
    ErrorParts {
        status: SHY_STATUS_PANIC,
        stage: SHY_ERROR_STAGE_PANIC,
        code: SHY_ERROR_CODE_PANIC,
        message: "Rust panic caught at FFI boundary".to_owned(),
        span: None,
        diagnostic_count: 0,
    }
    .into_ffi_error()
}

fn resolver_error_parts(status: ShyStatus) -> ErrorParts {
    ErrorParts {
        status,
        stage: SHY_ERROR_STAGE_RESOLVER,
        code: SHY_ERROR_CODE_RESOLVER_ERROR,
        message: "variable resolver callback failed".to_owned(),
        span: None,
        diagnostic_count: 0,
    }
}

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

fn resource_limit_error_parts(error: shunting_yard::ResourceLimitError) -> ErrorParts {
    let code = match error {
        shunting_yard::ResourceLimitError::InputTooLarge { .. } => SHY_ERROR_CODE_INPUT_TOO_LARGE,
        shunting_yard::ResourceLimitError::TooManyTokens { .. } => SHY_ERROR_CODE_TOO_MANY_TOKENS,
        shunting_yard::ResourceLimitError::AstTooLarge { .. } => SHY_ERROR_CODE_AST_TOO_LARGE,
        shunting_yard::ResourceLimitError::ExpressionTooDeep { .. } => {
            SHY_ERROR_CODE_EXPRESSION_TOO_DEEP
        }
        shunting_yard::ResourceLimitError::TooManyFunctionArguments { .. } => {
            SHY_ERROR_CODE_TOO_MANY_FUNCTION_ARGUMENTS
        }
        shunting_yard::ResourceLimitError::TooManyParserRecoveries { .. } => {
            SHY_ERROR_CODE_TOO_MANY_PARSER_RECOVERIES
        }
    };

    ErrorParts {
        status: SHY_STATUS_EVALUATION_ERROR,
        stage: SHY_ERROR_STAGE_RESOURCE_LIMIT,
        code,
        message: error.to_string(),
        span: None,
        diagnostic_count: 1,
    }
}

fn lexical_error_parts(
    span: Option<shunting_yard::SourceSpan>,
    error: shunting_yard::LexicalError,
) -> ErrorParts {
    ErrorParts {
        status: SHY_STATUS_EVALUATION_ERROR,
        stage: SHY_ERROR_STAGE_LEXICAL,
        code: SHY_ERROR_CODE_LEXICAL_ERROR,
        message: format!("lexical error: {error}"),
        span: span.map(|span| (span.start, span.end)),
        diagnostic_count: 1,
    }
}

fn parse_error_parts(diagnostics: shunting_yard::ParseDiagnostics) -> ErrorParts {
    let diagnostic_count = diagnostics.len();
    let recovery_count = diagnostics.recovery_count();
    let first_span = diagnostics
        .diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic.span.map(|span| (span.start, span.end)));

    let code = if recovery_count > 0 {
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

fn eval_error_parts(error: shunting_yard::EvalError) -> ErrorParts {
    match error {
        shunting_yard::EvalError::ResourceLimit(error) => resource_limit_error_parts(error),
        shunting_yard::EvalError::LexicalError(error) => lexical_error_parts(None, error),
        shunting_yard::EvalError::ParserError => ErrorParts {
            status: SHY_STATUS_EVALUATION_ERROR,
            stage: SHY_ERROR_STAGE_PARSE,
            code: SHY_ERROR_CODE_PARSE_ERROR,
            message: "parser error".to_owned(),
            span: None,
            diagnostic_count: 1,
        },
        shunting_yard::EvalError::ParserRecovery { count } => ErrorParts {
            status: SHY_STATUS_EVALUATION_ERROR,
            stage: SHY_ERROR_STAGE_PARSE,
            code: SHY_ERROR_CODE_PARSE_RECOVERY,
            message: format!("parser recovered from {count} error(s)"),
            span: None,
            diagnostic_count: count,
        },
        error @ shunting_yard::EvalError::InvalidArity { .. } => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INVALID_ARITY)
        }
        error @ shunting_yard::EvalError::InvalidExpression => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INVALID_EXPRESSION)
        }
        error @ shunting_yard::EvalError::InvalidType { .. } => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INVALID_TYPE)
        }
        error @ shunting_yard::EvalError::DivisionByZero => {
            eval_error_code_parts(error, SHY_ERROR_CODE_DIVISION_BY_ZERO)
        }
        error @ shunting_yard::EvalError::IntegerOverflow { .. } => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INTEGER_OVERFLOW)
        }
        error @ shunting_yard::EvalError::InvalidShiftCount { .. } => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INVALID_SHIFT_COUNT)
        }
        error @ shunting_yard::EvalError::InvalidExponent { .. } => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INVALID_EXPONENT)
        }
        error @ shunting_yard::EvalError::InvalidPrecision => {
            eval_error_code_parts(error, SHY_ERROR_CODE_INVALID_PRECISION)
        }
        error @ shunting_yard::EvalError::NonFiniteFloat => {
            eval_error_code_parts(error, SHY_ERROR_CODE_NON_FINITE_FLOAT)
        }
        error @ shunting_yard::EvalError::SubnormalFloat => {
            eval_error_code_parts(error, SHY_ERROR_CODE_SUBNORMAL_FLOAT)
        }
        error @ shunting_yard::EvalError::UnexpectedOpcode => {
            eval_error_code_parts(error, SHY_ERROR_CODE_UNEXPECTED_OPCODE)
        }
        error @ shunting_yard::EvalError::UnknownVariable(_) => {
            eval_error_code_parts(error, SHY_ERROR_CODE_UNKNOWN_VARIABLE)
        }
    }
}

fn eval_error_code_parts(error: shunting_yard::EvalError, code: i32) -> ErrorParts {
    ErrorParts {
        status: SHY_STATUS_EVALUATION_ERROR,
        stage: SHY_ERROR_STAGE_EVALUATION,
        code,
        message: error.to_string(),
        span: None,
        diagnostic_count: 1,
    }
}

fn error_parts_from_core_error(error: shunting_yard::Error) -> ErrorParts {
    match error {
        shunting_yard::Error::ResourceLimit(error) => resource_limit_error_parts(error),
        shunting_yard::Error::Lexical { span, error } => lexical_error_parts(Some(span), error),
        shunting_yard::Error::Parse(diagnostics) => parse_error_parts(diagnostics),
        shunting_yard::Error::Eval(error) => eval_error_parts(error),
    }
}

fn failure_from_parts(parts: ErrorParts) -> FfiFailure {
    let status = parts.status;
    FfiFailure {
        status,
        error: parts.into_ffi_error(),
    }
}

fn ffi_boundary_with_error<F>(out_error: *mut *mut ShyError, f: F) -> ShyStatus
where
    F: FnOnce() -> ShyStatus,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(status) => status,
        Err(_) => {
            write_optional_error(out_error, panic_error());
            SHY_STATUS_PANIC
        }
    }
}

impl ShyValue {
    fn from_value(value: shunting_yard::Value) -> Self {
        match value {
            shunting_yard::Value::Bool(value) => Self {
                kind: SHY_VALUE_BOOL,
                bool_value: u8::from(value),
                integer_value: 0,
                float_value: 0.0,
            },
            shunting_yard::Value::Integer(value) => Self {
                kind: SHY_VALUE_INTEGER,
                bool_value: 0,
                integer_value: value,
                float_value: 0.0,
            },
            shunting_yard::Value::Float(value) => Self {
                kind: SHY_VALUE_FLOAT,
                bool_value: 0,
                integer_value: 0,
                float_value: value,
            },
        }
    }
}

/// Free a parsed-expression handle returned by an FFI parse entrypoint.
///
/// # Safety
///
/// `expression` must be null or a pointer returned by this crate through an
/// `out_expression` parameter. It must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_parsed_expression_free(expression: *mut ShyParsedExpression) {
    if expression.is_null() {
        return;
    }

    // SAFETY:
    // - expression must have been returned by this crate through an out_expression parameter.
    // - Box::from_raw takes ownership and drops the allocation exactly once.
    unsafe { drop(Box::from_raw(expression)) };
}

/// Free an error object returned by an extended FFI entrypoint.
///
/// # Safety
///
/// `error` must be null or a pointer returned by this crate through an
/// `out_error` parameter. It must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_free(error: *mut ShyError) {
    if error.is_null() {
        return;
    }

    // SAFETY:
    // - error must have been returned by this crate through an out_error parameter.
    // - Box::from_raw takes ownership and drops the allocation exactly once.
    unsafe { drop(Box::from_raw(error)) };
}

/// Return the status associated with an error object.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_status(error: *const ShyError) -> ShyStatus {
    if error.is_null() {
        return SHY_STATUS_NULL_POINTER;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).status }
}

/// Return the stage associated with an error object.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_stage(error: *const ShyError) -> i32 {
    if error.is_null() {
        return SHY_ERROR_STAGE_NONE;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).stage }
}

/// Return the stable code associated with an error object.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_code(error: *const ShyError) -> i32 {
    if error.is_null() {
        return SHY_ERROR_CODE_NULL_POINTER;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).code }
}

/// Return a borrowed human-readable error message.
///
/// The returned pointer remains valid until `shy_error_free(error)`.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_message(error: *const ShyError) -> *const c_char {
    if error.is_null() {
        return ptr::null();
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).message.as_ptr() }
}

/// Return nonzero when an error object contains a source span.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_has_span(error: *const ShyError) -> i32 {
    if error.is_null() {
        return 0;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).has_span }
}

/// Return the inclusive start byte offset for an error source span.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_span_start(error: *const ShyError) -> i32 {
    if error.is_null() {
        return -1;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).span_start }
}

/// Return the exclusive end byte offset for an error source span.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_span_end(error: *const ShyError) -> i32 {
    if error.is_null() {
        return -1;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).span_end }
}

/// Return the number of diagnostics represented by an error object.
///
/// # Safety
///
/// `error` must be null or a live pointer returned by this crate through an
/// `out_error` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_error_diagnostic_count(error: *const ShyError) -> i32 {
    if error.is_null() {
        return 0;
    }

    // SAFETY:
    // - error was checked for null above.
    // - caller must pass a live ShyError pointer returned by this crate.
    unsafe { (*error).diagnostic_count }
}

struct FfiResolver {
    callback: ShyVariableResolverCallback,
    user_data: *mut c_void,
    last_error_parts: Option<ErrorParts>,
}

impl FfiResolver {
    fn resolve_callback(
        &mut self,
        name: &str,
    ) -> Result<shunting_yard::Value, shunting_yard::EvalError> {
        let ffi_name =
            CString::new(name).map_err(|_| shunting_yard::EvalError::InvalidExpression)?;

        let mut out_value = ShyValue {
            kind: SHY_VALUE_INTEGER,
            bool_value: 0,
            integer_value: 0,
            float_value: 0.0,
        };

        // SAFETY:
        // - callback was checked non-null before FfiResolver was constructed.
        // - ffi_name.as_ptr() is valid and NUL-terminated for the duration of the call.
        // - user_data is caller-provided and passed through without dereferencing.
        // - out_value points to valid writable storage for one ShyValue.
        let status = unsafe { (self.callback)(ffi_name.as_ptr(), self.user_data, &mut out_value) };

        let status = status_from_code(status);
        if status != SHY_STATUS_OK {
            self.last_error_parts = Some(resolver_error_parts(status));
            return Err(shunting_yard::EvalError::UnknownVariable(name.to_owned()));
        }

        shunting_yard::Value::try_from(out_value).map_err(|status| {
            self.last_error_parts = Some(match status {
                SHY_STATUS_INVALID_VALUE => invalid_value_error_parts(),
                status => resolver_error_parts(status),
            });
            shunting_yard::EvalError::InvalidExpression
        })
    }
}

impl shunting_yard::VariableResolver for &mut FfiResolver {
    fn resolve(&mut self, name: &str) -> Result<shunting_yard::Value, shunting_yard::EvalError> {
        self.resolve_callback(name)
    }
}

impl TryFrom<ShyValue> for shunting_yard::Value {
    type Error = ShyStatus;

    fn try_from(value: ShyValue) -> Result<Self, Self::Error> {
        match value.kind {
            SHY_VALUE_BOOL => Ok(shunting_yard::Value::Bool(value.bool_value != 0)),
            SHY_VALUE_INTEGER => Ok(shunting_yard::Value::Integer(value.integer_value)),
            SHY_VALUE_FLOAT => Ok(shunting_yard::Value::Float(value.float_value)),
            _ => Err(SHY_STATUS_INVALID_VALUE),
        }
    }
}

fn parse_expression_ex_impl(expression: &str) -> Result<ShyParsedExpression, FfiFailure> {
    match shunting_yard::parse_detailed(expression) {
        Ok(parsed) => Ok(ShyParsedExpression {
            parsed: parsed.into_owned(),
        }),
        Err(error) => Err(failure_from_parts(error_parts_from_core_error(error))),
    }
}

fn evaluate_no_vars_ex_impl(expression: &str) -> Result<ShyValue, FfiFailure> {
    let variables = HashMap::new();

    match shunting_yard::evaluate_detailed(expression, &variables) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(error) => Err(failure_from_parts(error_parts_from_core_error(error))),
    }
}

fn evaluate_parsed_no_vars_ex_impl(
    expression: &ShyParsedExpression,
) -> Result<ShyValue, FfiFailure> {
    let variables = HashMap::new();

    match shunting_yard::evaluate_parsed_detailed(&expression.parsed, &variables) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(error) => Err(failure_from_parts(error_parts_from_core_error(error))),
    }
}

fn evaluate_with_callback_ex_impl(
    expression: &str,
    callback: ShyVariableResolverCallback,
    user_data: *mut c_void,
) -> Result<ShyValue, FfiFailure> {
    let mut resolver = FfiResolver {
        callback,
        user_data,
        last_error_parts: None,
    };

    match shunting_yard::evaluate_with_resolver_detailed(expression, &mut resolver) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(error) => {
            let parts = match resolver.last_error_parts {
                Some(parts) => parts,
                None => error_parts_from_core_error(error),
            };
            Err(failure_from_parts(parts))
        }
    }
}

fn evaluate_parsed_with_callback_ex_impl(
    expression: &ShyParsedExpression,
    callback: ShyVariableResolverCallback,
    user_data: *mut c_void,
) -> Result<ShyValue, FfiFailure> {
    let mut resolver = FfiResolver {
        callback,
        user_data,
        last_error_parts: None,
    };

    match shunting_yard::evaluate_parsed_detailed(&expression.parsed, &mut resolver) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(error) => {
            let parts = match resolver.last_error_parts {
                Some(parts) => parts,
                None => error_parts_from_core_error(error),
            };
            Err(failure_from_parts(parts))
        }
    }
}

/// Parse an expression into an owned parsed-expression handle.
///
/// # Safety
///
/// `expression` must be null or point to a valid NUL-terminated C string for
/// the duration of the call. `out_expression` must be null or point to valid,
/// writable storage for one `ShyParsedExpression *`. `out_error` must be null
/// or point to valid, writable storage for one `ShyError *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_parse_expression_ex(
    expression: *const c_char,
    out_expression: *mut *mut ShyParsedExpression,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    ffi_boundary_with_error(out_error, || {
        clear_optional_error(out_error);
        clear_parsed_expression_output(out_expression);

        if expression.is_null() || out_expression.is_null() {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        }

        // SAFETY:
        // - expression was checked for null above.
        // - caller must provide a valid NUL-terminated C string.
        // - CStr does not take ownership of the pointer.
        let expression = unsafe { CStr::from_ptr(expression) };

        let expression = match expression.to_str() {
            Ok(expression) => expression,
            Err(_) => {
                write_optional_error(out_error, invalid_utf8_error());
                return SHY_STATUS_INVALID_UTF8;
            }
        };

        match parse_expression_ex_impl(expression) {
            Ok(parsed) => {
                let parsed = allocate_parsed_expression(parsed);

                // SAFETY:
                // - out_expression was checked for null above.
                // - caller must provide writable storage for one ShyParsedExpression pointer.
                unsafe { out_expression.write(parsed) };
                SHY_STATUS_OK
            }
            Err(failure) => {
                let status = failure.status;
                write_optional_error(out_error, failure.error);
                status
            }
        }
    })
}

/// Parse an expression into an owned parsed-expression handle.
///
/// # Safety
///
/// `expression` must be null or point to a valid NUL-terminated C string for
/// the duration of the call. `out_expression` must be null or point to valid,
/// writable storage for one `ShyParsedExpression *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_parse_expression(
    expression: *const c_char,
    out_expression: *mut *mut ShyParsedExpression,
) -> ShyStatus {
    // SAFETY:
    // - forwards the caller-provided pointers to the extended entrypoint.
    // - passes a null out_error pointer to preserve the status-only ABI.
    unsafe { shy_parse_expression_ex(expression, out_expression, ptr::null_mut()) }
}

/// Evaluate a parsed expression that does not require variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a live parsed-expression handle
/// returned by this crate. `out_value` must be null or point to valid, writable
/// storage for one [`ShyValue`]. `out_error` must be null or point to valid,
/// writable storage for one `ShyError *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_parsed_no_vars_ex(
    expression: *const ShyParsedExpression,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    ffi_boundary_with_error(out_error, || {
        clear_optional_error(out_error);

        if expression.is_null() || out_value.is_null() {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        }

        // SAFETY:
        // - expression was checked for null above.
        // - caller must pass a live handle returned by this crate.
        let expression = unsafe { &*expression };

        match evaluate_parsed_no_vars_ex_impl(expression) {
            Ok(value) => {
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(failure) => {
                let status = failure.status;
                write_optional_error(out_error, failure.error);
                status
            }
        }
    })
}

/// Evaluate a parsed expression that does not require variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a live parsed-expression handle
/// returned by this crate. `out_value` must be null or point to valid, writable
/// storage for one [`ShyValue`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_parsed_no_vars(
    expression: *const ShyParsedExpression,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - forwards the caller-provided pointers to the extended entrypoint.
    // - passes a null out_error pointer to preserve the status-only ABI.
    unsafe { shy_evaluate_parsed_no_vars_ex(expression, out_value, ptr::null_mut()) }
}

/// Evaluate a parsed expression using a C callback for variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a live parsed-expression handle
/// returned by this crate. `resolver` must be null or a valid function pointer
/// that follows the [`ShyVariableResolver`] contract. `user_data` is
/// caller-owned and is passed through without being dereferenced. `out_value`
/// must be null or point to valid, writable storage for one [`ShyValue`].
/// `out_error` must be null or point to valid, writable storage for one
/// `ShyError *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_parsed_with_callback_ex(
    expression: *const ShyParsedExpression,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    ffi_boundary_with_error(out_error, || {
        clear_optional_error(out_error);

        if expression.is_null() || out_value.is_null() {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        }

        let Some(resolver) = resolver else {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        };

        // SAFETY:
        // - expression was checked for null above.
        // - caller must pass a live handle returned by this crate.
        let expression = unsafe { &*expression };

        match evaluate_parsed_with_callback_ex_impl(expression, resolver, user_data) {
            Ok(value) => {
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(failure) => {
                let status = failure.status;
                write_optional_error(out_error, failure.error);
                status
            }
        }
    })
}

/// Evaluate a parsed expression using a C callback for variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a live parsed-expression handle
/// returned by this crate. `resolver` must be null or a valid function pointer
/// that follows the [`ShyVariableResolver`] contract. `user_data` is
/// caller-owned and is passed through without being dereferenced. `out_value`
/// must be null or point to valid, writable storage for one [`ShyValue`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_parsed_with_callback(
    expression: *const ShyParsedExpression,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - forwards the caller-provided pointers to the extended entrypoint.
    // - passes a null out_error pointer to preserve the status-only ABI.
    unsafe {
        shy_evaluate_parsed_with_callback_ex(
            expression,
            resolver,
            user_data,
            out_value,
            ptr::null_mut(),
        )
    }
}

/// Evaluate an expression that does not require variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a valid NUL-terminated C string for
/// the duration of the call. `out_value` must be null or point to valid,
/// writable storage for one [`ShyValue`]. `out_error` must be null or point to
/// valid, writable storage for one `ShyError *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_no_vars_ex(
    expression: *const c_char,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    ffi_boundary_with_error(out_error, || {
        clear_optional_error(out_error);

        if expression.is_null() || out_value.is_null() {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        }

        // SAFETY:
        // - expression was checked for null above.
        // - caller must provide a valid NUL-terminated C string.
        // - CStr does not take ownership of the pointer.
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
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(failure) => {
                let status = failure.status;
                write_optional_error(out_error, failure.error);
                status
            }
        }
    })
}

/// Evaluate an expression that does not require variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a valid NUL-terminated C string for
/// the duration of the call. `out_value` must be null or point to valid,
/// writable storage for one [`ShyValue`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_no_vars(
    expression: *const c_char,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - forwards the caller-provided pointers to the extended entrypoint.
    // - passes a null out_error pointer to preserve the status-only ABI.
    unsafe { shy_evaluate_no_vars_ex(expression, out_value, ptr::null_mut()) }
}

/// Evaluate an expression using a C callback for variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a valid NUL-terminated C string for
/// the duration of the call. `resolver` must be null or a valid function
/// pointer that follows the [`ShyVariableResolver`] contract. `user_data` is
/// caller-owned and is passed through without being dereferenced. `out_value`
/// must be null or point to valid, writable storage for one [`ShyValue`].
/// `out_error` must be null or point to valid, writable storage for one
/// `ShyError *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_with_callback_ex(
    expression: *const c_char,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    ffi_boundary_with_error(out_error, || {
        clear_optional_error(out_error);

        if expression.is_null() || out_value.is_null() {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        }

        let Some(resolver) = resolver else {
            write_optional_error(out_error, null_pointer_error());
            return SHY_STATUS_NULL_POINTER;
        };

        // SAFETY:
        // - expression was checked for null above.
        // - caller must provide a valid NUL-terminated C string.
        // - CStr does not take ownership of the pointer.
        let expression = unsafe { CStr::from_ptr(expression) };

        let expression = match expression.to_str() {
            Ok(expression) => expression,
            Err(_) => {
                write_optional_error(out_error, invalid_utf8_error());
                return SHY_STATUS_INVALID_UTF8;
            }
        };

        match evaluate_with_callback_ex_impl(expression, resolver, user_data) {
            Ok(value) => {
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(failure) => {
                let status = failure.status;
                write_optional_error(out_error, failure.error);
                status
            }
        }
    })
}

/// Evaluate an expression using a C callback for variable lookup.
///
/// # Safety
///
/// `expression` must be null or point to a valid NUL-terminated C string for
/// the duration of the call. `resolver` must be null or a valid function
/// pointer that follows the [`ShyVariableResolver`] contract. `user_data` is
/// caller-owned and is passed through without being dereferenced. `out_value`
/// must be null or point to valid, writable storage for one [`ShyValue`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shy_evaluate_with_callback(
    expression: *const c_char,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - forwards the caller-provided pointers to the extended entrypoint.
    // - passes a null out_error pointer to preserve the status-only ABI.
    unsafe {
        shy_evaluate_with_callback_ex(expression, resolver, user_data, out_value, ptr::null_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_boundary_with_error_converts_panic_to_status() {
        let status = ffi_boundary_with_error(ptr::null_mut(), || panic!("intentional test panic"));

        assert_eq!(status, SHY_STATUS_PANIC);
    }

    #[test]
    fn shy_value_converts_to_core_value() {
        assert_eq!(
            shunting_yard::Value::try_from(ShyValue {
                kind: SHY_VALUE_BOOL,
                bool_value: 1,
                integer_value: 0,
                float_value: 0.0,
            }),
            Ok(shunting_yard::Value::Bool(true))
        );
        assert_eq!(
            shunting_yard::Value::try_from(ShyValue {
                kind: SHY_VALUE_INTEGER,
                bool_value: 0,
                integer_value: 42,
                float_value: 0.0,
            }),
            Ok(shunting_yard::Value::Integer(42))
        );
        assert_eq!(
            shunting_yard::Value::try_from(ShyValue {
                kind: SHY_VALUE_FLOAT,
                bool_value: 0,
                integer_value: 0,
                float_value: 3.5,
            }),
            Ok(shunting_yard::Value::Float(3.5))
        );
    }

    #[test]
    fn shy_value_rejects_unknown_kind() {
        assert_eq!(
            shunting_yard::Value::try_from(ShyValue {
                kind: 999,
                bool_value: 0,
                integer_value: 0,
                float_value: 0.0,
            }),
            Err(SHY_STATUS_INVALID_VALUE)
        );
    }
}
