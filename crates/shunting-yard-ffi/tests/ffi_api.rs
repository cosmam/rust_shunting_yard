use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;

use shunting_yard_ffi::{
    SHY_VALUE_BOOL, SHY_VALUE_FLOAT, SHY_VALUE_INTEGER, ShyStatus, ShyValue, ShyValueKind,
    ShyVariableResolver, shy_evaluate_no_vars, shy_evaluate_with_callback,
};

fn default_test_value() -> ShyValue {
    ShyValue {
        kind: SHY_VALUE_INTEGER,
        bool_value: 7,
        integer_value: -1,
        float_value: -1.0,
    }
}

fn c_string(text: &str) -> CString {
    match CString::new(text) {
        Ok(value) => value,
        Err(error) => panic!("test input contains interior NUL: {error}"),
    }
}

fn evaluate(expression: *const std::ffi::c_char, out_value: *mut ShyValue) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers come from NUL-terminated CString values or
    //   local NUL-terminated byte arrays that live for the duration of the call.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    unsafe { shy_evaluate_no_vars(expression, out_value) }
}

fn evaluate_with_callback(
    expression: *const c_char,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers come from NUL-terminated CString values or
    //   local NUL-terminated byte arrays that live for the duration of the call.
    // - Resolver callbacks used by these tests follow the FFI callback contract.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    unsafe { shy_evaluate_with_callback(expression, resolver, user_data, out_value) }
}

unsafe extern "C" fn resolve_x_to_integer(
    name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return ShyStatus::ResolverError;
    }

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: SHY_VALUE_INTEGER,
            bool_value: 0,
            integer_value: 40,
            float_value: 0.0,
        });
    }

    ShyStatus::Ok
}

unsafe extern "C" fn resolve_x_to_bool(
    name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return ShyStatus::ResolverError;
    }

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: SHY_VALUE_BOOL,
            bool_value: 1,
            integer_value: 0,
            float_value: 0.0,
        });
    }

    ShyStatus::Ok
}

unsafe extern "C" fn resolve_x_to_float(
    name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return ShyStatus::ResolverError;
    }

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: SHY_VALUE_FLOAT,
            bool_value: 0,
            integer_value: 0,
            float_value: 1.5,
        });
    }

    ShyStatus::Ok
}

struct TestContext {
    value: i64,
    calls: usize,
}

unsafe extern "C" fn resolve_from_user_data(
    name: *const c_char,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || user_data.is_null() || out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return ShyStatus::ResolverError;
    }

    // SAFETY:
    // - user_data was checked for null above.
    // - this test passes a valid mutable TestContext pointer as user_data.
    let context = unsafe { &mut *user_data.cast::<TestContext>() };
    context.calls += 1;

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: SHY_VALUE_INTEGER,
            bool_value: 0,
            integer_value: context.value,
            float_value: 0.0,
        });
    }

    ShyStatus::Ok
}

unsafe extern "C" fn failing_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    _out_value: *mut ShyValue,
) -> ShyStatus {
    ShyStatus::ResolverError
}

unsafe extern "C" fn invalid_kind_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: 999,
            bool_value: 0,
            integer_value: 0,
            float_value: 0.0,
        });
    }

    ShyStatus::Ok
}

unsafe extern "C" fn infinite_float_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: SHY_VALUE_FLOAT,
            bool_value: 0,
            integer_value: 0,
            float_value: f64::INFINITY,
        });
    }

    ShyStatus::Ok
}

unsafe extern "C" fn subnormal_float_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if out_value.is_null() {
        return ShyStatus::NullPointer;
    }

    // SAFETY:
    // - out_value was checked for null above.
    // - the FFI adapter provides writable storage for one ShyValue.
    unsafe {
        out_value.write(ShyValue {
            kind: SHY_VALUE_FLOAT,
            bool_value: 0,
            integer_value: 0,
            float_value: f64::MIN_POSITIVE / 2.0,
        });
    }

    ShyStatus::Ok
}

