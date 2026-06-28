#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "shunting_yard_ffi.h"

_Static_assert(sizeof(ShyStatus) == 4, "ShyStatus must be 4 bytes");
_Static_assert(sizeof(ShyValueKind) == 4, "ShyValueKind must be 4 bytes");

_Static_assert(offsetof(ShyValue, kind) == 0, "kind offset mismatch");
_Static_assert(
    offsetof(ShyValue, bool_value) > offsetof(ShyValue, kind),
    "bool_value offset mismatch"
);
_Static_assert(
    offsetof(ShyValue, integer_value) > offsetof(ShyValue, bool_value),
    "integer_value offset mismatch"
);
_Static_assert(
    offsetof(ShyValue, float_value) > offsetof(ShyValue, integer_value),
    "float_value offset mismatch"
);

static void test_integer_success(void) {
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_no_vars("1 + 2", &value);

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 3);
}

static void test_null_expression(void) {
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_no_vars(NULL, &value);

    assert(status == SHY_STATUS_NULL_POINTER);
}

static void test_null_output(void) {
    ShyStatus status = shy_evaluate_no_vars("1 + 2", NULL);

    assert(status == SHY_STATUS_NULL_POINTER);
}

static void test_evaluation_error(void) {
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_no_vars("1 / 0", &value);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
}

static void test_invalid_utf8(void) {
    const char invalid_utf8[] = {(char)0xff, '\0'};
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_no_vars(invalid_utf8, &value);

    assert(status == SHY_STATUS_INVALID_UTF8);
}

int main(void) {
    test_integer_success();
    test_null_expression();
    test_null_output();
    test_evaluation_error();
    test_invalid_utf8();

    puts("shunting_yard_ffi smoke test passed");
    return 0;
}
