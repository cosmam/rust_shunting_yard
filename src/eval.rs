//! Evaluation for parsed expressions.
//!
//! # Overview
//!
//! This module evaluates an [`Expression`] tree into a runtime [`Value`]. Literal
//! values evaluate directly, variables are resolved from a caller-provided
//! binding map, and compound expressions are evaluated recursively before their
//! operators or functions are applied.
//!
//! # Errors
//!
//! Evaluation returns [`EvalError`] when an expression cannot be evaluated, a
//! referenced variable is missing, or an operator is used with the wrong arity.

use crate::ast::{Expression, Func, Opcode};
use std::collections::HashMap;

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
    /// An opcode was found that was already supposed to be filtered out
    UnexpectedOpcode,
    /// A variable reference could not be found in the provided bindings.
    UnknownVariable(String),
}

/// Evaluate an expression into a runtime value.
///
/// `expr` is evaluated recursively. Literal expression nodes become their
/// corresponding [`Value`] variants, variable nodes are looked up in
/// `variables`, and compound nodes delegate to the appropriate unary, binary, or
/// function evaluator after their child expressions have been evaluated.
///
/// # Errors
///
/// Returns [`EvalError::UnknownVariable`] when `expr` references a variable that
/// is not present in `variables`.
///
/// Returns [`EvalError::InvalidExpression`] when `expr` contains an
/// [`Expression::Error`] or [`Expression::LexicalError`] node.
///
/// Returns [`EvalError::InvalidArity`] when a unary or binary operator is used
/// in a position where that operator is not supported.
pub fn eval(expr: &Expression, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    match expr {
        Expression::Bool(n) => Ok(Value::Bool(*n)),
        Expression::Integer(n) => Ok(Value::Integer(*n)),
        Expression::Float(n) => Ok(Value::Float(*n)),

        Expression::UnaryOperation { operator, value } => {
            let value = eval(value, variables)?;
            apply_unary(operator, value)
        }

        Expression::BinaryOperation { lhs, operator, rhs } => {
            let left = eval(lhs, variables)?;
            let right = eval(rhs, variables)?;
            apply_binary(operator, left, right)
        }

        Expression::Function { func, arguments } => {
            let values = arguments
                .iter()
                .map(|v| eval(v, variables))
                .collect::<Result<Vec<_>, _>>()?;

            apply_function(func, values)
        }

        Expression::Variable(name) => {
            if let Some(value) = variables.get(*name) {
                Ok(value.clone())
            } else {
                Err(EvalError::UnknownVariable(name.to_string()))
            }
        }

        Expression::Error | Expression::LexicalError(_) => Err(EvalError::InvalidExpression),
    }
}

/************** Binary operations **************/

/// Apply a binary operator to two evaluated values.
///
/// # Parameters
///
/// - `op`: TODO: Document the operator being applied.
/// - `lhs`: TODO: Document the left-hand value.
/// - `rhs`: TODO: Document the right-hand value.
///
/// # Errors
///
/// TODO: Document invalid arity and operand errors.
fn apply_binary(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    match op {
        Opcode::Equals
        | Opcode::NotEquals
        | Opcode::LessThan
        | Opcode::GreaterThan
        | Opcode::GreaterThanEquals
        | Opcode::LessThanEquals
        | Opcode::ApproximatelyEquals => apply_binary_comparison(op, lhs, rhs),
        Opcode::Power
        | Opcode::Multiply
        | Opcode::Divide
        | Opcode::Plus
        | Opcode::Minus
        | Opcode::Modulo => apply_binary_arithmatic(op, lhs, rhs),
        Opcode::BitwiseAnd | Opcode::BitwiseOr | Opcode::BitwiseXor => {
            apply_binary_bit_operation(op, lhs, rhs)
        }
        Opcode::BitshiftLeft | Opcode::BitshiftRight => apply_bitshift_operation(op, lhs, rhs),
        Opcode::LogicalAnd | Opcode::LogicalOr => apply_binary_logical_operation(op, lhs, rhs),
        Opcode::Degrees | Opcode::BitwiseNot | Opcode::LogicalNot => Err(EvalError::InvalidArity),
    }
}

