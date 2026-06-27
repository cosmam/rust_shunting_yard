#![allow(clippy::ptr_arg, clippy::vec_box)]
#![deny(
    clippy::expect_used,
    clippy::todo,
    clippy::unimplemented,
    clippy::unwrap_used
)]
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
    // LALRPOP emits parser stack unwraps in generated code; keep the allowance
    // scoped to the generated parser module instead of weakening crate policy.
    #[allow(clippy::unwrap_used)]
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
/// Named resolver implementations are passed to resolver-based entrypoints by
/// value. The crate provides a built-in borrowed [`HashMap`] adapter for the
/// map-backed APIs, but it does not provide a blanket implementation for
/// borrowed named resolver types.
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

/// Parsed expression that can be evaluated repeatedly with different resolvers.
#[derive(Clone, Debug)]
pub struct ParsedExpression<'input> {
    ast: ast::Expression<'input>,
}

impl<'input> ParsedExpression<'input> {
    fn new(ast: ast::Expression<'input>) -> Self {
        Self { ast }
    }
}

/// Byte range in the original source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl SourceSpan {
    /// Build a source span from byte offsets.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// One parser diagnostic associated with a source span when one is available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseDiagnostic {
    /// Source range for this diagnostic, if the parser reported one.
    pub span: Option<SourceSpan>,
    /// Structured diagnostic detail.
    pub kind: ParseDiagnosticKind,
}

/// Structured parser diagnostic categories.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseDiagnosticKind {
    /// The parser received an invalid token.
    InvalidToken,
    /// The input ended before the parser could finish an expression.
    UnrecognizedEof {
        /// Token names that would have allowed parsing to continue.
        expected: Vec<String>,
    },
    /// The parser found an unexpected token.
    UnrecognizedToken {
        /// Debug representation of the unexpected token.
        token: String,
        /// Token names that would have allowed parsing to continue.
        expected: Vec<String>,
    },
    /// The parser found extra input after a complete expression.
    ExtraToken {
        /// Debug representation of the extra token.
        token: String,
    },
    /// Parser user error.
    User {
        /// Owned error message.
        message: String,
    },
    /// Parser recovery diagnostic containing the tokens dropped by recovery.
    Recovery {
        /// Debug representations of tokens discarded during recovery.
        dropped_tokens: Vec<String>,
        /// Underlying parser diagnostic that caused recovery.
        cause: Box<ParseDiagnosticKind>,
    },
}

/// Structured parser diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{count} parse diagnostic(s)", count = .diagnostics.len())]
pub struct ParseDiagnostics {
    /// Individual parser diagnostics.
    pub diagnostics: Vec<ParseDiagnostic>,
}

impl ParseDiagnostics {
    /// Return the number of diagnostics.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Return true when no diagnostics are present.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Return the number of parser recovery diagnostics.
    pub fn recovery_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.kind, ParseDiagnosticKind::Recovery { .. }))
            .count()
    }
}

/// Top-level error returned by diagnostic-aware APIs.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum Error {
    /// A configured resource limit was exceeded.
    #[error("{0}")]
    ResourceLimit(ResourceLimitError),
    /// The lexer found malformed source text.
    #[error("lexical error at bytes {span:?}: {error}")]
    Lexical {
        /// Source range for the lexical error.
        span: SourceSpan,
        /// Lexical error detail.
        error: LexicalError,
    },
    /// The parser could not produce a valid expression.
    #[error("{0}")]
    Parse(ParseDiagnostics),
    /// Evaluation failed after parsing succeeded.
    #[error("{0}")]
    Eval(EvalError),
}

impl From<ResourceLimitError> for Error {
    fn from(error: ResourceLimitError) -> Self {
        Error::ResourceLimit(error)
    }
}

