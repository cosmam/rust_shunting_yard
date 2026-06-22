#![allow(clippy::ptr_arg, clippy::vec_box)]

use lalrpop_util::lalrpop_mod;
use std::collections::HashMap;

mod ast;
mod eval;
mod lexer;
mod tokens;

pub use tokens::LexicalError;

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

/// Resolves variable names during expression evaluation.
///
/// Implement this trait when values should come from a source other than a
/// [`HashMap`], such as a runtime context, cache, external environment, or
/// future FFI adapter.
///
/// Returned values are validated by evaluation before use, so invalid
/// floating-point values such as NaN, infinity, and subnormal floats are
/// rejected.
pub trait VariableResolver {
    /// Resolve one variable name into a runtime value.
    ///
    /// # Errors
    ///
    /// Returns [`EvalError`] when the variable is unknown or resolution fails.
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError>;
}

impl<F> VariableResolver for F
where
    F: FnMut(&str) -> Result<Value, EvalError>,
{
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        self(name)
    }
}

impl VariableResolver for &HashMap<String, Value> {
    fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
        self.get(name)
            .cloned()
            .ok_or_else(|| EvalError::UnknownVariable(name.to_string()))
    }
}

/// Integer arithmetic operation associated with a checked arithmetic failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithmeticOp {
    /// Unary negation.
    Negate,
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Modulo/remainder.
    Modulo,
    /// Euclidean remainder.
    Remainder,
    /// Exponentiation.
    Power,
    /// Left shift.
    ShiftLeft,
    /// Right shift.
    ShiftRight,
    /// Float-to-integer conversion.
    FloatToInteger,
    /// Round-to-precision operation.
    Round,
    /// Floor-to-precision operation.
    Floor,
    /// Ceiling-to-precision operation.
    Ceiling,
}

/// Resource limit exceeded before or during evaluation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ResourceLimitError {
    /// Input is longer than the configured maximum.
    #[error("input too large: {actual} bytes exceeds {max} bytes")]
    InputTooLarge {
        /// Actual input length in bytes.
        actual: usize,
        /// Maximum allowed input length in bytes.
        max: usize,
    },
    /// Token count exceeds the configured maximum.
    #[error("too many tokens: {actual} exceeds {max}")]
    TooManyTokens {
        /// Actual token count.
        actual: usize,
        /// Maximum allowed token count.
        max: usize,
    },
    /// AST node count exceeds the configured maximum.
    #[error("AST too large: {actual} nodes exceeds {max}")]
    AstTooLarge {
        /// Actual AST node count when the limit was exceeded.
        actual: usize,
        /// Maximum allowed AST node count.
        max: usize,
    },
    /// AST nesting depth exceeds the configured maximum.
    #[error("expression too deep: depth {actual} exceeds {max}")]
    ExpressionTooDeep {
        /// Actual depth when the limit was exceeded.
        actual: usize,
        /// Maximum allowed depth.
        max: usize,
    },
    /// A function call has too many arguments.
    #[error("too many function arguments: {actual} exceeds {max}")]
    TooManyFunctionArguments {
        /// Actual argument count.
        actual: usize,
        /// Maximum allowed argument count.
        max: usize,
    },
    /// Parser recovery count exceeds the configured maximum.
    #[error("too many parser recoveries: {actual} exceeds {max}")]
    TooManyParserRecoveries {
        /// Actual parser recovery count.
        actual: usize,
        /// Maximum allowed parser recovery count.
        max: usize,
    },
}

/// Options controlling resource limits for expression evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalOptions {
    /// Maximum input size in bytes.
    pub max_input_bytes: usize,
    /// Maximum number of lexer tokens.
    pub max_tokens: usize,
    /// Maximum number of AST nodes.
    pub max_ast_nodes: usize,
    /// Maximum AST nesting depth.
    pub max_depth: usize,
    /// Maximum arguments accepted for one function call.
    pub max_function_args: usize,
    /// Maximum parser recoveries accepted after parsing.
    pub max_parser_recoveries: usize,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024,
            max_tokens: 4096,
            max_ast_nodes: 4096,
            max_depth: 256,
            max_function_args: 256,
            max_parser_recoveries: 64,
        }
    }
}

