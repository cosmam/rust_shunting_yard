//! C ABI adapter for the shunting_yard crate.

#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

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
