#include <stddef.h>
#include <stdint.h>

#include "shunting_yard_ffi.h"

_Static_assert(sizeof(ShyStatus) == 4, "ShyStatus size changed");

_Static_assert(SHY_STATUS_OK == 0, "SHY_STATUS_OK changed");
_Static_assert(SHY_STATUS_NULL_POINTER == 1, "SHY_STATUS_NULL_POINTER changed");
_Static_assert(SHY_STATUS_INVALID_UTF8 == 2, "SHY_STATUS_INVALID_UTF8 changed");
_Static_assert(
    SHY_STATUS_EVALUATION_ERROR == 3,
    "SHY_STATUS_EVALUATION_ERROR changed"
);
_Static_assert(SHY_STATUS_PANIC == 4, "SHY_STATUS_PANIC changed");
_Static_assert(SHY_STATUS_RESOLVER_ERROR == 5, "SHY_STATUS_RESOLVER_ERROR changed");
_Static_assert(SHY_STATUS_INVALID_VALUE == 6, "SHY_STATUS_INVALID_VALUE changed");
_Static_assert(SHY_STATUS_INVALID_OPTIONS == 7, "SHY_STATUS_INVALID_OPTIONS changed");

_Static_assert(SHY_VALUE_BOOL == 0, "SHY_VALUE_BOOL changed");
_Static_assert(SHY_VALUE_INTEGER == 1, "SHY_VALUE_INTEGER changed");
_Static_assert(SHY_VALUE_FLOAT == 2, "SHY_VALUE_FLOAT changed");

_Static_assert(SHY_ERROR_STAGE_NONE == 0, "SHY_ERROR_STAGE_NONE changed");
_Static_assert(SHY_ERROR_STAGE_INPUT == 1, "SHY_ERROR_STAGE_INPUT changed");
_Static_assert(SHY_ERROR_STAGE_LEXICAL == 2, "SHY_ERROR_STAGE_LEXICAL changed");
_Static_assert(SHY_ERROR_STAGE_PARSE == 3, "SHY_ERROR_STAGE_PARSE changed");
_Static_assert(
    SHY_ERROR_STAGE_RESOURCE_LIMIT == 4,
    "SHY_ERROR_STAGE_RESOURCE_LIMIT changed"
);
_Static_assert(SHY_ERROR_STAGE_EVALUATION == 5, "SHY_ERROR_STAGE_EVALUATION changed");
_Static_assert(SHY_ERROR_STAGE_RESOLVER == 6, "SHY_ERROR_STAGE_RESOLVER changed");
_Static_assert(SHY_ERROR_STAGE_PANIC == 7, "SHY_ERROR_STAGE_PANIC changed");
_Static_assert(
    SHY_ERROR_STAGE_INVALID_VALUE == 8,
    "SHY_ERROR_STAGE_INVALID_VALUE changed"
);

_Static_assert(SHY_ERROR_CODE_NONE == 0, "SHY_ERROR_CODE_NONE changed");
_Static_assert(SHY_ERROR_CODE_NULL_POINTER == 1, "SHY_ERROR_CODE_NULL_POINTER changed");
_Static_assert(SHY_ERROR_CODE_INVALID_UTF8 == 2, "SHY_ERROR_CODE_INVALID_UTF8 changed");
_Static_assert(SHY_ERROR_CODE_PANIC == 3, "SHY_ERROR_CODE_PANIC changed");
_Static_assert(SHY_ERROR_CODE_LEXICAL_ERROR == 100, "lexical error code changed");
_Static_assert(SHY_ERROR_CODE_PARSE_ERROR == 200, "parse error code changed");
_Static_assert(SHY_ERROR_CODE_PARSE_RECOVERY == 201, "parse recovery code changed");
_Static_assert(SHY_ERROR_CODE_RESOURCE_LIMIT == 300, "resource error code changed");
_Static_assert(SHY_ERROR_CODE_INPUT_TOO_LARGE == 301, "input limit code changed");
_Static_assert(SHY_ERROR_CODE_TOO_MANY_TOKENS == 302, "token limit code changed");
_Static_assert(SHY_ERROR_CODE_AST_TOO_LARGE == 303, "AST limit code changed");
_Static_assert(SHY_ERROR_CODE_EXPRESSION_TOO_DEEP == 304, "depth limit code changed");
_Static_assert(
    SHY_ERROR_CODE_TOO_MANY_FUNCTION_ARGUMENTS == 305,
    "function argument limit code changed"
);
_Static_assert(
    SHY_ERROR_CODE_TOO_MANY_PARSER_RECOVERIES == 306,
    "parser recovery limit code changed"
);
_Static_assert(SHY_ERROR_CODE_EVAL_ERROR == 400, "eval error code changed");
_Static_assert(SHY_ERROR_CODE_INVALID_ARITY == 401, "arity error code changed");
_Static_assert(SHY_ERROR_CODE_INVALID_TYPE == 402, "type error code changed");
_Static_assert(
    SHY_ERROR_CODE_DIVISION_BY_ZERO == 403,
    "division-by-zero code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INTEGER_OVERFLOW == 404,
    "integer overflow code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INVALID_SHIFT_COUNT == 405,
    "invalid shift code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INVALID_EXPONENT == 406,
    "invalid exponent code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INVALID_PRECISION == 407,
    "invalid precision code changed"
);
_Static_assert(
    SHY_ERROR_CODE_NON_FINITE_FLOAT == 408,
    "non-finite float code changed"
);
_Static_assert(
    SHY_ERROR_CODE_SUBNORMAL_FLOAT == 409,
    "subnormal float code changed"
);
_Static_assert(
    SHY_ERROR_CODE_UNEXPECTED_OPCODE == 410,
    "unexpected opcode code changed"
);
_Static_assert(
    SHY_ERROR_CODE_UNKNOWN_VARIABLE == 411,
    "unknown variable code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INVALID_EXPRESSION == 412,
    "invalid expression code changed"
);
_Static_assert(
    SHY_ERROR_CODE_RESOLVER_ERROR == 500,
    "resolver error code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INVALID_VALUE_KIND == 600,
    "invalid value kind code changed"
);
_Static_assert(
    SHY_ERROR_CODE_INVALID_OPTIONS == 700,
    "invalid options code changed"
);