#[test]
fn evaluate_no_vars_rejects_null_expression() {
    let mut out = default_test_value();

    let status = evaluate(ptr::null(), &mut out);

    assert_eq!(status, ShyStatus::NullPointer);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_no_vars_rejects_null_output() {
    let expression = c_string("1 + 2");

    let status = evaluate(expression.as_ptr(), ptr::null_mut());

    assert_eq!(status, ShyStatus::NullPointer);
}

#[test]
fn evaluate_no_vars_rejects_invalid_utf8() {
    let bytes = [0xff_u8, 0x00_u8];
    let mut out = default_test_value();

    let status = evaluate(bytes.as_ptr().cast(), &mut out);

    assert_eq!(status, ShyStatus::InvalidUtf8);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_no_vars_returns_integer_value() {
    let expression = c_string("1 + 2");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 3);
    assert_eq!(out.bool_value, 0);
    assert_eq!(out.float_value, 0.0);
}

#[test]
fn evaluate_no_vars_returns_bool_value() {
    let expression = c_string("true");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_BOOL);
    assert_eq!(out.bool_value, 1);
    assert_eq!(out.integer_value, 0);
    assert_eq!(out.float_value, 0.0);
}

#[test]
fn evaluate_no_vars_returns_float_value() {
    let expression = c_string("1.5 + 2.0");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_FLOAT);
    assert_eq!(out.bool_value, 0);
    assert_eq!(out.integer_value, 0);
    assert_eq!(out.float_value, 3.5);
}

#[test]
fn evaluate_no_vars_reports_evaluation_error() {
    let expression = c_string("1 / 0");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, ShyStatus::EvaluationError);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_no_vars_reports_parse_or_lex_error_as_evaluation_error() {
    let expression = c_string("$");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, ShyStatus::EvaluationError);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_null_expression() {
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        ptr::null(),
        Some(resolve_x_to_integer),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::NullPointer);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_null_callback() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();

    let status = evaluate_with_callback(expression.as_ptr(), None, ptr::null_mut(), &mut out);

    assert_eq!(status, ShyStatus::NullPointer);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_null_output() {
    let expression = c_string("x + 2");

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(resolve_x_to_integer),
        ptr::null_mut(),
        ptr::null_mut(),
    );

    assert_eq!(status, ShyStatus::NullPointer);
}

#[test]
fn evaluate_with_callback_rejects_invalid_utf8() {
    let bytes = [0xff_u8, 0x00_u8];
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        bytes.as_ptr().cast(),
        Some(resolve_x_to_integer),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::InvalidUtf8);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_returns_integer_value() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(resolve_x_to_integer),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 42);
    assert_eq!(out.bool_value, 0);
    assert_eq!(out.float_value, 0.0);
}

#[test]
fn evaluate_with_callback_returns_bool_value() {
    let expression = c_string("x");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(resolve_x_to_bool),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_BOOL);
    assert_eq!(out.bool_value, 1);
    assert_eq!(out.integer_value, 0);
    assert_eq!(out.float_value, 0.0);
}

#[test]
fn evaluate_with_callback_returns_float_value() {
    let expression = c_string("x + 2.0");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(resolve_x_to_float),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_FLOAT);
    assert_eq!(out.bool_value, 0);
    assert_eq!(out.integer_value, 0);
    assert_eq!(out.float_value, 3.5);
}

#[test]
fn evaluate_with_callback_passes_user_data_and_supports_repeated_lookups() {
    let expression = c_string("x + x");
    let mut out = default_test_value();
    let mut context = TestContext {
        value: 20,
        calls: 0,
    };

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(resolve_from_user_data),
        ptr::from_mut(&mut context).cast(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::Ok);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 40);
    assert_eq!(context.calls, 2);
}

#[test]
fn evaluate_with_callback_returns_callback_failure_status() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(failing_resolver),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::ResolverError);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_unknown_value_kind() {
    let expression = c_string("x");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(invalid_kind_resolver),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::InvalidValue);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_non_finite_float() {
    let expression = c_string("x");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(infinite_float_resolver),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::EvaluationError);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_subnormal_float() {
    let expression = c_string("x");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(subnormal_float_resolver),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, ShyStatus::EvaluationError);
    assert_eq!(out, default_test_value());
}

#[test]
fn ffi_type_sizes_are_as_expected() {
    assert_eq!(std::mem::size_of::<ShyStatus>(), 4);
    assert_eq!(std::mem::size_of::<ShyValueKind>(), 4);
}