/// Apply a binary comparison operator.
///
/// # Parameters
///
/// - `op`: TODO: Document supported comparison operators.
/// - `lhs`: TODO: Document the left-hand value.
/// - `rhs`: TODO: Document the right-hand value.
///
/// # Errors
///
/// TODO: Document comparison-specific error cases.
fn apply_binary_comparison(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

/// Apply a binary arithmetic operator.
///
/// # Parameters
///
/// - `op`: TODO: Document supported arithmetic operators.
/// - `lhs`: TODO: Document the left-hand value.
/// - `rhs`: TODO: Document the right-hand value.
///
/// # Errors
///
/// TODO: Document arithmetic-specific error cases.
fn apply_binary_arithmatic(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

/// Apply a binary bitwise operator.
///
/// # Parameters
///
/// - `op`: TODO: Document supported bitwise operators.
/// - `lhs`: TODO: Document the left-hand value.
/// - `rhs`: TODO: Document the right-hand value.
///
/// # Errors
///
/// TODO: Document bitwise-specific error cases.
fn apply_binary_bit_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    match (op, lhs, rhs) {
        (Opcode::Equals, _, _)
        | (Opcode::NotEquals, _, _)
        | (Opcode::LessThanEquals, _, _)
        | (Opcode::GreaterThanEquals, _, _)
        | (Opcode::ApproximatelyEquals, _, _)
        | (Opcode::LessThan, _, _)
        | (Opcode::GreaterThan, _, _)
        | (Opcode::Power, _, _)
        | (Opcode::Multiply, _, _)
        | (Opcode::Divide, _, _)
        | (Opcode::Plus, _, _)
        | (Opcode::Minus, _, _)
        | (Opcode::Modulo, _, _)
        | (Opcode::BitshiftLeft, _, _)
        | (Opcode::BitshiftRight, _, _)
        | (Opcode::LogicalAnd, _, _)
        | (Opcode::LogicalOr, _, _)
        | (Opcode::LogicalNot, _, _)
        | (Opcode::BitwiseNot, _, _)
        | (Opcode::Degrees, _, _) => Err(EvalError::UnexpectedOpcode),
        (_, Value::Float(_), _) | (_, _, Value::Float(_)) => Err(EvalError::InvalidType(
            "Bitwise operations on floats not supported".to_string(),
        )),
        (Opcode::BitwiseAnd, Value::Bool(b_lhs), Value::Bool(b_rhs)) => {
            Ok(Value::Bool(b_lhs & b_rhs))
        }
        (Opcode::BitwiseOr, Value::Bool(b_lhs), Value::Bool(b_rhs)) => {
            Ok(Value::Bool(b_lhs | b_rhs))
        }
        (Opcode::BitwiseXor, Value::Bool(b_lhs), Value::Bool(b_rhs)) => {
            Ok(Value::Bool(b_lhs ^ b_rhs))
        }
        (Opcode::BitwiseAnd, Value::Integer(i_lhs), Value::Integer(i_rhs)) => {
            Ok(Value::Integer(i_lhs & i_rhs))
        }
        (Opcode::BitwiseOr, Value::Integer(i_lhs), Value::Integer(i_rhs)) => {
            Ok(Value::Integer(i_lhs | i_rhs))
        }
        (Opcode::BitwiseXor, Value::Integer(i_lhs), Value::Integer(i_rhs)) => {
            Ok(Value::Integer(i_lhs ^ i_rhs))
        }
        (Opcode::BitwiseAnd, _, _) | (Opcode::BitwiseOr, _, _) | (Opcode::BitwiseXor, _, _) => Err(
            EvalError::InvalidType("Cannot mix types for bitwise operations".to_string()),
        ),
    }
}

/// Apply a bitshift operator.
///
/// # Parameters
///
/// - `op`: TODO: Document supported bitshift operators.
/// - `lhs`: TODO: Document the value being shifted.
/// - `rhs`: TODO: Document the shift amount.
///
/// # Errors
///
/// TODO: Document bitshift-specific error cases.
fn apply_bitshift_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    if let (Value::Integer(l), Value::Integer(r)) = (lhs, rhs) {
        match op {
            Opcode::BitshiftLeft => Ok(Value::Integer(l << r)),
            Opcode::BitshiftRight => Ok(Value::Integer(l >> r)),
            _ => Err(EvalError::UnexpectedOpcode),
        }
    } else {
        Err(EvalError::InvalidType(
            "Logical operations must operate on bools".to_string(),
        ))
    }
}

