#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "shunting_yard_ffi.h"

_Static_assert(sizeof(ShyStatus) == 4, "ShyStatus must be 4 bytes");

_Static_assert(offsetof(ShyValue, kind) == 0, "kind offset mismatch");
_Static_assert(sizeof(((ShyValue *)0)->kind) == 4, "ShyValue.kind must be int32_t");
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

typedef struct TestContext {
    int64_t value;
    int calls;
} TestContext;

static ShyStatus resolve_from_context(
    const char *name,
    void *user_data,
    ShyValue *out_value
) {
    TestContext *context = (TestContext *)user_data;

    if (name == NULL || context == NULL || out_value == NULL) {
        return SHY_STATUS_NULL_POINTER;
    }

    if (strcmp(name, "x") != 0) {
        return SHY_STATUS_RESOLVER_ERROR;
    }

    context->calls += 1;
    out_value->kind = SHY_VALUE_INTEGER;
    out_value->bool_value = 0;
    out_value->integer_value = context->value;
    out_value->float_value = 0.0;

    return SHY_STATUS_OK;
}

static ShyStatus failing_resolver(
    const char *name,
    void *user_data,
    ShyValue *out_value
) {
    (void)name;
    (void)user_data;
    (void)out_value;

    return SHY_STATUS_RESOLVER_ERROR;
}

static ShyStatus invalid_kind_resolver(
    const char *name,
    void *user_data,
    ShyValue *out_value
) {
    (void)name;
    (void)user_data;

    if (out_value == NULL) {
        return SHY_STATUS_NULL_POINTER;
    }

    out_value->kind = 999;
    out_value->bool_value = 0;
    out_value->integer_value = 0;
    out_value->float_value = 0.0;

    return SHY_STATUS_OK;
}

static ShyStatus invalid_status_resolver(
    const char *name,
    void *user_data,
    ShyValue *out_value
) {
    (void)name;
    (void)user_data;
    (void)out_value;

    return 999;
}

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

static void test_callback_integer_success(void) {
    TestContext context = {.value = 40, .calls = 0};
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_with_callback(
        "x + 2",
        resolve_from_context,
        &context,
        &value
    );

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 42);
    assert(context.calls == 1);
}

static void test_callback_repeated_lookup(void) {
    TestContext context = {.value = 20, .calls = 0};
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_with_callback(
        "x + x",
        resolve_from_context,
        &context,
        &value
    );

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 40);
    assert(context.calls == 2);
}

static void test_callback_null_resolver(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyStatus status = shy_evaluate_with_callback("x + 2", NULL, NULL, &value);

    assert(status == SHY_STATUS_NULL_POINTER);
    assert(value.integer_value == -1);
}

static void test_callback_resolver_error(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyStatus status = shy_evaluate_with_callback(
        "x + 2",
        failing_resolver,
        NULL,
        &value
    );

    assert(status == SHY_STATUS_RESOLVER_ERROR);
    assert(value.integer_value == -1);
}

static void test_callback_invalid_value_kind(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyStatus status = shy_evaluate_with_callback(
        "x",
        invalid_kind_resolver,
        NULL,
        &value
    );

    assert(status == SHY_STATUS_INVALID_VALUE);
    assert(value.integer_value == -1);
}

static void test_callback_invalid_status(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyStatus status = shy_evaluate_with_callback(
        "x",
        invalid_status_resolver,
        NULL,
        &value
    );

    assert(status == SHY_STATUS_RESOLVER_ERROR);
    assert(value.integer_value == -1);
}

static void test_callback_null_expression(void) {
    TestContext context = {.value = 40, .calls = 0};
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyStatus status = shy_evaluate_with_callback(
        NULL,
        resolve_from_context,
        &context,
        &value
    );

    assert(status == SHY_STATUS_NULL_POINTER);
    assert(context.calls == 0);
    assert(value.integer_value == -1);
}

int main(void) {
    test_integer_success();
    test_null_expression();
    test_null_output();
    test_evaluation_error();
    test_invalid_utf8();
    test_callback_integer_success();
    test_callback_repeated_lookup();
    test_callback_null_resolver();
    test_callback_resolver_error();
    test_callback_invalid_value_kind();
    test_callback_invalid_status();
    test_callback_null_expression();

    puts("shunting_yard_ffi smoke test passed");
    return 0;
}