impl From<EvalError> for Error {
    fn from(error: EvalError) -> Self {
        Error::Eval(error)
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

/// Parse expression text using default evaluation options.
///
/// # Errors
///
/// Returns parser, lexical, or resource-limit errors when `text` cannot be
/// parsed within [`EvalOptions::default`].
pub fn parse(text: &str) -> Result<ParsedExpression<'_>, EvalError> {
    parse_with_options(text, &EvalOptions::default())
}

/// Parse expression text using default evaluation options and return structured
/// diagnostic errors.
///
/// # Examples
///
/// ```
/// use shunting_yard::{parse_detailed, Error};
///
/// let error = parse_detailed("$");
///
/// assert!(matches!(error, Err(Error::Lexical { .. })));
/// ```
///
/// # Errors
///
/// Returns [`Error::Lexical`], [`Error::Parse`], or [`Error::ResourceLimit`]
/// when `text` cannot be parsed within [`EvalOptions::default`].
pub fn parse_detailed(text: &str) -> Result<ParsedExpression<'_>, Error> {
    parse_with_options_detailed(text, &EvalOptions::default())
}

/// Parse expression text into a reusable parsed expression.
///
/// The returned [`ParsedExpression`] hides the internal AST and can be evaluated
/// repeatedly by APIs that accept parsed expressions.
///
/// # Errors
///
/// Returns [`EvalError::ResourceLimit`] when `text`, token count, AST size,
/// depth, function arity, or parser recovery count exceeds `options`.
/// Parser and lexer errors are returned unchanged.
pub fn parse_with_options<'input>(
    text: &'input str,
    options: &EvalOptions,
) -> Result<ParsedExpression<'input>, EvalError> {
    if text.len() > options.max_input_bytes {
        return Err(EvalError::ResourceLimit(
            ResourceLimitError::InputTooLarge {
                actual: text.len(),
                max: options.max_input_bytes,
            },
        ));
    }

    let lexer = lexer::Lexer::new(text);
    let ast = parse_tokens_with_options(lexer, options)?;
    Ok(ParsedExpression::new(ast))
}

/// Parse expression text into a reusable parsed expression and return
/// structured diagnostic errors.
///
/// The returned [`ParsedExpression`] hides the internal AST and can be evaluated
/// repeatedly by APIs that accept parsed expressions.
///
/// # Errors
///
/// Returns [`Error::ResourceLimit`] when `text`, token count, AST size, depth,
/// function arity, or parser recovery count exceeds `options`. Lexer failures
/// are returned as [`Error::Lexical`], and parser failures are returned as
/// [`Error::Parse`].
pub fn parse_with_options_detailed<'input>(
    text: &'input str,
    options: &EvalOptions,
) -> Result<ParsedExpression<'input>, Error> {
    if text.len() > options.max_input_bytes {
        return Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge {
            actual: text.len(),
            max: options.max_input_bytes,
        }));
    }

    let lexer = lexer::Lexer::new(text);
    let ast = parse_tokens_with_options_detailed(lexer, options)?;
    Ok(ParsedExpression::new(ast))
}

/// Evaluate a previously parsed expression with a resolver.
///
/// # Examples
///
/// ```
/// use shunting_yard::{evaluate_parsed, parse, EvalError, Value};
///
/// let parsed = parse("x + 2")?;
///
/// let value = evaluate_parsed(&parsed, |name: &str| match name {
///     "x" => Ok(Value::Integer(40)),
///     other => Err(EvalError::UnknownVariable(other.to_owned())),
/// })?;
///
/// assert_eq!(value, Value::Integer(42));
/// # Ok::<(), EvalError>(())
/// ```
///
/// # Errors
///
/// Returns resolver or evaluation errors from the parsed expression.
pub fn evaluate_parsed<R>(parsed: &ParsedExpression<'_>, resolver: R) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    eval::eval(&parsed.ast, resolver)
}

