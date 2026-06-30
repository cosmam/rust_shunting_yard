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

_Static_assert(
    offsetof(ShyEvalOptions, abi_size) == 0,
    "ShyEvalOptions.abi_size offset mismatch"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_input_bytes) >
        offsetof(ShyEvalOptions, abi_size),
    "ShyEvalOptions.max_input_bytes offset mismatch"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_tokens) >
        offsetof(ShyEvalOptions, max_input_bytes),
    "ShyEvalOptions.max_tokens offset mismatch"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_ast_nodes) >
        offsetof(ShyEvalOptions, max_tokens),
    "ShyEvalOptions.max_ast_nodes offset mismatch"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_depth) >
        offsetof(ShyEvalOptions, max_ast_nodes),
    "ShyEvalOptions.max_depth offset mismatch"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_function_args) >
        offsetof(ShyEvalOptions, max_depth),
    "ShyEvalOptions.max_function_args offset mismatch"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_parser_recoveries) >
        offsetof(ShyEvalOptions, max_function_args),
    "ShyEvalOptions.max_parser_recoveries offset mismatch"
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

static void test_eval_options_default(void) {
    ShyEvalOptions options = {0};
    ShyStatus status = shy_eval_options_default(&options);

    assert(status == SHY_STATUS_OK);
    assert(options.abi_size == sizeof(ShyEvalOptions));
    assert(options.max_input_bytes == 16 * 1024);
    assert(options.max_tokens == 4096);
    assert(options.max_ast_nodes == 4096);
    assert(options.max_depth == 256);
    assert(options.max_function_args == 256);
    assert(options.max_parser_recoveries == 64);
}

static void test_no_vars_with_options_success(void) {
    ShyEvalOptions options = {0};
    ShyValue value = {0};
    ShyStatus status = shy_eval_options_default(&options);

    assert(status == SHY_STATUS_OK);

    status = shy_evaluate_no_vars_with_options("1 + 2", &options, &value);

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 3);
}

static void test_no_vars_with_options_token_limit(void) {
    ShyEvalOptions options = {0};
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_eval_options_default(&options);

    assert(status == SHY_STATUS_OK);

    options.max_tokens = 1;
    status = shy_evaluate_no_vars_with_options_ex(
        "1 + 2",
        &options,
        &value,
        &error
    );

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_RESOURCE_LIMIT);
    assert(shy_error_code(error) == SHY_ERROR_CODE_TOO_MANY_TOKENS);

    shy_error_free(error);
}

static void test_with_options_rejects_invalid_options(void) {
    ShyEvalOptions options = {0};
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_eval_options_default(&options);

    assert(status == SHY_STATUS_OK);

    options.abi_size = 0;
    status = shy_evaluate_no_vars_with_options_ex(
        "1 + 2",
        &options,
        &value,
        &error
    );

    assert(status == SHY_STATUS_INVALID_OPTIONS);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_INPUT);
    assert(shy_error_code(error) == SHY_ERROR_CODE_INVALID_OPTIONS);

    shy_error_free(error);
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

static void test_callback_with_options_success(void) {
    ShyEvalOptions options = {0};
    TestContext context = {.value = 40, .calls = 0};
    ShyValue value = {0};
    ShyStatus status = shy_eval_options_default(&options);

    assert(status == SHY_STATUS_OK);

    status = shy_evaluate_with_callback_with_options(
        "x + 2",
        &options,
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

static void test_no_vars_ex_success_clears_error(void) {
    ShyValue value = {0};
    ShyError *error = (ShyError *)1;
    ShyStatus status = shy_evaluate_no_vars_ex("1 + 2", &value, &error);

    assert(status == SHY_STATUS_OK);
    assert(error == NULL);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 3);
}

static void test_callback_ex_success_clears_error(void) {
    TestContext context = {.value = 40, .calls = 0};
    ShyValue value = {0};
    ShyError *error = (ShyError *)1;
    ShyStatus status = shy_evaluate_with_callback_ex(
        "x + 2",
        resolve_from_context,
        &context,
        &value,
        &error
    );

    assert(status == SHY_STATUS_OK);
    assert(error == NULL);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 42);
    assert(context.calls == 1);
}

static void test_no_vars_ex_reports_division_by_zero(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_evaluate_no_vars_ex("1 / 0", &value, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_status(error) == SHY_STATUS_EVALUATION_ERROR);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_EVALUATION);
    assert(shy_error_code(error) == SHY_ERROR_CODE_DIVISION_BY_ZERO);
    assert(shy_error_message(error) != NULL);

    shy_error_free(error);
}

static void test_no_vars_ex_reports_lexical_span(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_evaluate_no_vars_ex("$", &value, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_LEXICAL);
    assert(shy_error_code(error) == SHY_ERROR_CODE_LEXICAL_ERROR);
    assert(shy_error_has_span(error) == 1);
    assert(shy_error_span_start(error) == 0);
    assert(shy_error_span_end(error) == 1);
    assert(shy_error_diagnostic_count(error) == 0);

    shy_error_free(error);
}

