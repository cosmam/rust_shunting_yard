use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;

use shunting_yard_ffi::{
    SHY_ERROR_CODE_DIVISION_BY_ZERO, SHY_ERROR_CODE_INPUT_TOO_LARGE, SHY_ERROR_CODE_INVALID_UTF8,
    SHY_ERROR_CODE_INVALID_VALUE_KIND, SHY_ERROR_CODE_LEXICAL_ERROR,
    SHY_ERROR_CODE_NON_FINITE_FLOAT, SHY_ERROR_CODE_NULL_POINTER, SHY_ERROR_CODE_PARSE_RECOVERY,
    SHY_ERROR_CODE_RESOLVER_ERROR, SHY_ERROR_CODE_SUBNORMAL_FLOAT, SHY_ERROR_STAGE_EVALUATION,
    SHY_ERROR_STAGE_INPUT, SHY_ERROR_STAGE_INVALID_VALUE, SHY_ERROR_STAGE_LEXICAL,
    SHY_ERROR_STAGE_NONE, SHY_ERROR_STAGE_PARSE, SHY_ERROR_STAGE_RESOLVER,
    SHY_ERROR_STAGE_RESOURCE_LIMIT, SHY_STATUS_EVALUATION_ERROR, SHY_STATUS_INVALID_UTF8,
    SHY_STATUS_INVALID_VALUE, SHY_STATUS_NULL_POINTER, SHY_STATUS_OK, SHY_STATUS_RESOLVER_ERROR,
    SHY_VALUE_BOOL, SHY_VALUE_FLOAT, SHY_VALUE_INTEGER, ShyError, ShyParsedExpression, ShyStatus,
    ShyValue, ShyValueKind, ShyVariableResolver, shy_error_code, shy_error_diagnostic_count,
    shy_error_free, shy_error_has_span, shy_error_message, shy_error_span_end,
    shy_error_span_start, shy_error_stage, shy_error_status, shy_evaluate_no_vars,
    shy_evaluate_no_vars_ex, shy_evaluate_parsed_no_vars, shy_evaluate_parsed_no_vars_ex,
    shy_evaluate_parsed_with_callback, shy_evaluate_parsed_with_callback_ex,
    shy_evaluate_with_callback, shy_evaluate_with_callback_ex, shy_parse_expression,
    shy_parse_expression_ex, shy_parsed_expression_free,
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

fn evaluate_ex(
    expression: *const c_char,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers come from NUL-terminated CString values or
    //   local NUL-terminated byte arrays that live for the duration of the call.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    // - Non-null error pointers come from valid mutable ShyError pointer storage.
    unsafe { shy_evaluate_no_vars_ex(expression, out_value, out_error) }
}

fn evaluate_with_callback_ex(
    expression: *const c_char,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers come from NUL-terminated CString values or
    //   local NUL-terminated byte arrays that live for the duration of the call.
    // - Resolver callbacks used by these tests follow the FFI callback contract.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    // - Non-null error pointers come from valid mutable ShyError pointer storage.
    unsafe { shy_evaluate_with_callback_ex(expression, resolver, user_data, out_value, out_error) }
}

fn parse_expression(
    expression: *const c_char,
    out_expression: *mut *mut ShyParsedExpression,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers come from NUL-terminated CString values or
    //   local NUL-terminated byte arrays that live for the duration of the call.
    // - Non-null output pointers come from valid mutable ShyParsedExpression
    //   pointer storage.
    unsafe { shy_parse_expression(expression, out_expression) }
}

fn parse_expression_ex(
    expression: *const c_char,
    out_expression: *mut *mut ShyParsedExpression,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers come from NUL-terminated CString values or
    //   local NUL-terminated byte arrays that live for the duration of the call.
    // - Non-null output pointers come from valid mutable ShyParsedExpression
    //   pointer storage.
    // - Non-null error pointers come from valid mutable ShyError pointer storage.
    unsafe { shy_parse_expression_ex(expression, out_expression, out_error) }
}

fn evaluate_parsed_no_vars(
    expression: *const ShyParsedExpression,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers are live handles returned by this crate.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    unsafe { shy_evaluate_parsed_no_vars(expression, out_value) }
}

fn evaluate_parsed_no_vars_ex(
    expression: *const ShyParsedExpression,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers are live handles returned by this crate.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    // - Non-null error pointers come from valid mutable ShyError pointer storage.
    unsafe { shy_evaluate_parsed_no_vars_ex(expression, out_value, out_error) }
}

fn evaluate_parsed_with_callback(
    expression: *const ShyParsedExpression,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers are live handles returned by this crate.
    // - Resolver callbacks used by these tests follow the FFI callback contract.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    unsafe { shy_evaluate_parsed_with_callback(expression, resolver, user_data, out_value) }
}

fn evaluate_parsed_with_callback_ex(
    expression: *const ShyParsedExpression,
    resolver: ShyVariableResolver,
    user_data: *mut c_void,
    out_value: *mut ShyValue,
    out_error: *mut *mut ShyError,
) -> ShyStatus {
    // SAFETY:
    // - Tests pass null pointers only for API paths that explicitly accept and
    //   reject null pointers before dereferencing.
    // - Non-null expression pointers are live handles returned by this crate.
    // - Resolver callbacks used by these tests follow the FFI callback contract.
    // - Non-null output pointers come from valid mutable ShyValue storage.
    // - Non-null error pointers come from valid mutable ShyError pointer storage.
    unsafe {
        shy_evaluate_parsed_with_callback_ex(expression, resolver, user_data, out_value, out_error)
    }
}

struct ErrorHandle(*mut ShyError);

impl ErrorHandle {
    fn new(error: *mut ShyError) -> Self {
        assert!(!error.is_null());
        Self(error)
    }

    fn as_ptr(&self) -> *const ShyError {
        self.0
    }
}

impl Drop for ErrorHandle {
    fn drop(&mut self) {
        // SAFETY:
        // - ErrorHandle is only built from ShyError pointers returned by this crate.
        // - Drop runs exactly once for the owned handle.
        unsafe { shy_error_free(self.0) };
    }
}

struct ParsedHandle(*mut ShyParsedExpression);

impl ParsedHandle {
    fn new(expression: *mut ShyParsedExpression) -> Self {
        assert!(!expression.is_null());
        Self(expression)
    }

    fn as_ptr(&self) -> *const ShyParsedExpression {
        self.0
    }
}

impl Drop for ParsedHandle {
    fn drop(&mut self) {
        // SAFETY:
        // - ParsedHandle is only built from ShyParsedExpression pointers returned by this crate.
        // - Drop runs exactly once for the owned handle.
        unsafe { shy_parsed_expression_free(self.0) };
    }
}

fn error_status(error: *const ShyError) -> ShyStatus {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_status(error) }
}

