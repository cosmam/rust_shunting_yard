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
}