static void test_no_vars_ex_error_output_may_be_null(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyStatus status = shy_evaluate_no_vars_ex("1 / 0", &value, NULL);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(value.integer_value == -1);
}

static void test_callback_ex_reports_resolver_error(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_evaluate_with_callback_ex(
        "x",
        failing_resolver,
        NULL,
        &value,
        &error
    );

    assert(status == SHY_STATUS_RESOLVER_ERROR);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_status(error) == SHY_STATUS_RESOLVER_ERROR);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_RESOLVER);
    assert(shy_error_code(error) == SHY_ERROR_CODE_RESOLVER_ERROR);

    shy_error_free(error);
}

static void test_callback_ex_reports_invalid_value_kind(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_evaluate_with_callback_ex(
        "x",
        invalid_kind_resolver,
        NULL,
        &value,
        &error
    );

    assert(status == SHY_STATUS_INVALID_VALUE);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_status(error) == SHY_STATUS_INVALID_VALUE);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_INVALID_VALUE);
    assert(shy_error_code(error) == SHY_ERROR_CODE_INVALID_VALUE_KIND);

    shy_error_free(error);
}

static void test_parse_and_evaluate_no_vars(void) {
    ShyParsedExpression *parsed = NULL;
    ShyValue value = {0};
    ShyStatus status = shy_parse_expression("1 + 2", &parsed);

    assert(status == SHY_STATUS_OK);
    assert(parsed != NULL);

    status = shy_evaluate_parsed_no_vars(parsed, &value);

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 3);

    shy_parsed_expression_free(parsed);
}

static void test_parse_with_options_success(void) {
    ShyEvalOptions options = {0};
    ShyParsedExpression *parsed = NULL;
    ShyStatus status = shy_eval_options_default(&options);

    assert(status == SHY_STATUS_OK);

    status = shy_parse_expression_with_options("1 + 2", &options, &parsed);

    assert(status == SHY_STATUS_OK);
    assert(parsed != NULL);

    shy_parsed_expression_free(parsed);
}

static void test_parse_once_evaluate_many(void) {
    ShyParsedExpression *parsed = NULL;
    ShyStatus status = shy_parse_expression("1 + 2", &parsed);

    assert(status == SHY_STATUS_OK);
    assert(parsed != NULL);

    for (int i = 0; i < 3; i++) {
        ShyValue value = {0};
        status = shy_evaluate_parsed_no_vars(parsed, &value);

        assert(status == SHY_STATUS_OK);
        assert(value.kind == SHY_VALUE_INTEGER);
        assert(value.integer_value == 3);
    }

    shy_parsed_expression_free(parsed);
}

static void test_parsed_callback_uses_runtime_user_data(void) {
    ShyParsedExpression *parsed = NULL;
    ShyValue value = {0};
    TestContext context_a = {.value = 40, .calls = 0};
    TestContext context_b = {.value = 10, .calls = 0};
    ShyStatus status = shy_parse_expression("x + 2", &parsed);

    assert(status == SHY_STATUS_OK);
    assert(parsed != NULL);

    status = shy_evaluate_parsed_with_callback(
        parsed,
        resolve_from_context,
        &context_a,
        &value
    );

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 42);

    value = (ShyValue){0};
    status = shy_evaluate_parsed_with_callback(
        parsed,
        resolve_from_context,
        &context_b,
        &value
    );

    assert(status == SHY_STATUS_OK);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 12);
    assert(context_a.calls == 1);
    assert(context_b.calls == 1);

    shy_parsed_expression_free(parsed);
}

static void test_parsed_ex_evaluation_success_clears_error(void) {
    ShyParsedExpression *parsed = NULL;
    ShyValue value = {0};
    ShyError *error = (ShyError *)1;
    TestContext context = {.value = 40, .calls = 0};
    ShyStatus status = shy_parse_expression("x + 2", &parsed);

    assert(status == SHY_STATUS_OK);
    assert(parsed != NULL);

    status = shy_evaluate_parsed_with_callback_ex(
        parsed,
        resolve_from_context,
        &context,
        &value,
        &error
    );

    assert(status == SHY_STATUS_OK);
    assert(error == NULL);
    assert(value.kind == SHY_VALUE_INTEGER);
    assert(value.integer_value == 42);
    assert(context.calls == 1);

    shy_parsed_expression_free(parsed);
}

static void test_parse_expression_ex_reports_parse_error(void) {
    ShyParsedExpression *parsed = NULL;
    ShyError *error = NULL;
    ShyStatus status = shy_parse_expression_ex("1 +", &parsed, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(parsed == NULL);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_PARSE);
    assert(shy_error_diagnostic_count(error) >= 1);

    shy_error_free(error);
}

