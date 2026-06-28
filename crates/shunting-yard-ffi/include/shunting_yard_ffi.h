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

typedef struct ShyValue {
    int32_t kind;
    uint8_t bool_value;
    int64_t integer_value;
    double float_value;
} ShyValue;

typedef struct ShyError ShyError;

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

ShyStatus shy_evaluate_no_vars(
    const char *expression,
    ShyValue *out_value
);

ShyStatus shy_evaluate_with_callback(
    const char *expression,
    ShyVariableResolver resolver,
    void *user_data,
    ShyValue *out_value
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

#ifdef __cplusplus
}
#endif

#endif