fn error_stage(error: *const ShyError) -> i32 {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_stage(error) }
}

fn error_code(error: *const ShyError) -> i32 {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_code(error) }
}

fn error_message(error: *const ShyError) -> Option<String> {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    let message = unsafe { shy_error_message(error) };
    if message.is_null() {
        return None;
    }

    // SAFETY:
    // - shy_error_message returned a non-null NUL-terminated pointer borrowed
    //   from a live ShyError.
    let message = unsafe { CStr::from_ptr(message) };
    Some(message.to_string_lossy().into_owned())
}

fn error_has_span(error: *const ShyError) -> i32 {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_has_span(error) }
}

fn error_span_start(error: *const ShyError) -> i32 {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_span_start(error) }
}

fn error_span_end(error: *const ShyError) -> i32 {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_span_end(error) }
}

fn error_diagnostic_count(error: *const ShyError) -> i32 {
    // SAFETY:
    // - Tests pass either null or a live ShyError pointer returned by this crate.
    unsafe { shy_error_diagnostic_count(error) }
}

unsafe extern "C" fn resolve_x_to_integer(
    name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || out_value.is_null() {
        return SHY_STATUS_NULL_POINTER;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return SHY_STATUS_RESOLVER_ERROR;
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

    SHY_STATUS_OK
}

unsafe extern "C" fn resolve_x_to_bool(
    name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || out_value.is_null() {
        return SHY_STATUS_NULL_POINTER;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return SHY_STATUS_RESOLVER_ERROR;
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

    SHY_STATUS_OK
}

unsafe extern "C" fn resolve_x_to_float(
    name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if name.is_null() || out_value.is_null() {
        return SHY_STATUS_NULL_POINTER;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return SHY_STATUS_RESOLVER_ERROR;
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

    SHY_STATUS_OK
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
        return SHY_STATUS_NULL_POINTER;
    }

    // SAFETY:
    // - name was checked for null above.
    // - the FFI adapter provides a valid NUL-terminated variable name.
    let name = unsafe { CStr::from_ptr(name) };

    if name.to_bytes() != b"x" {
        return SHY_STATUS_RESOLVER_ERROR;
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

    SHY_STATUS_OK
}

unsafe extern "C" fn failing_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    _out_value: *mut ShyValue,
) -> ShyStatus {
    SHY_STATUS_RESOLVER_ERROR
}

unsafe extern "C" fn invalid_kind_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if out_value.is_null() {
        return SHY_STATUS_NULL_POINTER;
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

    SHY_STATUS_OK
}

unsafe extern "C" fn invalid_status_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    _out_value: *mut ShyValue,
) -> ShyStatus {
    999
}

unsafe extern "C" fn infinite_float_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if out_value.is_null() {
        return SHY_STATUS_NULL_POINTER;
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

    SHY_STATUS_OK
}

unsafe extern "C" fn subnormal_float_resolver(
    _name: *const c_char,
    _user_data: *mut c_void,
    out_value: *mut ShyValue,
) -> ShyStatus {
    if out_value.is_null() {
        return SHY_STATUS_NULL_POINTER;
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

    SHY_STATUS_OK
}

#[test]
fn parse_expression_ex_returns_handle_and_clears_error() {
    let expression = c_string("1 + 2");
    let mut parsed = ptr::null_mut();
    let mut error = ptr::NonNull::<ShyError>::dangling().as_ptr();

    let status = parse_expression_ex(expression.as_ptr(), &mut parsed, &mut error);

    assert_eq!(status, SHY_STATUS_OK);
    assert!(!parsed.is_null());
    assert!(error.is_null());

    let _parsed = ParsedHandle::new(parsed);
}

#[test]
fn parse_expression_ex_rejects_null_expression() {
    let mut parsed = ptr::NonNull::<ShyParsedExpression>::dangling().as_ptr();
    let mut error = ptr::null_mut();

    let status = parse_expression_ex(ptr::null(), &mut parsed, &mut error);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
    assert!(parsed.is_null());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NULL_POINTER);
}

#[test]
fn parse_expression_ex_rejects_null_output() {
    let expression = c_string("1 + 2");
    let mut error = ptr::null_mut();

    let status = parse_expression_ex(expression.as_ptr(), ptr::null_mut(), &mut error);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NULL_POINTER);
}