/// Evaluate a previously parsed expression with a resolver and return a
/// top-level diagnostic error.
///
/// # Errors
///
/// Returns [`Error::Eval`] when resolver lookup or expression evaluation fails.
pub fn evaluate_parsed_detailed<R>(
    parsed: &ParsedExpression<'_>,
    resolver: R,
) -> Result<Value, Error>
where
    R: VariableResolver,
{
    eval::eval(&parsed.ast, resolver).map_err(Error::Eval)
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

/// Parse and evaluate an expression string with diagnostic-aware errors.
///
/// This map-backed entrypoint is the diagnostic-aware counterpart to
/// [`evaluate`].
///
/// # Examples
///
/// ```
/// use shunting_yard::{evaluate_detailed, Error, EvalError};
/// use std::collections::HashMap;
///
/// let variables = HashMap::new();
/// let error = evaluate_detailed("1 / 0", &variables);
///
/// assert_eq!(error, Err(Error::Eval(EvalError::DivisionByZero)));
/// ```
///
/// # Errors
///
/// Returns [`Error::ResourceLimit`], [`Error::Lexical`], [`Error::Parse`], or
/// [`Error::Eval`] according to the stage that failed.
pub fn evaluate_detailed(text: &str, variables: &HashMap<String, Value>) -> Result<Value, Error> {
    evaluate_with_options_and_resolver_detailed(text, variables, &EvalOptions::default())
}

/// Parse and evaluate an expression string with a custom variable resolver.
///
/// This is the callback-backed counterpart to [`evaluate`]. It uses
/// [`EvalOptions::default`] for resource limits. Use
/// [`evaluate_with_resolver`] when passing a named [`VariableResolver`] type
/// instead of a callback.
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

/// Parse and evaluate an expression string with a custom variable resolver.
///
/// This is the default-limited entrypoint for named [`VariableResolver`]
/// implementations. Use [`evaluate_with`] for callback-based resolution.
/// Named resolvers are passed by value.
/// Resolver-returned values are validated before evaluation uses them.
///
/// # Examples
///
/// ```
/// # use shunting_yard::{evaluate_with_resolver, EvalError, Value, VariableResolver};
/// struct RuntimeResolver;
///
/// impl VariableResolver for RuntimeResolver {
///     fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
///         match name {
///             "x" => Ok(Value::Integer(40)),
///             other => Err(EvalError::UnknownVariable(other.to_owned())),
///         }
///     }
/// }
///
/// let value = evaluate_with_resolver("x + 2", RuntimeResolver);
///
/// assert_eq!(value, Ok(Value::Integer(42)));
/// ```
///
/// # Errors
///
/// Returns parser, lexical, resource-limit, resolver, or evaluation errors.
pub fn evaluate_with_resolver<R>(text: &str, resolver: R) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    evaluate_with_options_and_resolver(text, resolver, &EvalOptions::default())
}

/// Parse and evaluate an expression string with a custom variable resolver and
/// diagnostic-aware errors.
///
/// This is the default-limited diagnostic entrypoint for named
/// [`VariableResolver`] implementations. Closures also implement
/// [`VariableResolver`] and can be passed directly.
///
/// # Errors
///
/// Returns [`Error::ResourceLimit`], [`Error::Lexical`], [`Error::Parse`], or
/// [`Error::Eval`] according to the stage that failed.
pub fn evaluate_with_resolver_detailed<R>(text: &str, resolver: R) -> Result<Value, Error>
where
    R: VariableResolver,
{
    evaluate_with_options_and_resolver_detailed(text, resolver, &EvalOptions::default())
}

/// Parse and evaluate an expression string with explicit resource limits.
///
/// This map-backed entrypoint preserves the original public API while routing
/// variable lookup through the same resolver path as callback evaluation.
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
/// limits and the variable resolution strategy. Named resolvers are passed by
/// value; borrowed named resolvers need their own explicit [`VariableResolver`]
/// implementation for that borrowed type.
///
/// # Errors
///
/// Returns [`EvalError::ResourceLimit`] when `text`, token count, AST size,
/// depth, function arity, or parser recovery count exceeds `options`.
/// Parser, lexer, evaluation, and resolver errors are returned unchanged.
///
/// # Examples
///
/// ```
/// # use shunting_yard::{
/// #     evaluate_with_options_and_resolver, EvalError, EvalOptions, Value, VariableResolver,
/// # };
/// struct RuntimeResolver;
///
/// impl VariableResolver for RuntimeResolver {
///     fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
///         match name {
///             "x" => Ok(Value::Integer(40)),
///             other => Err(EvalError::UnknownVariable(other.to_owned())),
///         }
///     }
/// }
///
/// let options = EvalOptions {
///     max_tokens: 3,
///     ..EvalOptions::default()
/// };
///
/// let value = evaluate_with_options_and_resolver("x + 2", RuntimeResolver, &options);
///
/// assert_eq!(value, Ok(Value::Integer(42)));
/// ```
pub fn evaluate_with_options_and_resolver<R>(
    text: &str,
    resolver: R,
    options: &EvalOptions,
) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    let parsed = parse_with_options(text, options)?;
    evaluate_parsed(&parsed, resolver)
}

