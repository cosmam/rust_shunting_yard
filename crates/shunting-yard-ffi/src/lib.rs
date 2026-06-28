//! C ABI adapter for the shunting_yard crate.

#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::{CStr, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Status code returned by FFI functions.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShyStatus {
    /// Operation completed successfully.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// The input C string was not valid UTF-8.
    InvalidUtf8 = 2,
    /// Parsing or evaluation failed.
    EvaluationError = 3,
    /// A Rust panic was caught before crossing the ABI boundary.
    Panic = 4,
    /// The callback returned a value kind that is not recognized.
    InvalidValue = 6,
}

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

impl TryFrom<ShyValue> for shunting_yard::Value {
    type Error = ShyStatus;

    fn try_from(value: ShyValue) -> Result<Self, Self::Error> {
        match value.kind {
            SHY_VALUE_BOOL => Ok(shunting_yard::Value::Bool(value.bool_value != 0)),
            SHY_VALUE_INTEGER => Ok(shunting_yard::Value::Integer(value.integer_value)),
            SHY_VALUE_FLOAT => Ok(shunting_yard::Value::Float(value.float_value)),
            _ => Err(ShyStatus::InvalidValue),
        }
    }
}

fn evaluate_no_vars_impl(expression: &str) -> Result<ShyValue, ShyStatus> {
    let variables = HashMap::new();

    shunting_yard::evaluate_detailed(expression, &variables)
        .map(ShyValue::from_value)
        .map_err(|_error| ShyStatus::EvaluationError)
}

fn ffi_boundary<F>(f: F) -> ShyStatus
where
    F: FnOnce() -> ShyStatus,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(status) => status,
        Err(_) => ShyStatus::Panic,
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
            return ShyStatus::NullPointer;
        }

        // SAFETY:
        // - expression was checked for null above.
        // - caller must provide a valid NUL-terminated C string.
        // - CStr does not take ownership of the pointer.
        let expression = unsafe { CStr::from_ptr(expression) };

        let expression = match expression.to_str() {
            Ok(expression) => expression,
            Err(_) => return ShyStatus::InvalidUtf8,
        };

        match evaluate_no_vars_impl(expression) {
            Ok(value) => {
                // SAFETY:
                // - out_value was checked for null above.
                // - caller must provide valid writable storage for ShyValue.
                unsafe { out_value.write(value) };
                ShyStatus::Ok
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

        assert_eq!(status, ShyStatus::Panic);
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
            Err(ShyStatus::InvalidValue)
        );
    }
}