#[test]
fn parse_expression_ex_rejects_invalid_utf8() {
    let bytes = [0xff_u8, 0x00_u8];
    let mut parsed = ptr::NonNull::<ShyParsedExpression>::dangling().as_ptr();
    let mut error = ptr::null_mut();

    let status = parse_expression_ex(bytes.as_ptr().cast(), &mut parsed, &mut error);

    assert_eq!(status, SHY_STATUS_INVALID_UTF8);
    assert!(parsed.is_null());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_INVALID_UTF8);
}

#[test]
fn parse_expression_ex_reports_parse_error() {
    let expression = c_string("1 +");
    let mut parsed = ptr::NonNull::<ShyParsedExpression>::dangling().as_ptr();
    let mut error = ptr::null_mut();

    let status = parse_expression_ex(expression.as_ptr(), &mut parsed, &mut error);

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert!(parsed.is_null());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_PARSE);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_PARSE_RECOVERY);
    assert!(error_diagnostic_count(error.as_ptr()) >= 1);
}

#[test]
fn parsed_no_vars_evaluates_repeatedly() {
    let expression = c_string("1 + 2");
    let mut parsed = ptr::null_mut();

    let status = parse_expression(expression.as_ptr(), &mut parsed);

    assert_eq!(status, SHY_STATUS_OK);
    let parsed = ParsedHandle::new(parsed);

    for _ in 0..3 {
        let mut out = default_test_value();
        let status = evaluate_parsed_no_vars(parsed.as_ptr(), &mut out);

        assert_eq!(status, SHY_STATUS_OK);
        assert_eq!(out.kind, SHY_VALUE_INTEGER);
        assert_eq!(out.integer_value, 3);
    }
}

