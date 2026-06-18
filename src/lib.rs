#![allow(clippy::ptr_arg, clippy::vec_box)]

use lalrpop_util::lalrpop_mod;
use std::collections::HashMap;

mod ast;
mod eval;
mod lexer;
mod tokens;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(clippy::pedantic)]
    #[allow(dead_code)]
    calc
);

/// Runtime value produced by expression evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Boolean value.
    Bool(bool),
    /// Signed integer value.
    Integer(i64),
    /// Floating-point value.
    Float(f64),
}

/// Error returned when expression evaluation fails.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum EvalError {
    /// An operator or function was used with an unsupported number of operands.
    #[error("invalid operand count")]
    InvalidArity,
    /// The expression tree contains an error node or lexical error node.
    #[error("invalid expression")]
    InvalidExpression,
    /// An invalid type was passed to a calculation
    #[error("invalid type: {0}")]
    InvalidType(String),
    /// There is some math error, such as division by zero
    #[error("math error: {0}")]
    MathError(String),
    /// The parser ran into an error it couldn't recover from
    #[error("parser error")]
    ParserError,
    /// An opcode was found that was already supposed to be filtered out
    #[error("unexpected opcode")]
    UnexpectedOpcode,
    /// A variable reference could not be found in the provided bindings.
    #[error("unknown variable: {0}")]
    UnknownVariable(String),
}

/// Parse and evaluate an expression string.
///
/// # Arguments
///
/// * `text` - Source expression to parse.
/// * `variables` - Runtime bindings used to resolve variable references.
///
/// # Returns
///
/// The evaluated [`Value`] when parsing and evaluation both succeed.
///
/// # Errors
///
/// Returns [`EvalError::ParserError`] when `text` is not a valid expression.
/// Evaluation errors from the parsed expression, such as unknown variables or
/// invalid operations, are returned unchanged.
pub fn evaluate(text: &str, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    let lexer = lexer::Lexer::new(text);
    evaluate_tokens(lexer, variables)
}