/// Apply a binary logical operator.
///
/// # Parameters
///
/// - `op`: TODO: Document supported logical operators.
/// - `lhs`: TODO: Document the left-hand value.
/// - `rhs`: TODO: Document the right-hand value.
///
/// # Errors
///
/// TODO: Document logical-operation error cases.
fn apply_binary_logical_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    if let (Value::Bool(l), Value::Bool(r)) = (lhs, rhs) {
        match op {
            Opcode::LogicalAnd => Ok(Value::Bool(l && r)),
            Opcode::LogicalOr => Ok(Value::Bool(l || r)),
            _ => Err(EvalError::UnexpectedOpcode),
        }
    } else {
        Err(EvalError::InvalidType(
            "Logical operations must operate on bools".to_string(),
        ))
    }
}

/************** Unary operations **************/

/// Apply a unary operator to one evaluated value.
///
/// # Parameters
///
/// - `op`: TODO: Document the operator being applied.
/// - `val`: TODO: Document the operand value.
///
/// # Errors
///
/// TODO: Document invalid arity and operand errors.
fn apply_unary(op: &Opcode, val: Value) -> Result<Value, EvalError> {
    match op {
        Opcode::Degrees | Opcode::Plus | Opcode::Minus => apply_unary_arithmatic(op, val),
        Opcode::BitwiseNot => apply_bitwise_not(val),
        Opcode::LogicalNot => apply_logical_not(val),
        Opcode::Equals
        | Opcode::NotEquals
        | Opcode::LessThan
        | Opcode::GreaterThan
        | Opcode::GreaterThanEquals
        | Opcode::LessThanEquals
        | Opcode::ApproximatelyEquals
        | Opcode::Power
        | Opcode::Multiply
        | Opcode::Divide
        | Opcode::Modulo
        | Opcode::BitshiftLeft
        | Opcode::BitshiftRight
        | Opcode::BitwiseAnd
        | Opcode::BitwiseOr
        | Opcode::BitwiseXor
        | Opcode::LogicalAnd
        | Opcode::LogicalOr => Err(EvalError::InvalidArity),
    }
}

/// Apply a unary arithmetic operator.
///
/// # Parameters
///
/// - `op`: TODO: Document supported unary arithmetic operators.
/// - `val`: TODO: Document the operand value.
///
/// # Errors
///
/// TODO: Document unary arithmetic error cases.
fn apply_unary_arithmatic(op: &Opcode, val: Value) -> Result<Value, EvalError> {
    match (op, val.clone()) {
        (_, Value::Bool(_)) => Err(EvalError::InvalidType(
            "Unary arithmatic operations not defined for bool".to_string(),
        )),
        (Opcode::Plus, _) => Ok(val),
        (Opcode::Minus, Value::Integer(i)) => Ok(Value::Integer(-i)),
        (Opcode::Minus, Value::Float(f)) => Ok(Value::Float(-f)),
        (Opcode::Degrees, Value::Integer(i)) => Ok(Value::Float((i as f64).to_radians())),
        (Opcode::Degrees, Value::Float(f)) => Ok(Value::Float(f.to_radians())),
        (_, _) => Err(EvalError::UnexpectedOpcode),
    }
}