_Static_assert(SHY_DIAGNOSTIC_KIND_NONE == 0, "diagnostic none kind changed");
_Static_assert(
    SHY_DIAGNOSTIC_KIND_INVALID_TOKEN == 1,
    "diagnostic invalid token kind changed"
);
_Static_assert(
    SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_EOF == 2,
    "diagnostic unrecognized EOF kind changed"
);
_Static_assert(
    SHY_DIAGNOSTIC_KIND_UNRECOGNIZED_TOKEN == 3,
    "diagnostic unrecognized token kind changed"
);
_Static_assert(
    SHY_DIAGNOSTIC_KIND_EXTRA_TOKEN == 4,
    "diagnostic extra token kind changed"
);
_Static_assert(SHY_DIAGNOSTIC_KIND_USER == 5, "diagnostic user kind changed");
_Static_assert(
    SHY_DIAGNOSTIC_KIND_RECOVERY == 6,
    "diagnostic recovery kind changed"
);

_Static_assert(sizeof(ShyValue) == 24, "ShyValue size changed");
_Static_assert(offsetof(ShyValue, kind) == 0, "ShyValue.kind offset changed");
_Static_assert(
    offsetof(ShyValue, bool_value) == 4,
    "ShyValue.bool_value offset changed"
);
_Static_assert(
    offsetof(ShyValue, integer_value) == 8,
    "ShyValue.integer_value offset changed"
);
_Static_assert(
    offsetof(ShyValue, float_value) == 16,
    "ShyValue.float_value offset changed"
);

_Static_assert(sizeof(ShyEvalOptions) == 56, "ShyEvalOptions size changed");
_Static_assert(
    offsetof(ShyEvalOptions, abi_size) == 0,
    "ShyEvalOptions.abi_size offset changed"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_input_bytes) == 8,
    "ShyEvalOptions.max_input_bytes offset changed"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_tokens) == 16,
    "ShyEvalOptions.max_tokens offset changed"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_ast_nodes) == 24,
    "ShyEvalOptions.max_ast_nodes offset changed"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_depth) == 32,
    "ShyEvalOptions.max_depth offset changed"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_function_args) == 40,
    "ShyEvalOptions.max_function_args offset changed"
);
_Static_assert(
    offsetof(ShyEvalOptions, max_parser_recoveries) == 48,
    "ShyEvalOptions.max_parser_recoveries offset changed"
);

static ShyStatus abi_resolver(
    const char *name,
    void *user_data,
    ShyValue *out_value
) {
    (void)name;
    (void)user_data;
    (void)out_value;
    return SHY_STATUS_RESOLVER_ERROR;
}