fn evaluate_tokens<'input, Tokens>(
    tokens: Tokens,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
{
    let parser = calc::ExpressionParser::new();

    let mut errors = Vec::new();
    let result = parser.parse(&mut errors, tokens);

    match result {
        Ok(ast) => eval::eval(&ast, variables),
        Err(_) => Err(EvalError::ParserError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Clone, Debug)]
    enum PropExpr {
        Int(i64),
        Add(Box<PropExpr>, Box<PropExpr>),
        Mul(Box<PropExpr>, Box<PropExpr>),
    }

    #[derive(Clone, Debug)]
    enum IllTypedExpr {
        AddBoolInt(bool, i64),
    }

    impl PropExpr {
        fn render(&self) -> String {
            match self {
                PropExpr::Int(value) => value.to_string(),
                PropExpr::Add(lhs, rhs) => format!("({} + {})", lhs.render(), rhs.render()),
                PropExpr::Mul(lhs, rhs) => format!("({} * {})", lhs.render(), rhs.render()),
            }
        }

        fn reference_eval(&self) -> i64 {
            match self {
                PropExpr::Int(value) => *value,
                PropExpr::Add(lhs, rhs) => lhs.reference_eval() + rhs.reference_eval(),
                PropExpr::Mul(lhs, rhs) => lhs.reference_eval() * rhs.reference_eval(),
            }
        }
    }

    impl IllTypedExpr {
        fn render(&self) -> String {
            match self {
                IllTypedExpr::AddBoolInt(lhs, rhs) => format!("({lhs} + {rhs})"),
            }
        }
    }

    fn variable_name() -> impl Strategy<Value = String> {
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_filter("not a reserved function name", |name| {
            !matches!(
                name.as_str(),
                "min"
                    | "max"
                    | "pow"
                    | "mod"
                    | "rem"
                    | "round"
                    | "cos"
                    | "sin"
                    | "tan"
                    | "acos"
                    | "asin"
                    | "atan"
                    | "abs"
                    | "ln"
                    | "log"
                    | "exp"
                    | "floor"
                    | "ceil"
                    | "ceiling"
            )
        })
    }

    fn prop_int_expr() -> impl Strategy<Value = PropExpr> {
        (-2i64..3)
            .prop_map(PropExpr::Int)
            .prop_recursive(3, 16, 2, |inner| {
                prop_oneof![
                    (inner.clone(), inner.clone())
                        .prop_map(|(lhs, rhs)| PropExpr::Add(Box::new(lhs), Box::new(rhs))),
                    (inner.clone(), inner)
                        .prop_map(|(lhs, rhs)| PropExpr::Mul(Box::new(lhs), Box::new(rhs))),
                ]
            })
    }

    fn ill_typed_expr() -> impl Strategy<Value = IllTypedExpr> {
        (any::<bool>(), -1_000i64..1_000).prop_map(|(lhs, rhs)| IllTypedExpr::AddBoolInt(lhs, rhs))
    }

    #[test]
    fn evaluate_parses_and_evaluates_expression_text() {
        let mut variables = HashMap::new();
        variables.insert("base".to_owned(), Value::Integer(4));

        assert_eq!(evaluate("base + 2 * 3", &variables), Ok(Value::Integer(10)));
    }

    #[test]
    fn evaluate_reports_parser_errors() {
        let variables = HashMap::new();
        let tokens = [Err(tokens::LexicalError::InvalidToken)];

        assert_eq!(
            evaluate_tokens(tokens, &variables),
            Err(EvalError::ParserError)
        );
    }

    #[test]
    fn evaluate_tokens_covers_success_and_error_for_vec_streams() {
        let variables = HashMap::new();
        let valid_tokens = vec![
            Ok((0, tokens::Token::Integer(1), 1)),
            Ok((2, tokens::Token::Plus, 3)),
            Ok((4, tokens::Token::Integer(2), 5)),
        ];
        let invalid_tokens = vec![Err(tokens::LexicalError::InvalidToken)];

        assert_eq!(
            evaluate_tokens(valid_tokens, &variables),
            Ok(Value::Integer(3))
        );
        assert_eq!(
            evaluate_tokens(invalid_tokens, &variables),
            Err(EvalError::ParserError)
        );
    }

    proptest! {
        #[test]
        fn prop_evaluate_integer_addition_matches_rust(
            lhs in -1_000_000i64..1_000_000,
            rhs in -1_000_000i64..1_000_000,
        ) {
            let expression = format!("({lhs}) + ({rhs})");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Ok(Value::Integer(lhs + rhs))
            );
        }

        #[test]
        fn prop_evaluate_integer_subtraction_matches_rust(
            lhs in -1_000_000i64..1_000_000,
            rhs in -1_000_000i64..1_000_000,
        ) {
            let expression = format!("({lhs}) - ({rhs})");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Ok(Value::Integer(lhs - rhs))
            );
        }

        #[test]
        fn prop_evaluate_integer_multiplication_matches_rust(
            lhs in -10_000i64..10_000,
            rhs in -10_000i64..10_000,
        ) {
            let expression = format!("({lhs}) * ({rhs})");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Ok(Value::Integer(lhs * rhs))
            );
        }

        #[test]
        fn prop_evaluate_integer_division_matches_rust_for_nonzero_divisors(
            lhs in -1_000_000i64..1_000_000,
            rhs in (-1_000_000i64..1_000_000).prop_filter("nonzero divisor", |value| *value != 0),
        ) {
            let expression = format!("({lhs}) / ({rhs})");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Ok(Value::Integer(lhs / rhs))
            );
        }

        #[test]
        fn prop_integer_division_by_zero_returns_math_error(lhs in -1_000_000i64..1_000_000) {
            let expression = format!("{lhs} / 0");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Err(EvalError::MathError("Division by zero".to_string()))
            );
        }

        #[test]
        fn prop_integer_modulo_by_zero_returns_math_error(lhs in -1_000_000i64..1_000_000) {
            let expression = format!("{lhs} % 0");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Err(EvalError::MathError("Modulo by zero".to_string()))
            );
        }

        #[test]
        fn prop_known_variable_evaluates_to_bound_value(
            name in variable_name(),
            value in -1_000_000i64..1_000_000,
        ) {
            let mut variables = HashMap::new();
            variables.insert(name.clone(), Value::Integer(value));

            prop_assert_eq!(
                evaluate(&name, &variables),
                Ok(Value::Integer(value))
            );
        }

        #[test]
        fn prop_unknown_variable_returns_unknown_variable(name in variable_name()) {
            prop_assert_eq!(
                evaluate(&name, &HashMap::new()),
                Err(EvalError::UnknownVariable(name))
            );
        }

        #[test]
        fn prop_min_matches_rust_min(values in prop::collection::vec(-1_000i64..1_000, 1..20)) {
            let arguments = values
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let expression = format!("min({arguments})");
            let expected = values.iter().copied().min().unwrap();

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Ok(Value::Integer(expected))
            );
        }

        #[test]
        fn prop_max_matches_rust_max(values in prop::collection::vec(-1_000i64..1_000, 1..20)) {
            let arguments = values
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let expression = format!("max({arguments})");
            let expected = values.iter().copied().max().unwrap();

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Ok(Value::Integer(expected))
            );
        }

        #[test]
        fn prop_multiplication_binds_before_addition(
            lhs in -1_000i64..1_000,
            rhs in -1_000i64..1_000,
            multiplier in -1_000i64..1_000,
        ) {
            let implicit = format!("{lhs} + {rhs} * {multiplier}");
            let explicit = format!("{lhs} + ({rhs} * {multiplier})");

            prop_assert_eq!(
                evaluate(&implicit, &HashMap::new()),
                evaluate(&explicit, &HashMap::new())
            );
        }

        #[test]
        fn prop_generated_integer_expression_does_not_parser_error(expr in prop_int_expr()) {
            let text = expr.render();

            prop_assert_ne!(
                evaluate(&text, &HashMap::new()),
                Err(EvalError::ParserError)
            );
        }

        #[test]
        fn prop_generated_integer_expression_matches_reference_evaluator(expr in prop_int_expr()) {
            let text = expr.render();

            prop_assert_eq!(
                evaluate(&text, &HashMap::new()),
                Ok(Value::Integer(expr.reference_eval()))
            );
        }

        #[test]
        fn prop_ill_typed_expression_returns_invalid_type(expr in ill_typed_expr()) {
            let text = expr.render();

            prop_assert!(matches!(
                evaluate(&text, &HashMap::new()),
                Err(EvalError::InvalidType(_))
            ));
        }

        #[test]
        fn prop_arbitrary_short_input_never_panics(input in ".{0,64}") {
            let _ = evaluate(&input, &HashMap::new());
        }
    }
}