/// Apply bitwise negation to a value.
///
/// # Parameters
///
/// - `val`: TODO: Document the operand value.
///
/// # Errors
///
/// TODO: Document bitwise-negation error cases.
fn apply_bitwise_not(val: Value) -> Result<Value, EvalError> {
    match val {
        // rust guarantees bools are only 0 or 1, so BitwiseNot is the same as LogicalNot
        Value::Bool(v) => Ok(Value::Bool(!v)),
        // the '!' operator in rust for ints represents bitwise negation
        Value::Integer(i) => Ok(Value::Integer(!i)),
        Value::Float(_) => Err(EvalError::InvalidType(
            "Bitwise operations not defined for floats".to_string(),
        )),
    }
}

/// Apply logical negation to a value.
///
/// # Parameters
///
/// - `val`: TODO: Document the operand value.
///
/// # Errors
///
/// TODO: Document logical-negation error cases.
fn apply_logical_not(val: Value) -> Result<Value, EvalError> {
    match val {
        Value::Bool(v) => Ok(Value::Bool(!v)),
        Value::Integer(_) | Value::Float(_) => Err(EvalError::InvalidType(
            "Logical operations must be bools".to_string(),
        )),
    }
}

/************** Functions operations **************/

/// Apply a built-in function to evaluated argument values.
///
/// # Parameters
///
/// - `func`: TODO: Document the function being applied.
/// - `vals`: TODO: Document the evaluated argument values.
///
/// # Errors
///
/// TODO: Document function-specific error cases.
fn apply_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

