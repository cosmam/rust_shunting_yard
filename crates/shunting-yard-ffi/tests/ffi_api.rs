use std::ffi::CString;
use std::ptr;

use shunting_yard_ffi::{
    SHY_VALUE_BOOL, SHY_VALUE_FLOAT, SHY_VALUE_INTEGER, ShyStatus, ShyValue, ShyValueKind,
    shy_evaluate_no_vars,
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
fn ffi_type_sizes_are_as_expected() {
    assert_eq!(std::mem::size_of::<ShyStatus>(), 4);
    assert_eq!(std::mem::size_of::<ShyValueKind>(), 4);
}