#[test]
fn parsed_no_vars_ex_rejects_null_handle() {
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_parsed_no_vars_ex(ptr::null(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NULL_POINTER);
}

#[test]
fn parsed_no_vars_ex_rejects_null_output() {
    let expression = c_string("1 + 2");
    let mut parsed = ptr::null_mut();
    let mut error = ptr::null_mut();

    assert_eq!(
        parse_expression(expression.as_ptr(), &mut parsed),
        SHY_STATUS_OK
    );
    let parsed = ParsedHandle::new(parsed);

    let status = evaluate_parsed_no_vars_ex(parsed.as_ptr(), ptr::null_mut(), &mut error);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NULL_POINTER);
}

#[test]
fn parsed_callback_evaluates_with_runtime_user_data() {
    let expression = c_string("x + 2");
    let mut parsed = ptr::null_mut();

    assert_eq!(
        parse_expression(expression.as_ptr(), &mut parsed),
        SHY_STATUS_OK
    );
    let parsed = ParsedHandle::new(parsed);

    let mut first = TestContext {
        value: 40,
        calls: 0,
    };
    let mut second = TestContext {
        value: 10,
        calls: 0,
    };
    let mut out = default_test_value();

    let status = evaluate_parsed_with_callback(
        parsed.as_ptr(),
        Some(resolve_from_user_data),
        ptr::from_mut(&mut first).cast(),
        &mut out,
    );
    assert_eq!(status, SHY_STATUS_OK);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 42);

    out = default_test_value();

    let status = evaluate_parsed_with_callback(
        parsed.as_ptr(),
        Some(resolve_from_user_data),
        ptr::from_mut(&mut second).cast(),
        &mut out,
    );
    assert_eq!(status, SHY_STATUS_OK);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 12);
    assert_eq!(first.calls, 1);
    assert_eq!(second.calls, 1);
}

#[test]
fn parsed_callback_repeated_lookup_calls_resolver_each_time() {
    let expression = c_string("x + x");
    let mut parsed = ptr::null_mut();

    assert_eq!(
        parse_expression(expression.as_ptr(), &mut parsed),
        SHY_STATUS_OK
    );
    let parsed = ParsedHandle::new(parsed);

    let mut context = TestContext {
        value: 20,
        calls: 0,
    };
    let mut out = default_test_value();

    let status = evaluate_parsed_with_callback(
        parsed.as_ptr(),
        Some(resolve_from_user_data),
        ptr::from_mut(&mut context).cast(),
        &mut out,
    );

    assert_eq!(status, SHY_STATUS_OK);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 40);
    assert_eq!(context.calls, 2);
}

#[test]
fn parsed_callback_ex_reports_resolver_error() {
    let expression = c_string("x + 2");
    let mut parsed = ptr::null_mut();

    assert_eq!(
        parse_expression(expression.as_ptr(), &mut parsed),
        SHY_STATUS_OK
    );
    let parsed = ParsedHandle::new(parsed);

    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_parsed_with_callback_ex(
        parsed.as_ptr(),
        Some(failing_resolver),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_RESOLVER_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_RESOLVER);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_RESOLVER_ERROR);
}

#[test]
fn parsed_callback_ex_reports_invalid_value_kind() {
    let expression = c_string("x");
    let mut parsed = ptr::null_mut();

    assert_eq!(
        parse_expression(expression.as_ptr(), &mut parsed),
        SHY_STATUS_OK
    );
    let parsed = ParsedHandle::new(parsed);

    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_parsed_with_callback_ex(
        parsed.as_ptr(),
        Some(invalid_kind_resolver),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_INVALID_VALUE);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INVALID_VALUE);
    assert_eq!(
        error_code(error.as_ptr()),
        SHY_ERROR_CODE_INVALID_VALUE_KIND
    );
}

