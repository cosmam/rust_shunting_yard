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
    last_status: Option<ShyStatus>,
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
            self.last_status = Some(status);
            return Err(shunting_yard::EvalError::UnknownVariable(name.to_owned()));
        }

        shunting_yard::Value::try_from(out_value).map_err(|status| {
            self.last_status = Some(status);
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

fn evaluate_no_vars_impl(expression: &str) -> Result<ShyValue, ShyStatus> {
    let variables = HashMap::new();

    shunting_yard::evaluate_detailed(expression, &variables)
        .map(ShyValue::from_value)
        .map_err(|_error| SHY_STATUS_EVALUATION_ERROR)
}

fn evaluate_with_callback_impl(
    expression: &str,
    callback: ShyVariableResolverCallback,
    user_data: *mut c_void,
) -> Result<ShyValue, ShyStatus> {
    let mut resolver = FfiResolver {
        callback,
        user_data,
        last_status: None,
    };

    match shunting_yard::evaluate_with_resolver_detailed(expression, &mut resolver) {
        Ok(value) => Ok(ShyValue::from_value(value)),
        Err(_error) => Err(resolver.last_status.unwrap_or(SHY_STATUS_EVALUATION_ERROR)),
    }
}

fn ffi_boundary<F>(f: F) -> ShyStatus
where
    F: FnOnce() -> ShyStatus,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(status) => status,
        Err(_) => SHY_STATUS_PANIC,
    }
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
    ffi_boundary(|| {
        if expression.is_null() || out_value.is_null() {
            return SHY_STATUS_NULL_POINTER;
        }

        // SAFETY:
        // - expression was checked for null above.
        // - caller must provide a valid NUL-terminated C string.
        // - CStr does not take ownership of the pointer.
        let expression = unsafe { CStr::from_ptr(expression) };

        let expression = match expression.to_str() {
            Ok(expression) => expression,
            Err(_) => return SHY_STATUS_INVALID_UTF8,
        };

        match evaluate_no_vars_impl(expression) {
            Ok(value) => {
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(status) => status,
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
    ffi_boundary(|| {
        if expression.is_null() || out_value.is_null() {
            return SHY_STATUS_NULL_POINTER;
        }

        let Some(resolver) = resolver else {
            return SHY_STATUS_NULL_POINTER;
        };

        // SAFETY:
        // - expression was checked for null above.
        // - caller must provide a valid NUL-terminated C string.
        // - CStr does not take ownership of the pointer.
        let expression = unsafe { CStr::from_ptr(expression) };

        let expression = match expression.to_str() {
            Ok(expression) => expression,
            Err(_) => return SHY_STATUS_INVALID_UTF8,
        };

        match evaluate_with_callback_impl(expression, resolver, user_data) {
            Ok(value) => {
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                SHY_STATUS_OK
            }
            Err(status) => status,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_boundary_converts_panic_to_status() {
        let status = ffi_boundary(|| panic!("intentional test panic"));

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