/// Error returned when expression evaluation fails.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum EvalError {
    /// An operator or function was used with an unsupported number of operands.
    #[error("invalid operand count: expected {expected}, got {actual}")]
    InvalidArity {
        /// Expected operand count or count description.
        expected: &'static str,
        /// Actual operand count.
        actual: usize,
    },
    /// The expression tree contains an error node or lexical error node.
    #[error("invalid expression")]
    InvalidExpression,
    /// An invalid type was passed to a calculation
    #[error("invalid type: expected {expected}, got {actual}")]
    InvalidType {
        /// Expected value type.
        expected: &'static str,
        /// Actual value type.
        actual: &'static str,
    },
    /// Division, modulo, or remainder by zero.
    #[error("division by zero")]
    DivisionByZero,
    /// Checked integer arithmetic overflowed.
    #[error("integer overflow during {op:?}")]
    IntegerOverflow {
        /// Operation that overflowed.
        op: ArithmeticOp,
    },
    /// A shift count was negative or too large for `i64`.
    #[error("invalid shift count: {count}")]
    InvalidShiftCount {
        /// Rejected shift count.
        count: i64,
    },
    /// Integer exponent could not be converted to a supported power.
    #[error("invalid exponent: {exponent}")]
    InvalidExponent {
        /// Rejected exponent.
        exponent: i64,
    },
    /// A rounding precision was zero, negative, or non-finite.
    #[error("invalid precision")]
    InvalidPrecision,
    /// A floating-point operation produced NaN or infinity.
    #[error("non-finite float result")]
    NonFiniteFloat,
    /// A floating-point operation produced a subnormal result.
    #[error("subnormal float result")]
    SubnormalFloat,
    /// Evaluation stopped because a resource limit was exceeded.
    #[error("{0}")]
    ResourceLimit(ResourceLimitError),
    /// The lexer found malformed input.
    #[error("lexical error: {0}")]
    LexicalError(LexicalError),
    /// The parser ran into an error it couldn't recover from
    #[error("parser error")]
    ParserError,
    /// The parser recovered from malformed input.
    #[error("parser recovered from {count} errors")]
    ParserRecovery {
        /// Number of recoveries reported by the parser.
        count: usize,
    },
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
    evaluate_with_options(text, variables, &EvalOptions::default())
}

/// Parse and evaluate an expression string with a custom variable resolver.
///
/// This is the callback-backed counterpart to [`evaluate`]. It uses
/// [`EvalOptions::default`] for resource limits.
///
/// # Examples
///
/// ```
/// # use shunting_yard::{evaluate_with, EvalError, Value};
/// let value = evaluate_with("x + 2", |name| match name {
///     "x" => Ok(Value::Integer(40)),
///     other => Err(EvalError::UnknownVariable(other.to_string())),
/// });
///
/// assert_eq!(value, Ok(Value::Integer(42)));
/// ```
///
/// # Errors
///
/// Returns [`EvalError::ParserError`] when `text` is not a valid expression.
/// Evaluation errors from parsing, resource limits, expression evaluation, or
/// the resolver are returned unchanged.
pub fn evaluate_with<R>(text: &str, resolver: R) -> Result<Value, EvalError>
where
    R: FnMut(&str) -> Result<Value, EvalError>,
{
    evaluate_with_options_and_resolver(text, resolver, &EvalOptions::default())
}

/// Parse and evaluate an expression string with explicit resource limits.
///
/// This map-backed entrypoint preserves the original public API while routing
/// variable lookup through [`VariableResolver`].
///
/// See [`evaluate`] for the default-limited map-backed entrypoint.
pub fn evaluate_with_options(
    text: &str,
    variables: &HashMap<String, Value>,
    options: &EvalOptions,
) -> Result<Value, EvalError> {
    evaluate_with_options_and_resolver(text, variables, options)
}

/// Parse and evaluate an expression string with explicit resource limits and a
/// custom variable resolver.
///
/// This is the most general public entrypoint: callers provide both resource
/// limits and the variable resolution strategy.
///
/// # Errors
///
/// Returns [`EvalError::ResourceLimit`] when `text`, token count, AST size,
/// depth, function arity, or parser recovery count exceeds `options`.
/// Parser, lexer, evaluation, and resolver errors are returned unchanged.
pub fn evaluate_with_options_and_resolver<R>(
    text: &str,
    resolver: R,
    options: &EvalOptions,
) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    if text.len() > options.max_input_bytes {
        return Err(EvalError::ResourceLimit(
            ResourceLimitError::InputTooLarge {
                actual: text.len(),
                max: options.max_input_bytes,
            },
        ));
    }

    let lexer = lexer::Lexer::new(text);
    evaluate_tokens_with_options_and_resolver(lexer, resolver, options)
}

#[cfg(test)]
fn evaluate_tokens<'input, Tokens>(
    tokens: Tokens,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
{
    evaluate_tokens_with_options_and_resolver(tokens, variables, &EvalOptions::default())
}