#[test]
fn parsed_status_only_variants_work() {
    let expression = c_string("x + 2");
    let mut parsed = ptr::null_mut();

    assert_eq!(
        parse_expression(expression.as_ptr(), &mut parsed),
        SHY_STATUS_OK
    );
    let parsed = ParsedHandle::new(parsed);

    let mut context = TestContext {
        value: 40,
        calls: 0,
    };
    let mut out = default_test_value();

    let status = evaluate_parsed_with_callback(
        parsed.as_ptr(),
        Some(resolve_from_user_data),
        ptr::from_mut(&mut context).cast(),
        &mut out,
    );

    assert_eq!(status, SHY_STATUS_OK);
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 42);
}

#[test]
fn error_accessors_handle_null() {
    assert_eq!(error_status(ptr::null()), SHY_STATUS_NULL_POINTER);
    assert_eq!(error_stage(ptr::null()), SHY_ERROR_STAGE_NONE);
    assert_eq!(error_code(ptr::null()), SHY_ERROR_CODE_NULL_POINTER);
    assert_eq!(error_message(ptr::null()), None);
    assert_eq!(error_has_span(ptr::null()), 0);
    assert_eq!(error_span_start(ptr::null()), -1);
    assert_eq!(error_span_end(ptr::null()), -1);
    assert_eq!(error_diagnostic_count(ptr::null()), 0);
}

#[test]
fn no_vars_ex_success_clears_error() {
    let expression = c_string("1 + 2");
    let mut out = default_test_value();
    let mut error = ptr::NonNull::<ShyError>::dangling().as_ptr();

    let status = evaluate_ex(expression.as_ptr(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_OK);
    assert!(error.is_null());
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 3);
}

#[test]
fn no_vars_ex_null_expression_reports_input_error() {
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_ex(ptr::null(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_status(error.as_ptr()), SHY_STATUS_NULL_POINTER);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NULL_POINTER);
    assert!(!error_message(error.as_ptr()).unwrap_or_default().is_empty());
}

#[test]
fn no_vars_ex_invalid_utf8_reports_input_error() {
    let bytes = [0xff_u8, 0x00_u8];
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_ex(bytes.as_ptr().cast(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_INVALID_UTF8);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_status(error.as_ptr()), SHY_STATUS_INVALID_UTF8);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_INVALID_UTF8);
}

#[test]
fn no_vars_ex_lexical_error_has_span() {
    let expression = c_string("$");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_ex(expression.as_ptr(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_LEXICAL);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_LEXICAL_ERROR);
    assert_eq!(error_has_span(error.as_ptr()), 1);
    assert_eq!(error_span_start(error.as_ptr()), 0);
    assert_eq!(error_span_end(error.as_ptr()), 1);
    assert_eq!(error_diagnostic_count(error.as_ptr()), 1);
}

#[test]
fn no_vars_ex_parse_error_reports_diagnostic_count() {
    let expression = c_string("1 +");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_ex(expression.as_ptr(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_PARSE);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_PARSE_RECOVERY);
    assert!(error_diagnostic_count(error.as_ptr()) >= 1);
}

#[test]
fn no_vars_ex_resource_limit_reports_input_too_large() {
    let expression = c_string(&"1".repeat(16 * 1024 + 1));
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_ex(expression.as_ptr(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_RESOURCE_LIMIT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_INPUT_TOO_LARGE);
}

#[test]
fn no_vars_ex_division_by_zero_reports_evaluation_error_code() {
    let expression = c_string("1 / 0");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_ex(expression.as_ptr(), &mut out, &mut error);

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_status(error.as_ptr()), SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_EVALUATION);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_DIVISION_BY_ZERO);
    assert!(!error_message(error.as_ptr()).unwrap_or_default().is_empty());
}

#[test]
fn no_vars_ex_failure_allows_null_error_output() {
    let expression = c_string("1 / 0");
    let mut out = default_test_value();

    let status = evaluate_ex(expression.as_ptr(), &mut out, ptr::null_mut());

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());
}

#[test]
fn callback_ex_null_resolver_reports_input_error() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_with_callback_ex(
        expression.as_ptr(),
        None,
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INPUT);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NULL_POINTER);
}

#[test]
fn callback_ex_success_clears_error() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();
    let mut error = ptr::NonNull::<ShyError>::dangling().as_ptr();

    let status = evaluate_with_callback_ex(
        expression.as_ptr(),
        Some(resolve_x_to_integer),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_OK);
    assert!(error.is_null());
    assert_eq!(out.kind, SHY_VALUE_INTEGER);
    assert_eq!(out.integer_value, 42);
}