// Some fo the tests here are defensive programming; the AST will not
// come out with a binary operator in a unary operation. But if that ever
// changes in the future, as a whole or for a particular operator, this
// will result in failing tests, which is what we want
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    // in general, we'll test from the public eval function. The exceptions
    // are redundant/defensive error handling, which we're testing as a way
    // to catch regressions/changed assumptions

    /************ Unary operation tests *************/

    #[test]
    fn test_logical_not_bool() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::LogicalNot,
            value: Box::new(Expression::Bool(true)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Bool(false)));
    }

    #[rstest]
    #[case(Expression::Integer(1))]
    #[case(Expression::Float(1.0))]
    fn test_logical_not_invalid(#[case] val: Expression) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::LogicalNot,
            value: Box::new(val),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Logical operations must be bools".to_string(),
            ))
        );
    }

    #[test]
    fn test_bitwise_not_bool() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::BitwiseNot,
            value: Box::new(Expression::Bool(true)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Bool(false)));
    }

    #[test]
    fn test_bitwise_not_int() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::BitwiseNot,
            value: Box::new(Expression::Integer(467)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Integer(-468)));
    }

    #[test]
    fn test_bitwise_not_float() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::BitwiseNot,
            value: Box::new(Expression::Float(1.0)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Bitwise operations not defined for floats".to_string(),
            ))
        );
    }

    #[rstest]
    #[case(Opcode::Degrees)]
    #[case(Opcode::Plus)]
    #[case(Opcode::Minus)]
    fn test_unary_arithmatic_bool(#[case] op: Opcode) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: op,
            value: Box::new(Expression::Bool(true)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Unary arithmatic operations not defined for bool".to_string(),
            ))
        );
    }

    #[rstest]
    #[case(Opcode::Equals)]
    #[case(Opcode::NotEquals)]
    #[case(Opcode::LessThanEquals)]
    #[case(Opcode::GreaterThanEquals)]
    #[case(Opcode::ApproximatelyEquals)]
    #[case(Opcode::LessThan)]
    #[case(Opcode::GreaterThan)]
    #[case(Opcode::Power)]
    #[case(Opcode::Multiply)]
    #[case(Opcode::Divide)]
    #[case(Opcode::Modulo)]
    #[case(Opcode::BitshiftLeft)]
    #[case(Opcode::BitshiftRight)]
    #[case(Opcode::LogicalAnd)]
    #[case(Opcode::LogicalOr)]
    #[case(Opcode::LogicalNot)]
    #[case(Opcode::BitwiseNot)]
    #[case(Opcode::BitwiseAnd)]
    #[case(Opcode::BitwiseOr)]
    #[case(Opcode::BitwiseXor)]
    fn test_unary_arithmatic_invalid_opcode(#[case] op: Opcode) {
        let result = apply_unary_arithmatic(&op, Value::Integer(1));

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[test]
    fn test_unary_plus_int() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Plus,
            value: Box::new(Expression::Integer(3)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Integer(3)));
    }

    #[test]
    fn test_unary_minus_int() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Minus,
            value: Box::new(Expression::Integer(3)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Integer(-3)));
    }

    #[test]
    fn test_unary_degrees_int() {
        let v: i64 = 3;
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Degrees,
            value: Box::new(Expression::Integer(v)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Float((v as f64).to_radians())));
    }

    #[test]
    fn test_unary_plus_float() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Plus,
            value: Box::new(Expression::Float(3.7)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Float(3.7)));
    }

    #[test]
    fn test_unary_minus_float() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Minus,
            value: Box::new(Expression::Float(3.7)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Float(-3.7)));
    }

    #[test]
    fn test_unary_degrees_float() {
        let v: f64 = 52.0;
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Degrees,
            value: Box::new(Expression::Float(v)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Float(v.to_radians())));
    }

    #[rstest]
    #[case(Opcode::Equals)]
    #[case(Opcode::NotEquals)]
    #[case(Opcode::LessThanEquals)]
    #[case(Opcode::GreaterThanEquals)]
    #[case(Opcode::ApproximatelyEquals)]
    #[case(Opcode::LessThan)]
    #[case(Opcode::GreaterThan)]
    #[case(Opcode::Power)]
    #[case(Opcode::Multiply)]
    #[case(Opcode::Divide)]
    #[case(Opcode::Modulo)]
    #[case(Opcode::BitshiftLeft)]
    #[case(Opcode::BitshiftRight)]
    #[case(Opcode::LogicalAnd)]
    #[case(Opcode::LogicalOr)]
    #[case(Opcode::BitwiseAnd)]
    #[case(Opcode::BitwiseOr)]
    #[case(Opcode::BitwiseXor)]
    fn test_apply_unary_invalid_opcode(#[case] op: Opcode) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: op,
            value: Box::new(Expression::Integer(1)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(EvalError::InvalidArity));
    }

    #[test]
    fn test_unary_eval_variable_unknown() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: Opcode::Degrees,
            value: Box::new(Expression::Variable("Test_Name")),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::UnknownVariable("Test_Name".to_string()))
        );
    }

    /************ Binary operation tests *************/

    #[rstest]
    #[case(Opcode::LogicalAnd, true, true, true)]
    #[case(Opcode::LogicalAnd, true, false, false)]
    #[case(Opcode::LogicalAnd, false, false, false)]
    #[case(Opcode::LogicalOr, true, true, true)]
    #[case(Opcode::LogicalOr, true, false, true)]
    #[case(Opcode::LogicalOr, false, false, false)]
    fn test_binary_boolean_algebra_valid(
        #[case] op: Opcode,
        #[case] lhs: bool,
        #[case] rhs: bool,
        #[case] expected: bool,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Bool(lhs)),
            operator: op,
            rhs: Box::new(Expression::Bool(rhs)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Bool(expected)));
    }

    #[rstest]
    #[case(Expression::Integer(1), Expression::Bool(true))]
    #[case(Expression::Bool(true), Expression::Integer(1))]
    #[case(Expression::Integer(1), Expression::Integer(1))]
    fn test_binary_boolean_algebra_invalid_types(#[case] lhs: Expression, #[case] rhs: Expression) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::LogicalOr,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Logical operations must operate on bools".to_string()
            ))
        );
    }

    #[rstest]
    #[case(Opcode::Equals)]
    #[case(Opcode::NotEquals)]
    #[case(Opcode::LessThanEquals)]
    #[case(Opcode::GreaterThanEquals)]
    #[case(Opcode::ApproximatelyEquals)]
    #[case(Opcode::LessThan)]
    #[case(Opcode::GreaterThan)]
    #[case(Opcode::Power)]
    #[case(Opcode::Multiply)]
    #[case(Opcode::Divide)]
    #[case(Opcode::Plus)]
    #[case(Opcode::Minus)]
    #[case(Opcode::Modulo)]
    #[case(Opcode::BitshiftLeft)]
    #[case(Opcode::BitshiftRight)]
    #[case(Opcode::LogicalNot)]
    #[case(Opcode::BitwiseNot)]
    #[case(Opcode::BitwiseAnd)]
    #[case(Opcode::BitwiseOr)]
    #[case(Opcode::BitwiseXor)]
    #[case(Opcode::Degrees)]
    fn test_apply_binary_logical_operation_invalid_opcode(#[case] op: Opcode) {
        let result = apply_binary_logical_operation(&op, Value::Bool(true), Value::Bool(true));

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[test]
    fn test_binary_eval_variable_unknown_lhs() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Variable("Test_Name")),
            operator: Opcode::LogicalOr,
            rhs: Box::new(Expression::Bool(true)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::UnknownVariable("Test_Name".to_string()))
        );
    }

    #[test]
    fn test_binary_eval_variable_unknown_rhs() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Bool(true)),
            operator: Opcode::LogicalOr,
            rhs: Box::new(Expression::Variable("Test_Name")),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::UnknownVariable("Test_Name".to_string()))
        );
    }

    #[rstest]
    #[case(Opcode::BitshiftLeft, 8055371489994718882, 11, 6011609612845125632)]
    #[case(Opcode::BitshiftLeft, -1821376069820021562, 26, 8453234592348897280)]
    #[case(Opcode::BitshiftLeft, 3897635188866812215, 28, -2591961689800835072)]
    #[case(Opcode::BitshiftLeft, -7693944058662696389, 7, -7147403602218902144)]
    #[case(Opcode::BitshiftRight, 7629495294638887680, 11, 3725339499335394)]
    #[case(Opcode::BitshiftRight, -5773960239512220022, 26, -86038712256)]
    #[case(Opcode::BitshiftRight, 2841882122645057328, 28, 10586835900)]
    #[case(Opcode::BitshiftRight, -3171532055615339402, 7, -24777594184494840)]
    fn test_binary_bitshift_valid(
        #[case] op: Opcode,
        #[case] lhs: i64,
        #[case] rhs: i64,
        #[case] expected: i64,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Integer(lhs)),
            operator: op,
            rhs: Box::new(Expression::Integer(rhs)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Integer(expected)));
    }

    #[rstest]
    #[case(Expression::Integer(1), Expression::Float(1.0))]
    #[case(Expression::Bool(true), Expression::Integer(1))]
    #[case(Expression::Bool(true), Expression::Float(1.0))]
    fn test_binary_bitshift_invalid_types(#[case] lhs: Expression, #[case] rhs: Expression) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::BitshiftLeft,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Logical operations must operate on bools".to_string()
            ))
        );
    }

    #[rstest]
    #[case(Opcode::BitwiseAnd, true, true, true)]
    #[case(Opcode::BitwiseAnd, true, false, false)]
    #[case(Opcode::BitwiseAnd, false, false, false)]
    #[case(Opcode::BitwiseOr, true, true, true)]
    #[case(Opcode::BitwiseOr, true, false, true)]
    #[case(Opcode::BitwiseOr, false, false, false)]
    #[case(Opcode::BitwiseXor, true, true, false)]
    #[case(Opcode::BitwiseXor, true, false, true)]
    #[case(Opcode::BitwiseXor, false, false, false)]
    fn test_binary_bit_operations_bool(
        #[case] op: Opcode,
        #[case] lhs: bool,
        #[case] rhs: bool,
        #[case] expected: bool,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Bool(lhs)),
            operator: op,
            rhs: Box::new(Expression::Bool(rhs)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Bool(expected)));
    }

    #[rstest]
    #[case(Opcode::BitwiseAnd, 54, 19, 18)]
    #[case(Opcode::BitwiseAnd, 54, 145, 16)]
    #[case(Opcode::BitwiseAnd, 108, 19, 0)]
    #[case(Opcode::BitwiseAnd, 108, 145, 0)]
    #[case(Opcode::BitwiseOr, 54, 19, 55)]
    #[case(Opcode::BitwiseOr, 54, 145, 183)]
    #[case(Opcode::BitwiseOr, 108, 19, 127)]
    #[case(Opcode::BitwiseOr, 108, 145, 253)]
    #[case(Opcode::BitwiseXor, 54, 19, 37)]
    #[case(Opcode::BitwiseXor, 54, 145, 167)]
    #[case(Opcode::BitwiseXor, 108, 19, 127)]
    #[case(Opcode::BitwiseXor, 108, 145, 253)]
    fn test_binary_bit_operations_int(
        #[case] op: Opcode,
        #[case] lhs: i64,
        #[case] rhs: i64,
        #[case] expected: i64,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Integer(lhs)),
            operator: op,
            rhs: Box::new(Expression::Integer(rhs)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Integer(expected)));
    }

    #[rstest]
    #[case(Expression::Integer(1), Expression::Float(1.0))]
    #[case(Expression::Float(1.0), Expression::Integer(1))]
    #[case(Expression::Float(1.0), Expression::Float(1.0))]
    fn test_apply_binary_bitshift_operation_invalid_float(
        #[case] lhs: Expression,
        #[case] rhs: Expression,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::BitwiseAnd,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Bitwise operations on floats not supported".to_string()
            ))
        );
    }

    #[rstest]
    #[case(Expression::Integer(1), Expression::Bool(true))]
    #[case(Expression::Bool(true), Expression::Integer(1))]
    fn test_apply_binary_bitshift_operation_invalid_mixed_types(
        #[case] lhs: Expression,
        #[case] rhs: Expression,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::BitwiseAnd,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Cannot mix types for bitwise operations".to_string()
            ))
        );
    }

    #[rstest]
    #[case(Opcode::Equals)]
    #[case(Opcode::NotEquals)]
    #[case(Opcode::LessThanEquals)]
    #[case(Opcode::GreaterThanEquals)]
    #[case(Opcode::ApproximatelyEquals)]
    #[case(Opcode::LessThan)]
    #[case(Opcode::GreaterThan)]
    #[case(Opcode::Power)]
    #[case(Opcode::Multiply)]
    #[case(Opcode::Divide)]
    #[case(Opcode::Plus)]
    #[case(Opcode::Minus)]
    #[case(Opcode::Modulo)]
    #[case(Opcode::LogicalNot)]
    #[case(Opcode::BitwiseNot)]
    #[case(Opcode::BitwiseAnd)]
    #[case(Opcode::BitwiseOr)]
    #[case(Opcode::BitwiseXor)]
    #[case(Opcode::Degrees)]
    fn test_apply_binary_bitshift_operation_invalid_opcode(#[case] op: Opcode) {
        let result = apply_bitshift_operation(&op, Value::Integer(1), Value::Integer(1));

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[rstest]
    #[case(Opcode::Equals)]
    #[case(Opcode::NotEquals)]
    #[case(Opcode::LessThanEquals)]
    #[case(Opcode::GreaterThanEquals)]
    #[case(Opcode::ApproximatelyEquals)]
    #[case(Opcode::LessThan)]
    #[case(Opcode::GreaterThan)]
    #[case(Opcode::Power)]
    #[case(Opcode::Multiply)]
    #[case(Opcode::Divide)]
    #[case(Opcode::Plus)]
    #[case(Opcode::Minus)]
    #[case(Opcode::Modulo)]
    #[case(Opcode::BitshiftLeft)]
    #[case(Opcode::BitshiftRight)]
    #[case(Opcode::LogicalAnd)]
    #[case(Opcode::LogicalOr)]
    #[case(Opcode::LogicalNot)]
    #[case(Opcode::BitwiseNot)]
    #[case(Opcode::Degrees)]
    fn test_apply_binary_bit_operation_invalid_opcode(#[case] op: Opcode) {
        let result = apply_binary_bit_operation(&op, Value::Integer(1), Value::Integer(1));

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }
}
