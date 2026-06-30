#ifndef SHUNTING_YARD_FFI_H
#define SHUNTING_YARD_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int32_t ShyStatus;

enum {
    SHY_STATUS_OK = 0,
    SHY_STATUS_NULL_POINTER = 1,
    SHY_STATUS_INVALID_UTF8 = 2,
    SHY_STATUS_EVALUATION_ERROR = 3,
    SHY_STATUS_PANIC = 4,
    SHY_STATUS_RESOLVER_ERROR = 5,
    SHY_STATUS_INVALID_VALUE = 6,
};

enum {
    SHY_VALUE_BOOL = 0,
    SHY_VALUE_INTEGER = 1,
    SHY_VALUE_FLOAT = 2,
};

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

enum {
    SHY_DIAGNOSTIC_KIND_NONE = 0,
    SHY_DIAGNOSTIC_KIND_INVALID_TOKEN = 1,
    SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_EOF = 2,
    SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_TOKEN = 3,
    SHY_DIAGNOSTIC_KIND_EXTRA_TOKEN = 4,
    SHY_DIAGNOSTIC_KIND_USER = 5,
    SHY_DIAGNOSTIC_KIND_RECOVERY = 6,
};

typedef struct ShyValue {
    int32_t kind;
    uint8_t bool_value;
    int64_t integer_value;
    double float_value;
} ShyValue;

typedef struct ShyError ShyError;
typedef struct ShyParsedExpression ShyParsedExpression;

/*
 * Callback used to resolve variable names.
 *
 * name:
 *   Valid NUL-terminated UTF-8 variable name. Valid only for the duration
 *   of the callback call. The callback must not retain this pointer.
 *
 * user_data:
 *   Caller-owned pointer passed through from shy_evaluate_with_callback.
 *   May be NULL. Rust does not dereference or retain it.
 *
 * out_value:
 *   Writable storage for one ShyValue. The callback must write this value
 *   before returning SHY_STATUS_OK.
 *
 * Return:
 *   SHY_STATUS_OK only after writing out_value. Return a non-OK status if
 *   the variable cannot be resolved.
 *
 * The callback must not unwind across the C ABI boundary.
 */
typedef ShyStatus (*ShyVariableResolver)(
    const char *name,
    void *user_data,
    ShyValue *out_value
);

/*
 * Parse an expression into an opaque handle.
 *
 * On success, out_expression receives a non-NULL handle that must be released
 * with shy_parsed_expression_free. On failure, *out_expression is set to NULL
 * when out_expression is non-NULL.
 */
ShyStatus shy_parse_expression(
    const char *expression,
    ShyParsedExpression **out_expression
);

/*
 * Extended parse function with optional error object reporting.
 *
 * If out_error is not NULL, *out_error is set to NULL on success. On failure,
 * *out_error receives an owned ShyError that must be released with
 * shy_error_free. Passing out_error as NULL is allowed and disables error
 * object allocation.
 */
ShyStatus shy_parse_expression_ex(
    const char *expression,
    ShyParsedExpression **out_expression,
    ShyError **out_error
);

/*
 * Free a parsed-expression handle returned through shy_parse_expression*.
 * Passing NULL is allowed. Handles are immutable and must not be used after
 * they are freed.
 */
void shy_parsed_expression_free(ShyParsedExpression *expression);

ShyStatus shy_evaluate_no_vars(
    const char *expression,
    ShyValue *out_value
);

/*
 * Extended no-variable evaluation.
 *
 * If out_error is not NULL, *out_error is set to NULL on success. On failure,
 * *out_error receives an owned ShyError that must be released with
 * shy_error_free. Passing out_error as NULL is allowed and disables error
 * object allocation.
 */
ShyStatus shy_evaluate_no_vars_ex(
    const char *expression,
    ShyValue *out_value,
    ShyError **out_error
);

ShyStatus shy_evaluate_with_callback(
    const char *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value
);

/*
 * Extended callback-backed evaluation.
 *
 * If out_error is not NULL, *out_error is set to NULL on success. On failure,
 * *out_error receives an owned ShyError that must be released with
 * shy_error_free. Passing out_error as NULL is allowed and disables error
 * object allocation.
 */
ShyStatus shy_evaluate_with_callback_ex(
    const char *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value,
    ShyError **out_error
);

ShyStatus shy_evaluate_parsed_no_vars(
    const ShyParsedExpression *expression,
    ShyValue *out_value
);

/*
 * Extended parsed no-variable evaluation.
 *
 * If out_error is not NULL, *out_error is set to NULL on success. On failure,
 * *out_error receives an owned ShyError that must be released with
 * shy_error_free. Passing out_error as NULL is allowed and disables error
 * object allocation.
 */
ShyStatus shy_evaluate_parsed_no_vars_ex(
    const ShyParsedExpression *expression,
    ShyValue *out_value,
    ShyError **out_error
);

ShyStatus shy_evaluate_parsed_with_callback(
    const ShyParsedExpression *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value
);

/*
 * Extended parsed callback-backed evaluation.
 *
 * If out_error is not NULL, *out_error is set to NULL on success. On failure,
 * *out_error receives an owned ShyError that must be released with
 * shy_error_free. Passing out_error as NULL is allowed and disables error
 * object allocation.
 */
ShyStatus shy_evaluate_parsed_with_callback_ex(
    const ShyParsedExpression *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value,
    ShyError **out_error
);

/*
 * Free an error object returned through an extended entrypoint's out_error.
 * Passing NULL is allowed. Do not free ShyError with free() or any other
 * allocator.
 */
void shy_error_free(ShyError *error);

/*
 * Error accessors. shy_error_message returns a borrowed pointer that remains
 * valid only until shy_error_free(error). Passing NULL to an accessor returns
 * the documented null/default value for that accessor.
 */
ShyStatus shy_error_status(const ShyError *error);
int32_t shy_error_stage(const ShyError *error);
int32_t shy_error_code(const ShyError *error);
const char *shy_error_message(const ShyError *error);
int32_t shy_error_has_span(const ShyError *error);
int32_t shy_error_span_start(const ShyError *error);
int32_t shy_error_span_end(const ShyError *error);
int32_t shy_error_diagnostic_count(const ShyError *error);

/*
 * Indexed parse diagnostic accessors.
 *
 * These accessors are meaningful for parse-stage ShyError objects. For other
 * errors, diagnostic count is zero and indexed accessors return default values.
 *
 * Passing NULL or an out-of-range diagnostic index returns the documented
 * default value for the accessor. Invalid expected-token indexes return NULL.
 *
 * Returned strings are borrowed from ShyError and remain valid only until
 * shy_error_free(error). Do not free returned strings.
 *
 * Expected-token strings are intended for diagnostics/display and should not be
 * treated as a stable machine-readable grammar schema.
 */
int32_t shy_error_diagnostic_kind(const ShyError *error, int32_t index);
int32_t shy_error_diagnostic_has_span(const ShyError *error, int32_t index);
int32_t shy_error_diagnostic_span_start(const ShyError *error, int32_t index);
int32_t shy_error_diagnostic_span_end(const ShyError *error, int32_t index);
int32_t shy_error_diagnostic_expected_count(
    const ShyError *error,
    int32_t index
);
const char *shy_error_diagnostic_expected_token(
    const ShyError *error,
    int32_t diagnostic_index,
    int32_t expected_index
);

#ifdef __cplusplus
}
#endif

#endif