fn evaluate_tokens_with_options_and_resolver<'input, Tokens, R>(
    tokens: Tokens,
    resolver: R,
    options: &EvalOptions,
) -> Result<Value, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
    R: VariableResolver,
{
    let parser = calc::ExpressionParser::new();
    let mut checked_tokens = Vec::new();

    for token in tokens {
        if checked_tokens.len() >= options.max_tokens {
            return Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyTokens {
                    actual: checked_tokens.len() + 1,
                    max: options.max_tokens,
                },
            ));
        }

        match token {
            Ok((_, tokens::Token::Error(error), _)) | Err(error) => {
                return Err(EvalError::LexicalError(error));
            }
            Ok(token) => checked_tokens.push(Ok(token)),
        }
    }

    let mut errors = Vec::new();
    let result = parser.parse(&mut errors, checked_tokens);

    match result {
        Ok(ast) => {
            if !errors.is_empty() {
                if errors.len() > options.max_parser_recoveries {
                    return Err(EvalError::ResourceLimit(
                        ResourceLimitError::TooManyParserRecoveries {
                            actual: errors.len(),
                            max: options.max_parser_recoveries,
                        },
                    ));
                }

                return Err(EvalError::ParserRecovery {
                    count: errors.len(),
                });
            }

            validate_ast_limits(&ast, options)?;
            eval::eval(&ast, resolver)
        }
        Err(_) => Err(EvalError::ParserError),
    }
}

