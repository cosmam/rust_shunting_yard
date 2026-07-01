#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "shunting_yard_ffi.h"

typedef struct ExampleContext {
    int64_t value;
    int calls;
} ExampleContext;

static int check_status(const char *label, ShyStatus actual, ShyStatus expected) {
    if (actual == expected) {
        return 0;
    }

    fprintf(
        stderr,
        "%s: expected status %d, got %d\n",
        label,
        (int)expected,
        (int)actual
    );
    return 1;
}

static ShyStatus resolve_x(
    const char *name,
    void *user_data,
    ShyValue *out_value
) {
    ExampleContext *context = (ExampleContext *)user_data;

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

static int evaluate_without_variables(void) {
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_no_vars("1 + 2", &value);

    if (check_status("no-variable evaluation", status, SHY_STATUS_OK) != 0) {
        return 1;
    }

    if (value.kind != SHY_VALUE_INTEGER || value.integer_value != 3) {
        fprintf(stderr, "no-variable evaluation produced the wrong value\n");
        return 1;
    }

    return 0;
}

static int evaluate_with_callback(void) {
    ExampleContext context = {.value = 40, .calls = 0};
    ShyValue value = {0};
    ShyStatus status = shy_evaluate_with_callback(
        "x + 2",
        resolve_x,
        &context,
        &value
    );

    if (check_status("callback evaluation", status, SHY_STATUS_OK) != 0) {
        return 1;
    }

    if (
        value.kind != SHY_VALUE_INTEGER ||
        value.integer_value != 42 ||
        context.calls != 1
    ) {
        fprintf(stderr, "callback evaluation produced the wrong value\n");
        return 1;
    }

    return 0;
}

static int enforce_resource_limits(void) {
    ShyEvalOptions options = {0};
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_eval_options_default(&options);

    if (check_status("default eval options", status, SHY_STATUS_OK) != 0) {
        return 1;
    }

    options.max_tokens = 1;
    status = shy_evaluate_no_vars_with_options_ex(
        "1 + 2",
        &options,
        &value,
        &error
    );

    if (
        check_status(
            "resource-limited evaluation",
            status,
            SHY_STATUS_EVALUATION_ERROR
        ) != 0
    ) {
        shy_error_free(error);
        return 1;
    }

    if (
        error == NULL ||
        shy_error_stage(error) != SHY_ERROR_STAGE_RESOURCE_LIMIT ||
        shy_error_code(error) != SHY_ERROR_CODE_TOO_MANY_TOKENS ||
        value.integer_value != -1
    ) {
        fprintf(stderr, "resource-limit error reporting failed\n");
        shy_error_free(error);
        return 1;
    }

    shy_error_free(error);
    return 0;
}

static int report_evaluation_error(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_evaluate_no_vars_ex("1 / 0", &value, &error);

    if (
        check_status(
            "division-by-zero evaluation",
            status,
            SHY_STATUS_EVALUATION_ERROR
        ) != 0
    ) {
        shy_error_free(error);
        return 1;
    }

    if (
        error == NULL ||
        shy_error_stage(error) != SHY_ERROR_STAGE_EVALUATION ||
        shy_error_code(error) != SHY_ERROR_CODE_DIVISION_BY_ZERO ||
        shy_error_message(error) == NULL ||
        value.integer_value != -1
    ) {
        fprintf(stderr, "evaluation error reporting failed\n");
        shy_error_free(error);
        return 1;
    }

    shy_error_free(error);
    return 0;
}

static int reuse_parsed_expression(void) {
    ShyParsedExpression *parsed = NULL;
    ShyStatus status = shy_parse_expression("x + 2", &parsed);

    if (check_status("parse expression", status, SHY_STATUS_OK) != 0) {
        return 1;
    }

    if (parsed == NULL) {
        fprintf(stderr, "parse expression returned a null handle\n");
        return 1;
    }

    ExampleContext first = {.value = 40, .calls = 0};
    ExampleContext second = {.value = 10, .calls = 0};
    ShyValue value = {0};

    status = shy_evaluate_parsed_with_callback(parsed, resolve_x, &first, &value);
    if (check_status("first parsed evaluation", status, SHY_STATUS_OK) != 0) {
        shy_parsed_expression_free(parsed);
        return 1;
    }
    if (value.kind != SHY_VALUE_INTEGER || value.integer_value != 42) {
        fprintf(stderr, "first parsed evaluation produced the wrong value\n");
        shy_parsed_expression_free(parsed);
        return 1;
    }

    value = (ShyValue){0};
    status = shy_evaluate_parsed_with_callback(parsed, resolve_x, &second, &value);
    if (check_status("second parsed evaluation", status, SHY_STATUS_OK) != 0) {
        shy_parsed_expression_free(parsed);
        return 1;
    }
    if (
        value.kind != SHY_VALUE_INTEGER ||
        value.integer_value != 12 ||
        first.calls != 1 ||
        second.calls != 1
    ) {
        fprintf(stderr, "parsed-expression reuse produced the wrong value\n");
        shy_parsed_expression_free(parsed);
        return 1;
    }

    shy_parsed_expression_free(parsed);
    return 0;
}

int main(void) {
    if (evaluate_without_variables() != 0) {
        return 1;
    }
    if (evaluate_with_callback() != 0) {
        return 1;
    }
    if (enforce_resource_limits() != 0) {
        return 1;
    }
    if (report_evaluation_error() != 0) {
        return 1;
    }
    if (reuse_parsed_expression() != 0) {
        return 1;
    }

    puts("shunting_yard_ffi C consumer example passed");
    return 0;
}