#[test]
fn callback_ex_resolver_error_reports_resolver_stage() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_with_callback_ex(
        expression.as_ptr(),
        Some(failing_resolver),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_RESOLVER_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_status(error.as_ptr()), SHY_STATUS_RESOLVER_ERROR);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_RESOLVER);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_RESOLVER_ERROR);
}

#[test]
fn callback_ex_invalid_value_kind_reports_invalid_value_stage() {
    let expression = c_string("x");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_with_callback_ex(
        expression.as_ptr(),
        Some(invalid_kind_resolver),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_INVALID_VALUE);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_status(error.as_ptr()), SHY_STATUS_INVALID_VALUE);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_INVALID_VALUE);
    assert_eq!(
        error_code(error.as_ptr()),
        SHY_ERROR_CODE_INVALID_VALUE_KIND
    );
}

#[test]
fn callback_ex_non_finite_float_reports_evaluation_error() {
    let expression = c_string("x");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_with_callback_ex(
        expression.as_ptr(),
        Some(infinite_float_resolver),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_EVALUATION);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_NON_FINITE_FLOAT);
}

#[test]
fn callback_ex_subnormal_float_reports_evaluation_error() {
    let expression = c_string("x");
    let mut out = default_test_value();
    let mut error = ptr::null_mut();

    let status = evaluate_with_callback_ex(
        expression.as_ptr(),
        Some(subnormal_float_resolver),
        ptr::null_mut(),
        &mut out,
        &mut error,
    );

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());

    let error = ErrorHandle::new(error);
    assert_eq!(error_stage(error.as_ptr()), SHY_ERROR_STAGE_EVALUATION);
    assert_eq!(error_code(error.as_ptr()), SHY_ERROR_CODE_SUBNORMAL_FLOAT);
}

#[test]
fn evaluate_no_vars_rejects_null_expression() {
    let mut out = default_test_value();

    let status = evaluate(ptr::null(), &mut out);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_no_vars_rejects_null_output() {
    let expression = c_string("1 + 2");

    let status = evaluate(expression.as_ptr(), ptr::null_mut());

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
}

#[test]
fn evaluate_no_vars_rejects_invalid_utf8() {
    let bytes = [0xff_u8, 0x00_u8];
    let mut out = default_test_value();

    let status = evaluate(bytes.as_ptr().cast(), &mut out);

    assert_eq!(status, SHY_STATUS_INVALID_UTF8);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_no_vars_returns_integer_value() {
    let expression = c_string("1 + 2");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_no_vars_reports_parse_or_lex_error_as_evaluation_error() {
    let expression = c_string("$");
    let mut out = default_test_value();

    let status = evaluate(expression.as_ptr(), &mut out);

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
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

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_rejects_null_callback() {
    let expression = c_string("x + 2");
    let mut out = default_test_value();

    let status = evaluate_with_callback(expression.as_ptr(), None, ptr::null_mut(), &mut out);

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
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

    assert_eq!(status, SHY_STATUS_NULL_POINTER);
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

    assert_eq!(status, SHY_STATUS_INVALID_UTF8);
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

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_OK);
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

    assert_eq!(status, SHY_STATUS_RESOLVER_ERROR);
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

    assert_eq!(status, SHY_STATUS_INVALID_VALUE);
    assert_eq!(out, default_test_value());
}

#[test]
fn evaluate_with_callback_maps_unknown_callback_status_to_resolver_error() {
    let expression = c_string("x");
    let mut out = default_test_value();

    let status = evaluate_with_callback(
        expression.as_ptr(),
        Some(invalid_status_resolver),
        ptr::null_mut(),
        &mut out,
    );

    assert_eq!(status, SHY_STATUS_RESOLVER_ERROR);
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

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
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

    assert_eq!(status, SHY_STATUS_EVALUATION_ERROR);
    assert_eq!(out, default_test_value());
}

#[test]
fn ffi_type_sizes_are_as_expected() {
    assert_eq!(std::mem::size_of::<ShyStatus>(), 4);
    assert_eq!(std::mem::size_of::<ShyValueKind>(), 4);
}