fn validate_ast_limits(expr: &ast::Expression<'_>, options: &EvalOptions) -> Result<(), EvalError> {
    fn walk(
        expr: &ast::Expression<'_>,
        depth: usize,
        nodes: &mut usize,
        options: &EvalOptions,
    ) -> Result<(), EvalError> {
        *nodes += 1;

        if *nodes > options.max_ast_nodes {
            return Err(EvalError::ResourceLimit(ResourceLimitError::AstTooLarge {
                actual: *nodes,
                max: options.max_ast_nodes,
            }));
        }

        if depth > options.max_depth {
            return Err(EvalError::ResourceLimit(
                ResourceLimitError::ExpressionTooDeep {
                    actual: depth,
                    max: options.max_depth,
                },
            ));
        }

        match expr {
            ast::Expression::UnaryOperation { value, .. } => {
                walk(value, depth + 1, nodes, options)?;
            }
            ast::Expression::BinaryOperation { lhs, rhs, .. } => {
                walk(lhs, depth + 1, nodes, options)?;
                walk(rhs, depth + 1, nodes, options)?;
            }
            ast::Expression::Function { arguments, .. } => {
                if arguments.len() > options.max_function_args {
                    return Err(EvalError::ResourceLimit(
                        ResourceLimitError::TooManyFunctionArguments {
                            actual: arguments.len(),
                            max: options.max_function_args,
                        },
                    ));
                }

                for argument in arguments {
                    walk(argument, depth + 1, nodes, options)?;
                }
            }
            ast::Expression::Bool(_)
            | ast::Expression::Integer(_)
            | ast::Expression::Float(_)
            | ast::Expression::Variable(_)
            | ast::Expression::LexicalError(_)
            | ast::Expression::Error => {}
        }

        Ok(())
    }

    let mut nodes = 0;
    walk(expr, 0, &mut nodes, options)
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

    fn is_reserved_variable_name(name: &str) -> bool {
        matches!(
            name,
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
    }

    fn variable_name() -> impl Strategy<Value = String> {
        "[a-zA-Z_][a-zA-Z0-9_]{0,12}".prop_filter("not a reserved function name", |name| {
            !is_reserved_variable_name(name)
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
            Err(EvalError::LexicalError(tokens::LexicalError::InvalidToken))
        );
    }

    #[test]
    fn reserved_function_names_are_not_variable_names() {
        assert!(is_reserved_variable_name("min"));
        assert!(is_reserved_variable_name("ceiling"));
        assert!(!is_reserved_variable_name("minimum"));
        assert!(!is_reserved_variable_name("runtime_value"));
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
            Err(EvalError::LexicalError(tokens::LexicalError::InvalidToken))
        );
    }

    #[test]
    fn evaluate_hostile_inputs_do_not_panic() {
        let mut variables = HashMap::new();
        variables.insert("low".to_owned(), Value::Integer(i64::MIN));
        variables.insert("high".to_owned(), Value::Integer(i64::MAX));
        variables.insert("inf_var".to_owned(), Value::Float(f64::INFINITY));
        variables.insert("nan_var".to_owned(), Value::Float(f64::NAN));
        variables.insert("tiny".to_owned(), Value::Float(f64::MIN_POSITIVE / 2.0));
        variables.insert("min_normal".to_owned(), Value::Float(f64::MIN_POSITIVE));

        let cases = [
            "9223372036854775807 + 1",
            "3037000500 * 3037000500",
            "low / -1",
            "low % -1",
            "rem(low, -1)",
            "1 / 0",
            "1 % 0",
            "mod(1, 0)",
            "rem(1, 0)",
            "1 << -1",
            "1 << 64",
            "1 >> -1",
            "1 >> 64",
            "pow(2, -1)",
            "pow(2, 63)",
            "ln(-1)",
            "acos(2)",
            "asin(2)",
            "exp(10000)",
            "1e308 * 1e308",
            "inf_var",
            "nan_var",
            "tiny",
            "min_normal / 2.0",
            "inf_var + 1",
            "nan_var + 1",
            "$",
            "1 +",
        ];

        for case in cases {
            let result = std::panic::catch_unwind(|| {
                let _ = evaluate(case, &variables);
            });

            assert!(result.is_ok(), "{case:?} panicked");
        }
    }

    #[test]
    fn evaluate_returns_typed_errors_for_hostile_inputs() {
        let mut variables = HashMap::new();
        variables.insert("low".to_owned(), Value::Integer(i64::MIN));
        variables.insert("inf_var".to_owned(), Value::Float(f64::INFINITY));
        variables.insert("nan_var".to_owned(), Value::Float(f64::NAN));
        variables.insert("tiny".to_owned(), Value::Float(f64::MIN_POSITIVE / 2.0));
        variables.insert("min_normal".to_owned(), Value::Float(f64::MIN_POSITIVE));

        assert_eq!(
            evaluate("9223372036854775807 + 1", &variables),
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Add,
            })
        );
        assert_eq!(
            evaluate("3037000500 * 3037000500", &variables),
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Multiply,
            })
        );
        assert_eq!(
            evaluate("low / -1", &variables),
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Divide,
            })
        );
        assert_eq!(
            evaluate("1 / 0", &variables),
            Err(EvalError::DivisionByZero)
        );
        assert_eq!(
            evaluate("1 << -1", &variables),
            Err(EvalError::InvalidShiftCount { count: -1 })
        );
        assert_eq!(
            evaluate("pow(2, -1)", &variables),
            Err(EvalError::InvalidExponent { exponent: -1 })
        );
        assert_eq!(
            evaluate("ln(-1)", &variables),
            Err(EvalError::NonFiniteFloat)
        );
        assert_eq!(
            evaluate("inf_var + 1", &variables),
            Err(EvalError::NonFiniteFloat)
        );
        assert_eq!(
            evaluate("inf_var", &variables),
            Err(EvalError::NonFiniteFloat)
        );
        assert_eq!(
            evaluate("nan_var", &variables),
            Err(EvalError::NonFiniteFloat)
        );
        assert_eq!(evaluate("tiny", &variables), Err(EvalError::SubnormalFloat));
        assert_eq!(
            evaluate("min_normal / 2.0", &variables),
            Err(EvalError::SubnormalFloat)
        );
        assert_eq!(
            evaluate("true + 1", &variables),
            Err(EvalError::InvalidType {
                expected: "integer or float",
                actual: "bool",
            })
        );
        assert_eq!(
            evaluate("abs(1, 2)", &variables),
            Err(EvalError::InvalidArity {
                expected: "1",
                actual: 2,
            })
        );
        assert_eq!(
            evaluate("$", &variables),
            Err(EvalError::LexicalError(LexicalError::UnknownSymbol(
                "$".to_owned()
            )))
        );
        assert_eq!(
            evaluate("1 +", &variables),
            Err(EvalError::ParserRecovery { count: 1 })
        );
    }

    #[test]
    fn evaluate_with_options_enforces_resource_limits() {
        let variables = HashMap::new();

        let options = EvalOptions {
            max_input_bytes: 1,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("1 + 2", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::InputTooLarge { actual: 5, max: 1 }
            ))
        );

        let options = EvalOptions {
            max_tokens: 1,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("1 + 2", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyTokens { actual: 2, max: 1 }
            ))
        );

        let options = EvalOptions {
            max_ast_nodes: 1,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("1 + 2", &variables, &options),
            Err(EvalError::ResourceLimit(ResourceLimitError::AstTooLarge {
                actual: 2,
                max: 1,
            }))
        );

        let options = EvalOptions {
            max_depth: 1,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("!!true", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::ExpressionTooDeep { actual: 2, max: 1 }
            ))
        );

        let options = EvalOptions {
            max_function_args: 1,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("min(1, 2)", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyFunctionArguments { actual: 2, max: 1 }
            ))
        );

        let options = EvalOptions {
            max_parser_recoveries: 0,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("1 +", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyParserRecoveries { actual: 1, max: 0 }
            ))
        );
    }

    #[test]
    fn evaluate_with_resolves_variable_from_callback() {
        let result = evaluate_with("base + 2", |name: &str| {
            assert_eq!(name, "base");
            Ok(Value::Integer(40))
        });

        assert_eq!(result, Ok(Value::Integer(42)));
    }

    #[test]
    fn evaluate_with_infers_callback_argument_type() {
        let result = evaluate_with("x", |name| match name {
            "x" => Ok(Value::Integer(7)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        });

        assert_eq!(result, Ok(Value::Integer(7)));
    }

    #[test]
    fn evaluate_with_reports_unknown_variable_from_callback() {
        let result = evaluate_with("missing", |name: &str| {
            Err(EvalError::UnknownVariable(name.to_owned()))
        });

        assert_eq!(
            result,
            Err(EvalError::UnknownVariable("missing".to_owned()))
        );
    }

    #[test]
    fn evaluate_with_propagates_callback_error() {
        let result = evaluate_with("x", |_name: &str| Err(EvalError::InvalidExpression));

        assert_eq!(result, Err(EvalError::InvalidExpression));
    }

    #[test]
    fn evaluate_with_allows_mutating_resolver_state() {
        let mut calls = 0;

        let result = evaluate_with("x + x", |name: &str| {
            assert_eq!(name, "x");
            calls += 1;
            Ok(Value::Integer(calls))
        });

        assert_eq!(result, Ok(Value::Integer(3)));
        assert_eq!(calls, 2);
    }

    #[test]
    fn evaluate_with_rejects_invalid_callback_floats_without_panicking() {
        let cases = [
            (f64::INFINITY, EvalError::NonFiniteFloat),
            (f64::NEG_INFINITY, EvalError::NonFiniteFloat),
            (f64::NAN, EvalError::NonFiniteFloat),
            (f64::MIN_POSITIVE / 2.0, EvalError::SubnormalFloat),
        ];

        for (value, expected) in cases {
            let result = std::panic::catch_unwind(|| {
                evaluate_with("bad", |name: &str| {
                    assert_eq!(name, "bad");
                    Ok(Value::Float(value))
                })
            });

            assert!(result.is_ok(), "{value:?} panicked");
            assert_eq!(result.unwrap(), Err(expected));
        }
    }

    #[test]
    fn evaluate_with_options_and_resolver_enforces_resource_limits() {
        use std::cell::Cell;

        let options = EvalOptions {
            max_input_bytes: 1,
            ..EvalOptions::default()
        };
        let called = Cell::new(false);
        let mut resolver = |_name: &str| {
            called.set(true);
            Ok(Value::Integer(0))
        };

        let result = evaluate_with_options_and_resolver("1 + 2", &mut resolver, &options);

        assert_eq!(
            result,
            Err(EvalError::ResourceLimit(
                ResourceLimitError::InputTooLarge { actual: 5, max: 1 }
            ))
        );
        assert!(!called.get());

        assert_eq!(
            evaluate_with_options_and_resolver("x", &mut resolver, &EvalOptions::default()),
            Ok(Value::Integer(0))
        );
        assert!(called.get());
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
                Err(EvalError::DivisionByZero)
            );
        }

        #[test]
        fn prop_integer_modulo_by_zero_returns_math_error(lhs in -1_000_000i64..1_000_000) {
            let expression = format!("{lhs} % 0");

            prop_assert_eq!(
                evaluate(&expression, &HashMap::new()),
                Err(EvalError::DivisionByZero)
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
        fn prop_hashmap_and_callback_resolution_match(
            name in variable_name(),
            value in -1_000_000i64..1_000_000,
            addend in -1_000_000i64..1_000_000,
        ) {
            let expression = format!("{name} + {addend}");
            let mut variables = HashMap::new();
            variables.insert(name.clone(), Value::Integer(value));

            let callback_result = evaluate_with(&expression, |candidate: &str| {
                variables
                    .get(candidate)
                    .cloned()
                    .ok_or_else(|| EvalError::UnknownVariable(candidate.to_owned()))
            });

            prop_assert_eq!(callback_result, evaluate(&expression, &variables));
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

            assert!(matches!(
                evaluate(&text, &HashMap::new()),
                Err(EvalError::InvalidType { .. })
            ));
        }

        #[test]
        fn prop_arbitrary_short_input_never_panics(input in ".{0,64}") {
            let _ = evaluate(&input, &HashMap::new());
        }
    }
}