static void test_diagnostic_accessors_handle_null(void) {
    assert(shy_error_diagnostic_kind(NULL, 0) == SHY_DIAGNOSTIC_KIND_NONE);
    assert(shy_error_diagnostic_has_span(NULL, 0) == 0);
    assert(shy_error_diagnostic_span_start(NULL, 0) == -1);
    assert(shy_error_diagnostic_span_end(NULL, 0) == -1);
    assert(shy_error_diagnostic_expected_count(NULL, 0) == 0);
    assert(shy_error_diagnostic_expected_token(NULL, 0, 0) == NULL);
}

static void test_parse_diagnostic_iteration(void) {
    ShyParsedExpression *parsed = NULL;
    ShyError *error = NULL;
    ShyStatus status = shy_parse_expression_ex("1 +", &parsed, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(parsed == NULL);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_PARSE);
    assert(shy_error_diagnostic_count(error) >= 1);

    int32_t kind = shy_error_diagnostic_kind(error, 0);
    assert(
        kind == SHY_DIAGNOSTIC_KIND_RECOVERY ||
        kind == SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_EOF ||
        kind == SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_TOKEN
    );

    assert(shy_error_diagnostic_has_span(error, 0) == 1);
    assert(shy_error_diagnostic_span_start(error, 0) >= 0);
    assert(
        shy_error_diagnostic_span_end(error, 0) >=
        shy_error_diagnostic_span_start(error, 0)
    );

    int32_t expected_count = shy_error_diagnostic_expected_count(error, 0);
    assert(expected_count >= 1);

    const char *token = shy_error_diagnostic_expected_token(error, 0, 0);
    assert(token != NULL);
    assert(token[0] != '\0');

    assert(
        shy_error_diagnostic_kind(error, shy_error_diagnostic_count(error)) ==
        SHY_DIAGNOSTIC_KIND_NONE
    );
    assert(
        shy_error_diagnostic_expected_token(error, 0, expected_count) == NULL
    );

    shy_error_free(error);
}

static void test_evaluation_error_has_no_indexed_diagnostics(void) {
    ShyValue value = {.kind = SHY_VALUE_INTEGER, .integer_value = -1};
    ShyError *error = NULL;
    ShyStatus status = shy_evaluate_no_vars_ex("1 / 0", &value, &error);

    assert(status == SHY_STATUS_EVALUATION_ERROR);
    assert(value.integer_value == -1);
    assert(error != NULL);
    assert(shy_error_stage(error) == SHY_ERROR_STAGE_EVALUATION);
    assert(shy_error_diagnostic_count(error) == 0);
    assert(shy_error_diagnostic_kind(error, 0) == SHY_DIAGNOSTIC_KIND_NONE);
    assert(shy_error_diagnostic_expected_token(error, 0, 0) == NULL);

    shy_error_free(error);
}

static void test_parsed_expression_free_null(void) {
    shy_parsed_expression_free(NULL);
}

static void test_error_accessors_handle_null(void) {
    assert(shy_error_status(NULL) == SHY_STATUS_NULL_POINTER);
    assert(shy_error_stage(NULL) == SHY_ERROR_STAGE_NONE);
    assert(shy_error_code(NULL) == SHY_ERROR_CODE_NULL_POINTER);
    assert(shy_error_message(NULL) == NULL);
    assert(shy_error_has_span(NULL) == 0);
    assert(shy_error_span_start(NULL) == -1);
    assert(shy_error_span_end(NULL) == -1);
    assert(shy_error_diagnostic_count(NULL) == 0);
    shy_error_free(NULL);
}

int main(void) {
    test_integer_success();
    test_null_expression();
    test_null_output();
    test_evaluation_error();
    test_invalid_utf8();
    test_eval_options_default();
    test_no_vars_with_options_success();
    test_no_vars_with_options_token_limit();
    test_with_options_rejects_invalid_options();
    test_callback_integer_success();
    test_callback_with_options_success();
    test_callback_repeated_lookup();
    test_callback_null_resolver();
    test_callback_resolver_error();
    test_callback_invalid_value_kind();
    test_callback_invalid_status();
    test_callback_null_expression();
    test_no_vars_ex_success_clears_error();
    test_callback_ex_success_clears_error();
    test_no_vars_ex_reports_division_by_zero();
    test_no_vars_ex_reports_lexical_span();
    test_no_vars_ex_error_output_may_be_null();
    test_callback_ex_reports_resolver_error();
    test_callback_ex_reports_invalid_value_kind();
    test_parse_and_evaluate_no_vars();
    test_parse_with_options_success();
    test_parse_once_evaluate_many();
    test_parsed_callback_uses_runtime_user_data();
    test_parsed_ex_evaluation_success_clears_error();
    test_parse_expression_ex_reports_parse_error();
    test_diagnostic_accessors_handle_null();
    test_parse_diagnostic_iteration();
    test_evaluation_error_has_no_indexed_diagnostics();
    test_parsed_expression_free_null();
    test_error_accessors_handle_null();

    puts("shunting_yard_ffi smoke test passed");
    return 0;
}