static void assert_exported_function_names(void) {
    ShyVariableResolver resolver = abi_resolver;

    ShyStatus (*eval_options_default_fn)(ShyEvalOptions *) =
        shy_eval_options_default;
    ShyStatus (*parse_fn)(const char *, ShyParsedExpression **) =
        shy_parse_expression;
    ShyStatus (*parse_with_options_fn)(
        const char *,
        const ShyEvalOptions *,
        ShyParsedExpression **
    ) = shy_parse_expression_with_options;
    ShyStatus (*parse_ex_fn)(const char *, ShyParsedExpression **, ShyError **) =
        shy_parse_expression_ex;
    ShyStatus (*parse_with_options_ex_fn)(
        const char *,
        const ShyEvalOptions *,
        ShyParsedExpression **,
        ShyError **
    ) = shy_parse_expression_with_options_ex;
    void (*parsed_free_fn)(ShyParsedExpression *) = shy_parsed_expression_free;

    ShyStatus (*eval_no_vars_fn)(const char *, ShyValue *) =
        shy_evaluate_no_vars;
    ShyStatus (*eval_no_vars_with_options_fn)(
        const char *,
        const ShyEvalOptions *,
        ShyValue *
    ) = shy_evaluate_no_vars_with_options;
    ShyStatus (*eval_no_vars_ex_fn)(const char *, ShyValue *, ShyError **) =
        shy_evaluate_no_vars_ex;
    ShyStatus (*eval_no_vars_with_options_ex_fn)(
        const char *,
        const ShyEvalOptions *,
        ShyValue *,
        ShyError **
    ) = shy_evaluate_no_vars_with_options_ex;

    ShyStatus (*eval_callback_fn)(
        const char *,
        ShyVariableResolver,
        void *,
        ShyValue *
    ) = shy_evaluate_with_callback;
    ShyStatus (*eval_callback_with_options_fn)(
        const char *,
        const ShyEvalOptions *,
        ShyVariableResolver,
        void *,
        ShyValue *
    ) = shy_evaluate_with_callback_with_options;
    ShyStatus (*eval_callback_ex_fn)(
        const char *,
        ShyVariableResolver,
        void *,
        ShyValue *,
        ShyError **
    ) = shy_evaluate_with_callback_ex;
    ShyStatus (*eval_callback_with_options_ex_fn)(
        const char *,
        const ShyEvalOptions *,
        ShyVariableResolver,
        void *,
        ShyValue *,
        ShyError **
    ) = shy_evaluate_with_callback_with_options_ex;

    ShyStatus (*eval_parsed_no_vars_fn)(
        const ShyParsedExpression *,
        ShyValue *
    ) = shy_evaluate_parsed_no_vars;
    ShyStatus (*eval_parsed_no_vars_ex_fn)(
        const ShyParsedExpression *,
        ShyValue *,
        ShyError **
    ) = shy_evaluate_parsed_no_vars_ex;
    ShyStatus (*eval_parsed_callback_fn)(
        const ShyParsedExpression *,
        ShyVariableResolver,
        void *,
        ShyValue *
    ) = shy_evaluate_parsed_with_callback;
    ShyStatus (*eval_parsed_callback_ex_fn)(
        const ShyParsedExpression *,
        ShyVariableResolver,
        void *,
        ShyValue *,
        ShyError **
    ) = shy_evaluate_parsed_with_callback_ex;

    void (*error_free_fn)(ShyError *) = shy_error_free;
    ShyStatus (*error_status_fn)(const ShyError *) = shy_error_status;
    int32_t (*error_stage_fn)(const ShyError *) = shy_error_stage;
    int32_t (*error_code_fn)(const ShyError *) = shy_error_code;
    const char *(*error_message_fn)(const ShyError *) = shy_error_message;
    int32_t (*error_has_span_fn)(const ShyError *) = shy_error_has_span;
    int32_t (*error_span_start_fn)(const ShyError *) = shy_error_span_start;
    int32_t (*error_span_end_fn)(const ShyError *) = shy_error_span_end;
    int32_t (*diagnostic_count_fn)(const ShyError *) =
        shy_error_diagnostic_count;
    int32_t (*diagnostic_kind_fn)(const ShyError *, int32_t) =
        shy_error_diagnostic_kind;
    int32_t (*diagnostic_has_span_fn)(const ShyError *, int32_t) =
        shy_error_diagnostic_has_span;
    int32_t (*diagnostic_span_start_fn)(const ShyError *, int32_t) =
        shy_error_diagnostic_span_start;
    int32_t (*diagnostic_span_end_fn)(const ShyError *, int32_t) =
        shy_error_diagnostic_span_end;
    int32_t (*diagnostic_expected_count_fn)(const ShyError *, int32_t) =
        shy_error_diagnostic_expected_count;
    const char *(*diagnostic_expected_token_fn)(
        const ShyError *,
        int32_t,
        int32_t
    ) = shy_error_diagnostic_expected_token;

    (void)resolver;
    (void)eval_options_default_fn;
    (void)parse_fn;
    (void)parse_with_options_fn;
    (void)parse_ex_fn;
    (void)parse_with_options_ex_fn;
    (void)parsed_free_fn;
    (void)eval_no_vars_fn;
    (void)eval_no_vars_with_options_fn;
    (void)eval_no_vars_ex_fn;
    (void)eval_no_vars_with_options_ex_fn;
    (void)eval_callback_fn;
    (void)eval_callback_with_options_fn;
    (void)eval_callback_ex_fn;
    (void)eval_callback_with_options_ex_fn;
    (void)eval_parsed_no_vars_fn;
    (void)eval_parsed_no_vars_ex_fn;
    (void)eval_parsed_callback_fn;
    (void)eval_parsed_callback_ex_fn;
    (void)error_free_fn;
    (void)error_status_fn;
    (void)error_stage_fn;
    (void)error_code_fn;
    (void)error_message_fn;
    (void)error_has_span_fn;
    (void)error_span_start_fn;
    (void)error_span_end_fn;
    (void)diagnostic_count_fn;
    (void)diagnostic_kind_fn;
    (void)diagnostic_has_span_fn;
    (void)diagnostic_span_start_fn;
    (void)diagnostic_span_end_fn;
    (void)diagnostic_expected_count_fn;
    (void)diagnostic_expected_token_fn;
}

int main(void) {
    assert_exported_function_names();
    return 0;
}
