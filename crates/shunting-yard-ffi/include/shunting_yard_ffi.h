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

typedef struct ShyValue {
    int32_t kind;
    uint8_t bool_value;
    int64_t integer_value;
    double float_value;
} ShyValue;

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

#ifdef __cplusplus
}
#endif

#endif