/// Parse and evaluate an expression string with explicit resource limits and
/// diagnostic-aware errors.
///
/// This is the most general diagnostic-aware entrypoint: callers provide both
/// resource limits and the variable resolution strategy.
///
/// # Errors
///
/// Returns [`Error::ResourceLimit`], [`Error::Lexical`], [`Error::Parse`], or
/// [`Error::Eval`] according to the stage that failed.
pub fn evaluate_with_options_and_resolver_detailed<R>(
    text: &str,
    resolver: R,
    options: &EvalOptions,
) -> Result<Value, Error>
where
    R: VariableResolver,
{
    let parsed = parse_with_options_detailed(text, options)?;
    evaluate_parsed_detailed(&parsed, resolver)
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

#[cfg(test)]
fn evaluate_tokens_with_options_and_resolver<'input, Tokens, R>(
    tokens: Tokens,
    resolver: R,
    options: &EvalOptions,
) -> Result<Value, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
    R: VariableResolver,
{
    let ast = parse_tokens_with_options(tokens, options)?;
    eval::eval(&ast, resolver)
}

fn parse_tokens_with_options<'input, Tokens>(
    tokens: Tokens,
    options: &EvalOptions,
) -> Result<ast::Expression<'input>, EvalError>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
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
            Ok(*ast)
        }
        Err(_) => Err(EvalError::ParserError),
    }
}

fn parse_tokens_with_options_detailed<'input, Tokens>(
    tokens: Tokens,
    options: &EvalOptions,
) -> Result<ast::Expression<'input>, Error>
where
    Tokens: IntoIterator<Item = lexer::Spanned<tokens::Token<'input>, usize, tokens::LexicalError>>,
{
    let parser = calc::ExpressionParser::new();
    let mut checked_tokens = Vec::new();

    for token in tokens {
        if checked_tokens.len() >= options.max_tokens {
            return Err(Error::ResourceLimit(ResourceLimitError::TooManyTokens {
                actual: checked_tokens.len() + 1,
                max: options.max_tokens,
            }));
        }

        match token {
            Ok((start, tokens::Token::Error(error), end)) => {
                return Err(Error::Lexical {
                    span: SourceSpan::new(start, end),
                    error,
                });
            }
            Err(error) => {
                return Err(Error::Lexical {
                    span: SourceSpan::new(0, 0),
                    error,
                });
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
                    return Err(Error::ResourceLimit(
                        ResourceLimitError::TooManyParserRecoveries {
                            actual: errors.len(),
                            max: options.max_parser_recoveries,
                        },
                    ));
                }

                return Err(Error::Parse(ParseDiagnostics {
                    diagnostics: errors.into_iter().map(recovery_to_diagnostic).collect(),
                }));
            }

            validate_ast_limits(&ast, options).map_err(eval_error_to_stage_error)?;
            Ok(*ast)
        }
        Err(error) => Err(Error::Parse(ParseDiagnostics {
            diagnostics: vec![parse_error_to_diagnostic(error)],
        })),
    }
}

fn eval_error_to_stage_error(error: EvalError) -> Error {
    match error {
        EvalError::ResourceLimit(error) => Error::ResourceLimit(error),
        error => Error::Eval(error),
    }
}

fn parse_error_to_diagnostic<'input>(
    error: lalrpop_util::ParseError<usize, tokens::Token<'input>, tokens::LexicalError>,
) -> ParseDiagnostic {
    match error {
        lalrpop_util::ParseError::InvalidToken { location } => ParseDiagnostic {
            span: Some(SourceSpan::new(location, location)),
            kind: ParseDiagnosticKind::InvalidToken,
        },
        lalrpop_util::ParseError::UnrecognizedEof { location, expected } => ParseDiagnostic {
            span: Some(SourceSpan::new(location, location)),
            kind: ParseDiagnosticKind::UnrecognizedEof { expected },
        },
        lalrpop_util::ParseError::UnrecognizedToken { token, expected } => {
            let (start, token, end) = token;
            ParseDiagnostic {
                span: Some(SourceSpan::new(start, end)),
                kind: ParseDiagnosticKind::UnrecognizedToken {
                    token: format!("{token:?}"),
                    expected,
                },
            }
        }
        lalrpop_util::ParseError::ExtraToken { token } => {
            let (start, token, end) = token;
            ParseDiagnostic {
                span: Some(SourceSpan::new(start, end)),
                kind: ParseDiagnosticKind::ExtraToken {
                    token: format!("{token:?}"),
                },
            }
        }
        lalrpop_util::ParseError::User { error } => ParseDiagnostic {
            span: None,
            kind: ParseDiagnosticKind::User {
                message: error.to_string(),
            },
        },
    }
}

