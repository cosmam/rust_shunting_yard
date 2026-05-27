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
#[derive(Clone, Debug, PartialEq)]
pub enum EvalError {
    /// An operator or function was used with an unsupported number of operands.
    InvalidArity,
    /// The expression tree contains an error node or lexical error node.
    InvalidExpression,
    /// An invalid type was passed to a calculation
    InvalidType(String),
    /// There is some math error, such as division by zero
    MathError(String),
    /// The parser ran into an error it couldn't recover from
    ParserError,
    /// An opcode was found that was already supposed to be filtered out
    UnexpectedOpcode,
    /// A variable reference could not be found in the provided bindings.
    UnknownVariable(String),
}

pub fn evaluate(text: &str, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    let lexer = lexer::Lexer::new(text);
    let parser = calc::ExpressionParser::new();

    let mut errors = Vec::new();
    let result = parser.parse(&mut errors, lexer);

    match result {
        Ok(ast) => eval::eval(&ast, variables),
        Err(_) => Err(EvalError::ParserError),
    }
}
