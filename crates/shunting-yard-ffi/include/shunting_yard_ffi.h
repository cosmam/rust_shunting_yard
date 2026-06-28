#ifndef SHUNTING_YARD_FFI_H
#define SHUNTING_YARD_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum ShyStatus {
    SHY_STATUS_OK = 0,
    SHY_STATUS_NULL_POINTER = 1,
    SHY_STATUS_INVALID_UTF8 = 2,
    SHY_STATUS_EVALUATION_ERROR = 3,
    SHY_STATUS_PANIC = 4,
    SHY_STATUS_INVALID_VALUE = 6,
} ShyStatus;

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

ShyStatus shy_evaluate_no_vars(
    const char *expression,
    ShyValue *out_value
);

#ifdef __cplusplus
}
#endif

#endif