fn recovery_to_diagnostic<'input>(
    recovery: lalrpop_util::ErrorRecovery<usize, tokens::Token<'input>, tokens::LexicalError>,
) -> ParseDiagnostic {
    let cause = parse_error_to_diagnostic(recovery.error);

    ParseDiagnostic {
        span: cause.span,
        kind: ParseDiagnosticKind::Recovery {
            dropped_tokens: recovery
                .dropped_tokens
                .into_iter()
                .map(|(_, token, _)| format!("{token:?}"))
                .collect(),
            cause: Box::new(cause.kind),
        },
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
    fn source_span_records_byte_offsets() {
        assert_eq!(SourceSpan::new(2, 5), SourceSpan { start: 2, end: 5 });
    }

    #[test]
    fn parse_diagnostics_reports_length_and_empty_state() {
        let empty = ParseDiagnostics {
            diagnostics: Vec::new(),
        };
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());

        let diagnostics = ParseDiagnostics {
            diagnostics: vec![ParseDiagnostic {
                span: Some(SourceSpan::new(1, 2)),
                kind: ParseDiagnosticKind::InvalidToken,
            }],
        };
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn parse_diagnostics_counts_recoveries() {
        let diagnostics = ParseDiagnostics {
            diagnostics: vec![
                ParseDiagnostic {
                    span: Some(SourceSpan::new(1, 2)),
                    kind: ParseDiagnosticKind::InvalidToken,
                },
                ParseDiagnostic {
                    span: Some(SourceSpan::new(2, 3)),
                    kind: ParseDiagnosticKind::Recovery {
                        dropped_tokens: vec!["Plus".to_owned()],
                        cause: Box::new(ParseDiagnosticKind::UnrecognizedEof {
                            expected: vec!["Integer".to_owned()],
                        }),
                    },
                },
            ],
        };

        assert_eq!(diagnostics.recovery_count(), 1);
    }

    #[test]
    fn top_level_error_wraps_resource_and_eval_failures() {
        assert_eq!(
            Error::from(ResourceLimitError::TooManyTokens { actual: 2, max: 1 }),
            Error::ResourceLimit(ResourceLimitError::TooManyTokens { actual: 2, max: 1 })
        );
        assert_eq!(
            Error::from(EvalError::DivisionByZero),
            Error::Eval(EvalError::DivisionByZero)
        );
    }

    #[test]
    fn parse_accepts_valid_expression() {
        assert!(parse("x + 2").is_ok());
    }

    #[test]
    fn parse_reports_lexical_error() {
        assert!(matches!(parse("$"), Err(EvalError::LexicalError(_))));
    }

    #[test]
    fn parse_detailed_reports_lexical_span() {
        assert!(matches!(
            parse_detailed("$"),
            Err(Error::Lexical {
                span: SourceSpan { start: 0, end: 1 },
                error: LexicalError::UnknownSymbol(_),
            })
        ));
    }

    #[test]
    fn parse_detailed_reports_token_stream_lexical_error_without_span() {
        assert_eq!(
            parse_tokens_with_options_detailed(
                [Err(tokens::LexicalError::InvalidToken)],
                &EvalOptions::default(),
            ),
            Err(Error::Lexical {
                span: SourceSpan::new(0, 0),
                error: LexicalError::InvalidToken,
            })
        );
    }

    #[test]
    fn parse_reports_parser_recovery() {
        assert!(matches!(
            parse("1 +"),
            Err(EvalError::ParserRecovery { count: 1 })
        ));
    }

    #[test]
    fn parse_detailed_preserves_recovery_diagnostics() {
        let diagnostics = match parse_detailed("1 +") {
            Err(Error::Parse(diagnostics)) => diagnostics,
            other => panic!("expected parse diagnostics, got {other:?}"),
        };

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics.recovery_count(), 1);

        let diagnostic = &diagnostics.diagnostics[0];
        assert!(diagnostic.span.is_some());
        assert!(matches!(
            diagnostic.kind,
            ParseDiagnosticKind::Recovery { .. }
        ));
    }

    #[test]
    fn parse_with_options_enforces_resource_limits() {
        let options = EvalOptions {
            max_input_bytes: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options("1 + 2", &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::InputTooLarge { actual: 5, max: 1 }
            ))
        ));

        let options = EvalOptions {
            max_tokens: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options("1 + 2", &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyTokens { actual: 2, max: 1 }
            ))
        ));

        let options = EvalOptions {
            max_ast_nodes: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options("1 + 2", &options),
            Err(EvalError::ResourceLimit(ResourceLimitError::AstTooLarge {
                actual: 2,
                max: 1,
            }))
        ));

        let options = EvalOptions {
            max_depth: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options("!!true", &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::ExpressionTooDeep { actual: 2, max: 1 }
            ))
        ));

        let options = EvalOptions {
            max_function_args: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options("min(1, 2)", &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyFunctionArguments { actual: 2, max: 1 }
            ))
        ));

        let options = EvalOptions {
            max_parser_recoveries: 0,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options("1 +", &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::TooManyParserRecoveries { actual: 1, max: 0 }
            ))
        ));
    }

    #[test]
    fn parse_with_options_detailed_enforces_resource_limits() {
        let options = EvalOptions {
            max_input_bytes: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options_detailed("1 + 2", &options),
            Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge {
                actual: 5,
                max: 1,
            }))
        ));

        let options = EvalOptions {
            max_tokens: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options_detailed("1 + 2", &options),
            Err(Error::ResourceLimit(ResourceLimitError::TooManyTokens {
                actual: 2,
                max: 1,
            }))
        ));

        let options = EvalOptions {
            max_ast_nodes: 1,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options_detailed("1 + 2", &options),
            Err(Error::ResourceLimit(ResourceLimitError::AstTooLarge {
                actual: 2,
                max: 1,
            }))
        ));

        let options = EvalOptions {
            max_parser_recoveries: 0,
            ..EvalOptions::default()
        };
        assert!(matches!(
            parse_with_options_detailed("1 +", &options),
            Err(Error::ResourceLimit(
                ResourceLimitError::TooManyParserRecoveries { actual: 1, max: 0 }
            ))
        ));
    }

    #[test]
    fn parsed_expression_can_be_evaluated_with_different_maps() {
        let parsed = match parse("x + 1") {
            Ok(parsed) => parsed,
            Err(error) => panic!("unexpected parse error: {error:?}"),
        };

        let mut first = HashMap::new();
        first.insert("x".to_owned(), Value::Integer(1));

        let mut second = HashMap::new();
        second.insert("x".to_owned(), Value::Integer(41));

        assert_eq!(evaluate_parsed(&parsed, &first), Ok(Value::Integer(2)));
        assert_eq!(evaluate_parsed(&parsed, &second), Ok(Value::Integer(42)));
    }

    #[test]
    fn parsed_expression_can_be_evaluated_with_callback() {
        let parsed = match parse("x + 1") {
            Ok(parsed) => parsed,
            Err(error) => panic!("unexpected parse error: {error:?}"),
        };

        let result = evaluate_parsed(&parsed, |name: &str| match name {
            "x" => Ok(Value::Integer(41)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        });

        assert_eq!(result, Ok(Value::Integer(42)));
    }

    #[test]
    fn parsed_expression_can_be_evaluated_with_named_resolver() {
        struct RuntimeResolver;

        impl VariableResolver for RuntimeResolver {
            fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
                match name {
                    "x" => Ok(Value::Integer(40)),
                    other => Err(EvalError::UnknownVariable(other.to_owned())),
                }
            }
        }

        let parsed = match parse("x + 2") {
            Ok(parsed) => parsed,
            Err(error) => panic!("unexpected parse error: {error:?}"),
        };

        assert_eq!(
            evaluate_parsed(&parsed, RuntimeResolver),
            Ok(Value::Integer(42))
        );
    }

    #[test]
    fn evaluate_parsed_rejects_invalid_resolver_float() {
        let parsed = match parse("x") {
            Ok(parsed) => parsed,
            Err(error) => panic!("unexpected parse error: {error:?}"),
        };

        let result = evaluate_parsed(&parsed, |_name: &str| Ok(Value::Float(f64::INFINITY)));

        assert_eq!(result, Err(EvalError::NonFiniteFloat));
    }

    #[test]
    fn evaluate_parsed_detailed_rejects_invalid_resolver_float() {
        let parsed = match parse("x") {
            Ok(parsed) => parsed,
            Err(error) => panic!("unexpected parse error: {error:?}"),
        };

        let result =
            evaluate_parsed_detailed(&parsed, |_name: &str| Ok(Value::Float(f64::INFINITY)));

        assert_eq!(result, Err(Error::Eval(EvalError::NonFiniteFloat)));
    }

    #[test]
    fn evaluate_detailed_wraps_eval_errors() {
        let variables = HashMap::new();

        assert_eq!(
            evaluate_detailed("1 / 0", &variables),
            Err(Error::Eval(EvalError::DivisionByZero))
        );
    }

    #[test]
    fn evaluate_with_resolver_detailed_accepts_callbacks() {
        let result = evaluate_with_resolver_detailed("x + 2", |name: &str| match name {
            "x" => Ok(Value::Integer(40)),
            other => Err(EvalError::UnknownVariable(other.to_owned())),
        });

        assert_eq!(result, Ok(Value::Integer(42)));
    }

    #[test]
    fn evaluate_with_resolver_detailed_accepts_named_resolver_type() {
        struct RuntimeResolver;

        impl VariableResolver for RuntimeResolver {
            fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
                match name {
                    "x" => Ok(Value::Integer(40)),
                    other => Err(EvalError::UnknownVariable(other.to_owned())),
                }
            }
        }

        assert_eq!(
            evaluate_with_resolver_detailed("x + 2", RuntimeResolver),
            Ok(Value::Integer(42))
        );
    }

    #[test]
    fn evaluate_with_options_and_resolver_detailed_preserves_error_stage() {
        use std::cell::Cell;

        let called = Cell::new(false);
        let mut resolver = |_name: &str| {
            called.set(true);
            Ok(Value::Integer(0))
        };

        let options = EvalOptions {
            max_input_bytes: 1,
            ..EvalOptions::default()
        };

        assert_eq!(
            evaluate_with_options_and_resolver_detailed("1 + 2", &mut resolver, &options),
            Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge {
                actual: 5,
                max: 1,
            }))
        );
        assert!(!called.get());

        assert!(matches!(
            evaluate_with_options_and_resolver_detailed(
                "$",
                &mut resolver,
                &EvalOptions::default()
            ),
            Err(Error::Lexical {
                span: SourceSpan { start: 0, end: 1 },
                ..
            })
        ));
        assert!(!called.get());

        assert!(matches!(
            evaluate_with_options_and_resolver_detailed(
                "1 +",
                &mut resolver,
                &EvalOptions::default()
            ),
            Err(Error::Parse(_))
        ));
        assert!(!called.get());

        assert_eq!(
            evaluate_with_options_and_resolver_detailed(
                "x",
                &mut resolver,
                &EvalOptions::default()
            ),
            Ok(Value::Integer(0))
        );
        assert!(called.get());
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
        assert_eq!(
            evaluate_with_options("(1 + 2) + 3", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::ExpressionTooDeep { actual: 2, max: 1 }
            ))
        );
        assert_eq!(
            evaluate_with_options("1 + (2 + 3)", &variables, &options),
            Err(EvalError::ResourceLimit(
                ResourceLimitError::ExpressionTooDeep { actual: 2, max: 1 }
            ))
        );
        assert_eq!(
            evaluate_with_options("min((1 + 2), 3)", &variables, &options),
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
            max_function_args: 2,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("min(1, 2)", &variables, &options),
            Ok(Value::Integer(1))
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

        let options = EvalOptions {
            max_parser_recoveries: 1,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options("1 +", &variables, &options),
            Err(EvalError::ParserRecovery { count: 1 })
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
    fn evaluate_with_resolver_accepts_named_resolver_type() {
        use std::cell::Cell;
        use std::rc::Rc;

        struct CountingResolver {
            lookups: Rc<Cell<usize>>,
        }

        impl VariableResolver for CountingResolver {
            fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
                self.lookups.set(self.lookups.get() + 1);

                match name {
                    "x" => Ok(Value::Integer(20)),
                    other => Err(EvalError::UnknownVariable(other.to_owned())),
                }
            }
        }

        let lookups = Rc::new(Cell::new(0));
        let resolver = CountingResolver {
            lookups: Rc::clone(&lookups),
        };

        assert_eq!(
            evaluate_with_resolver("x + x", resolver),
            Ok(Value::Integer(40))
        );
        assert_eq!(lookups.get(), 2);
    }

    #[test]
    fn evaluate_with_resolver_rejects_invalid_float_values_without_panicking() {
        struct StaticFloatResolver {
            value: f64,
        }

        impl VariableResolver for StaticFloatResolver {
            fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
                match name {
                    "x" => Ok(Value::Float(self.value)),
                    other => Err(EvalError::UnknownVariable(other.to_owned())),
                }
            }
        }

        let cases = [
            (f64::INFINITY, EvalError::NonFiniteFloat),
            (f64::NEG_INFINITY, EvalError::NonFiniteFloat),
            (f64::NAN, EvalError::NonFiniteFloat),
            (f64::MIN_POSITIVE / 2.0, EvalError::SubnormalFloat),
        ];

        for (value, expected) in cases {
            let result = std::panic::catch_unwind(|| {
                evaluate_with_resolver("x", StaticFloatResolver { value })
            });

            match result {
                Ok(actual) => assert_eq!(actual, Err(expected)),
                Err(_) => panic!("{value:?} panicked"),
            }
        }
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

            match result {
                Ok(actual) => assert_eq!(actual, Err(expected)),
                Err(_) => panic!("{value:?} panicked"),
            }
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

        let options = EvalOptions {
            max_input_bytes: 5,
            ..EvalOptions::default()
        };
        assert_eq!(
            evaluate_with_options_and_resolver("1 + 2", &mut resolver, &options),
            Ok(Value::Integer(3))
        );
        assert!(!called.get());

        assert_eq!(
            evaluate_with_options_and_resolver("x", &mut resolver, &EvalOptions::default()),
            Ok(Value::Integer(0))
        );
        assert!(called.get());
    }

    #[test]
    fn evaluate_with_options_and_resolver_accepts_named_resolver_type() {
        struct RuntimeResolver;

        impl VariableResolver for RuntimeResolver {
            fn resolve(&mut self, name: &str) -> Result<Value, EvalError> {
                match name {
                    "x" => Ok(Value::Integer(40)),
                    other => Err(EvalError::UnknownVariable(other.to_owned())),
                }
            }
        }

        let options = EvalOptions {
            max_tokens: 3,
            ..EvalOptions::default()
        };

        assert_eq!(
            evaluate_with_options_and_resolver("x + 2", RuntimeResolver, &options),
            Ok(Value::Integer(42))
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
        fn prop_parse_then_eval_matches_evaluate(
            name in variable_name(),
            value in -1_000_000i64..1_000_000,
            addend in -1_000_000i64..1_000_000,
        ) {
            let expression = format!("{name} + {addend}");
            let mut variables = HashMap::new();
            variables.insert(name.clone(), Value::Integer(value));

            let parsed = parse(&expression)
                .map_err(|error| TestCaseError::fail(format!("parse failed: {error:?}")))?;
            let parse_then_eval = evaluate_parsed(&parsed, &variables);
            let direct = evaluate(&expression, &variables);

            prop_assert_eq!(parse_then_eval, direct);
        }

        #[test]
        fn prop_min_matches_rust_min(values in prop::collection::vec(-1_000i64..1_000, 1..20)) {
            let arguments = values
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let expression = format!("min({arguments})");
            let expected = values
                .iter()
                .copied()
                .fold(values[0], i64::min);

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
            let expected = values
                .iter()
                .copied()
                .fold(values[0], i64::max);

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
