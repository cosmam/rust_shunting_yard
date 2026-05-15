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
use crate::{EvalError, Value};
use std::collections::HashMap;

const EPSILON: f64 = 0.000001;

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
/// in a position where that operator is not supported. The grammar should prevent
/// this, so this is mostly a defensive programming decision
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
/// Dispatches comparison, arithmetic, bitwise, bitshift, and boolean operators
/// to the helper that implements that operator family.
///
/// # Errors
///
/// Returns [`EvalError::InvalidArity`] when a unary-only opcode is supplied.
/// Errors from the selected operator-family helper are returned unchanged.
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
        | Opcode::Modulo => apply_binary_math_operation(op, lhs, rhs),
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
/// - `op`: Equality or ordering comparison operator.
/// - `lhs`: The evaluated left-hand value.
/// - `rhs`: The evaluated right-hand value.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] when `op` is not a supported
/// comparison operator. Returns [`EvalError::InvalidType`] when the operands
/// are not the same type after integer/float promotion.
#[rustfmt::skip]
fn apply_binary_comparison(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    match convert_binary_values(op, lhs, rhs) {
        (Opcode::Power, _, _)
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
        | (Opcode::BitwiseAnd, _, _)
        | (Opcode::BitwiseOr, _, _)
        | (Opcode::BitwiseXor, _, _)
        | (Opcode::Degrees, _, _) => Err(EvalError::UnexpectedOpcode),
        (Opcode::Equals, Value::Bool(l), Value::Bool(r)) |
        (Opcode::ApproximatelyEquals, Value::Bool(l), Value::Bool(r)) => {
            Ok(Value::Bool(l == r))
        }
        (Opcode::NotEquals, Value::Bool(l), Value::Bool(r)) => {
            Ok(Value::Bool(l != r))
        }
        (Opcode::GreaterThan, Value::Bool(l), Value::Bool(r)) => {
            Ok(Value::Bool(l & !r))
        }
        (Opcode::GreaterThanEquals, Value::Bool(l), Value::Bool(r)) => {
            Ok(Value::Bool(l >= r))
        }
        (Opcode::LessThan, Value::Bool(l), Value::Bool(r)) => {
            Ok(Value::Bool(!l & r))
        }
        (Opcode::LessThanEquals, Value::Bool(l), Value::Bool(r)) => {
            Ok(Value::Bool(l <= r))
        }
        (Opcode::Equals, Value::Integer(l), Value::Integer(r)) |
        (Opcode::ApproximatelyEquals, Value::Integer(l), Value::Integer(r)) => {
            Ok(Value::Bool(l == r))
        }
        (Opcode::NotEquals, Value::Integer(l), Value::Integer(r)) => {
            Ok(Value::Bool(l != r))
        }
        (Opcode::GreaterThan, Value::Integer(l), Value::Integer(r)) => {
            Ok(Value::Bool(l > r))
        }
        (Opcode::GreaterThanEquals, Value::Integer(l), Value::Integer(r)) => {
            Ok(Value::Bool(l >= r))
        }
        (Opcode::LessThan, Value::Integer(l), Value::Integer(r)) => {
            Ok(Value::Bool(l < r))
        }
        (Opcode::LessThanEquals, Value::Integer(l), Value::Integer(r)) => {
            Ok(Value::Bool(l <= r))
        }
        (Opcode::Equals, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool(l == r))
        }
        (Opcode::NotEquals, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool(l != r))
        }
        (Opcode::GreaterThan, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool(l > r))
        }
        (Opcode::GreaterThanEquals, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool(l >= r))
        }
        (Opcode::LessThan, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool(l < r))
        }
        (Opcode::LessThanEquals, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool(l <= r))
        }
        (Opcode::ApproximatelyEquals, Value::Float(l), Value::Float(r)) => {
            Ok(Value::Bool((l - r).abs() < (l.max(r) * EPSILON)))
        }
        _ => Err(EvalError::InvalidType(
            "Cannot mix types for binary comparison".to_string(),
        )),
    }
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
fn apply_binary_math_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    // TODO: Handle overflow/range errors for all binary math operations, not just integer power.
    match convert_binary_values(op, lhs, rhs) {
        (Opcode::Equals, _, _)
        | (Opcode::NotEquals, _, _)
        | (Opcode::LessThanEquals, _, _)
        | (Opcode::GreaterThanEquals, _, _)
        | (Opcode::ApproximatelyEquals, _, _)
        | (Opcode::LessThan, _, _)
        | (Opcode::GreaterThan, _, _)
        | (Opcode::BitshiftLeft, _, _)
        | (Opcode::BitshiftRight, _, _)
        | (Opcode::LogicalAnd, _, _)
        | (Opcode::LogicalOr, _, _)
        | (Opcode::LogicalNot, _, _)
        | (Opcode::BitwiseNot, _, _)
        | (Opcode::BitwiseAnd, _, _)
        | (Opcode::BitwiseOr, _, _)
        | (Opcode::BitwiseXor, _, _)
        | (Opcode::Degrees, _, _) => Err(EvalError::UnexpectedOpcode),
        (Opcode::Power, Value::Integer(l), Value::Integer(r)) => {
            let result: Result<u32, _> = r.try_into();
            match result {
                Ok(val) => match l.checked_pow(val) {
                    Some(v) => Ok(Value::Integer(v)),
                    None => Err(EvalError::MathError(
                        "Integer overflow on power".to_string(),
                    )),
                },
                Err(_) => Err(EvalError::MathError(
                    "Integer exponent too large".to_string(),
                )),
            }
        }
        (Opcode::Divide, Value::Integer(l), Value::Integer(r)) => match r {
            0 => Err(EvalError::MathError("Division by zero".to_string())),
            _ => Ok(Value::Integer(l / r)),
        },
        (Opcode::Modulo, Value::Integer(l), Value::Integer(r)) => match r {
            0 => Err(EvalError::MathError("Modulo by zero".to_string())),
            _ => Ok(Value::Integer(l % r)),
        },
        (Opcode::Multiply, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l * r)),
        (Opcode::Plus, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
        (Opcode::Minus, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l - r)),
        (Opcode::Power, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l.powf(r))),
        (Opcode::Divide, Value::Float(l), Value::Float(r)) => match r {
            0.0 => Err(EvalError::MathError("Division by zero".to_string())),
            _ => Ok(Value::Float(l / r)),
        },
        (Opcode::Modulo, Value::Float(l), Value::Float(r)) => match r {
            0.0 => Err(EvalError::MathError("Modulo by zero".to_string())),
            _ => Ok(Value::Float(l % r)),
        },
        (Opcode::Multiply, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
        (Opcode::Plus, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
        (Opcode::Minus, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
        // we already ensured there's no mixture of int and float, and handled other operators,
        // so the only other option is that one of the values is a bool
        _ => Err(EvalError::InvalidType(
            "Bools not supported for binary math".to_string(),
        )),
    }
}

fn convert_binary_values(op: &Opcode, lhs: Value, rhs: Value) -> (&Opcode, Value, Value) {
    match (lhs.clone(), rhs.clone()) {
        (Value::Integer(i), Value::Float(f)) => (op, Value::Float(i as f64), Value::Float(f)),
        (Value::Float(f), Value::Integer(i)) => (op, Value::Float(f), Value::Float(i as f64)),
        _ => (op, lhs, rhs),
    }
}

/// Apply a binary bitwise operator.
///
/// Supports bitwise and, or, and xor for pairs of booleans or pairs of signed
/// integers. Boolean operands use Rust's boolean bit operators, and integer
/// operands use Rust's integer bit operators.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] if `op` is not a bitwise binary
/// opcode, [`EvalError::InvalidType`] for any float operand, and
/// [`EvalError::InvalidType`] when the operands are otherwise not the same
/// supported type.
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
/// Shifts an integer left or right by an integer amount using Rust's `<<` and
/// `>>` operators.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] if both operands are integers but
/// `op` is not a bitshift opcode. Returns [`EvalError::InvalidType`] if either
/// operand is not an integer.
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
/// Supports boolean `&&` and `||` for pairs of boolean values.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] if both operands are booleans but
/// `op` is not a logical binary opcode. Returns [`EvalError::InvalidType`] if
/// either operand is not a boolean.
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
/// Dispatches unary plus, minus, degrees, bitwise not, and logical not to the
/// helper that implements that unary operator family.
///
/// # Errors
///
/// Returns [`EvalError::InvalidArity`] when a binary-only opcode is supplied.
/// Errors from the selected unary helper are returned unchanged.
fn apply_unary(op: &Opcode, val: Value) -> Result<Value, EvalError> {
    match op {
        Opcode::Degrees | Opcode::Plus | Opcode::Minus => apply_unary_math(op, val),
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
/// Unary plus returns the operand unchanged, unary minus negates integers and
/// floats, and degrees converts integer or floating-point degree values into
/// floating-point radians.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean operands. Returns
/// [`EvalError::UnexpectedOpcode`] for any opcode other than unary plus, unary
/// minus, or degrees.
fn apply_unary_math(op: &Opcode, val: Value) -> Result<Value, EvalError> {
    match (op, val.clone()) {
        (_, Value::Bool(_)) => Err(EvalError::InvalidType(
            "Unary math operations not defined for bool".to_string(),
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
/// Boolean values are negated with logical not, since Rust booleans are only
/// `true` or `false`. Integer values are negated with Rust's bitwise `!`.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for floating-point operands.
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
/// Boolean values are negated with Rust's logical `!`.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for integer and floating-point operands.
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
    match func {
        Func::Min
        | Func::Max => apply_n_nary_function(func, vals),
        Func::Round
        | Func::Floor
        | Func::Ceiling => apply_rounding_function(func, vals),
        Func::Power
        | Func::Modulo
        | Func::Remainder => apply_binary_function(func, vals),
        Func::Cos
        | Func::Sin
        | Func::Tan
        | Func::ACos
        | Func::ASin
        | Func::ATan
        | Func::Abs
        | Func::Ln
        | Func::Log
        | Func::Exp => apply_unary_function(func, vals),
    }
}

fn apply_n_nary_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_rounding_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_binary_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_unary_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

// Some of the tests here are defensive programming; the AST will not
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
    fn test_unary_math_bool(#[case] op: Opcode) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::UnaryOperation {
            operator: op,
            value: Box::new(Expression::Bool(true)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Unary math operations not defined for bool".to_string(),
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
    fn test_unary_math_invalid_opcode(#[case] op: Opcode) {
        let result = apply_unary_math(&op, Value::Integer(1));

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
    fn test_apply_unary_invalid_arity(#[case] op: Opcode) {
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
    fn test_apply_binary_bit_operation_invalid_float(
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
    fn test_apply_binary_bit_operation_invalid_mixed_types(
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

    #[rustfmt::skip]
    #[rstest]
    #[case(Opcode::Plus, Expression::Integer(10), Expression::Integer(4), Value::Integer(14))]
    #[case(Opcode::Plus, Expression::Integer(10), Expression::Float(0.4), Value::Float(10.4))]
    #[case(Opcode::Plus, Expression::Float(1.0), Expression::Integer(4), Value::Float(5.0))]
    #[case(Opcode::Plus, Expression::Float(1.0), Expression::Float(0.4), Value::Float(1.4))]
    #[case(Opcode::Minus, Expression::Integer(10), Expression::Integer(4), Value::Integer(6))]
    #[case(Opcode::Minus, Expression::Integer(10), Expression::Float(0.5), Value::Float(9.5))]
    #[case(Opcode::Minus, Expression::Float(10.5), Expression::Integer(4), Value::Float(6.5))]
    #[case(Opcode::Minus, Expression::Float(10.5), Expression::Float(0.5), Value::Float(10.0))]
    #[case(Opcode::Multiply, Expression::Integer(10), Expression::Integer(4), Value::Integer(40))]
    #[case(Opcode::Multiply, Expression::Integer(10), Expression::Float(0.5), Value::Float(5.0))]
    #[case(Opcode::Multiply, Expression::Float(10.5), Expression::Integer(4), Value::Float(42.0))]
    #[case(Opcode::Multiply, Expression::Float(10.5), Expression::Float(0.5), Value::Float(5.25))]
    #[case(Opcode::Divide, Expression::Integer(12), Expression::Integer(3), Value::Integer(4))]
    #[case(Opcode::Divide, Expression::Integer(12), Expression::Float(3.0), Value::Float(4.0))]
    #[case(Opcode::Divide, Expression::Float(12.0), Expression::Integer(3), Value::Float(4.0))]
    #[case(Opcode::Divide, Expression::Float(12.0), Expression::Float(3.0), Value::Float(4.0))]
    #[case(Opcode::Modulo, Expression::Integer(13), Expression::Integer(5), Value::Integer(3))]
    #[case(Opcode::Modulo, Expression::Integer(13), Expression::Float(5.0), Value::Float(3.0))]
    #[case(Opcode::Modulo, Expression::Float(13.0), Expression::Integer(5), Value::Float(3.0))]
    #[case(Opcode::Modulo, Expression::Float(13.0), Expression::Float(5.0), Value::Float(3.0))]
    #[case(Opcode::Power, Expression::Integer(2), Expression::Integer(3), Value::Integer(8))]
    #[case(Opcode::Power, Expression::Integer(2), Expression::Float(3.0), Value::Float(8.0))]
    #[case(Opcode::Power, Expression::Float(2.0), Expression::Integer(3), Value::Float(8.0))]
    #[case(Opcode::Power, Expression::Float(2.0), Expression::Float(3.0), Value::Float(8.0))]
    fn test_apply_binary_math_regular(
        #[case] op: Opcode,
        #[case] lhs: Expression,
        #[case] rhs: Expression,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: op,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn test_apply_binary_math_integer_exponent() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Integer(10)),
            operator: Opcode::Power,
            rhs: Box::new(Expression::Integer(5_000_000_000)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::MathError(
                "Integer exponent too large".to_string()
            ))
        );
    }

    #[test]
    fn test_apply_binary_math_integer_overflow() {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Integer(1_000_000_000)),
            operator: Opcode::Power,
            rhs: Box::new(Expression::Integer(1_000_000_000)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::MathError(
                "Integer overflow on power".to_string()
            ))
        );
    }

    #[rustfmt::skip]
    #[rstest]
    #[case(Expression::Integer(10), Expression::Integer(0))]
    #[case(Expression::Float(10.0), Expression::Integer(0))]
    #[case(Expression::Integer(10), Expression::Float(0.0))]
    #[case(Expression::Float(10.0), Expression::Float(0.0))]
    fn test_apply_binary_math_divide_error(
        #[case] lhs: Expression,
        #[case] rhs: Expression,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::Divide,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(EvalError::MathError("Division by zero".to_string())));
    }

    #[rustfmt::skip]
    #[rstest]
    #[case(Expression::Integer(10), Expression::Integer(0))]
    #[case(Expression::Float(10.0), Expression::Integer(0))]
    #[case(Expression::Integer(10), Expression::Float(0.0))]
    #[case(Expression::Float(10.0), Expression::Float(0.0))]
    fn test_apply_binary_math_modulo_error(
        #[case] lhs: Expression,
        #[case] rhs: Expression,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode:: Modulo,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(EvalError::MathError("Modulo by zero".to_string())));
    }

    #[rstest]
    #[case(Expression::Integer(1), Expression::Bool(true))]
    #[case(Expression::Float(1.0), Expression::Bool(true))]
    #[case(Expression::Bool(true), Expression::Integer(1))]
    #[case(Expression::Bool(true), Expression::Float(1.0))]
    fn test_apply_binary_math_operation_invalid_typoes(
        #[case] lhs: Expression,
        #[case] rhs: Expression,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::Multiply,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Bools not supported for binary math".to_string()
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
    #[case(Opcode::BitshiftLeft)]
    #[case(Opcode::BitshiftRight)]
    #[case(Opcode::LogicalAnd)]
    #[case(Opcode::LogicalOr)]
    #[case(Opcode::LogicalNot)]
    #[case(Opcode::BitwiseNot)]
    #[case(Opcode::BitwiseAnd)]
    #[case(Opcode::BitwiseOr)]
    #[case(Opcode::BitwiseXor)]
    #[case(Opcode::Degrees)]
    fn test_apply_binary_math_operation_invalid_opcode(#[case] op: Opcode) {
        let result = apply_binary_math_operation(&op, Value::Integer(1), Value::Integer(1));

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[rustfmt::skip]
    #[rstest]
    #[case(Opcode::Equals, Expression::Integer(1), Expression::Integer(1), Value::Bool(true))]
    #[case(Opcode::Equals, Expression::Integer(1), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::Equals, Expression::Float(1.0), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::Equals, Expression::Float(1.0), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::NotEquals, Expression::Integer(1), Expression::Integer(2), Value::Bool(true))]
    #[case(Opcode::NotEquals, Expression::Integer(1), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::NotEquals, Expression::Float(1.0), Expression::Integer(1), Value::Bool(false))]
    #[case(Opcode::NotEquals, Expression::Float(1.0), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThan, Expression::Integer(1), Expression::Integer(2), Value::Bool(true))]
    #[case(Opcode::LessThan, Expression::Integer(1), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThan, Expression::Float(2.0), Expression::Integer(1), Value::Bool(false))]
    #[case(Opcode::LessThan, Expression::Float(1.0), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThanEquals, Expression::Integer(2), Expression::Integer(2), Value::Bool(true))]
    #[case(Opcode::LessThanEquals, Expression::Integer(2), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThanEquals, Expression::Float(2.0), Expression::Integer(1), Value::Bool(false))]
    #[case(Opcode::LessThanEquals, Expression::Float(1.0), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Integer(2), Expression::Integer(1), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Integer(2), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Float(1.0), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::GreaterThan, Expression::Float(2.0), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::GreaterThanEquals, Expression::Integer(2), Expression::Integer(2), Value::Bool(true))]
    #[case(Opcode::GreaterThanEquals, Expression::Integer(2), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::GreaterThanEquals, Expression::Float(1.0), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::GreaterThanEquals, Expression::Float(2.0), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::ApproximatelyEquals, Expression::Integer(1), Expression::Integer(1), Value::Bool(true))]
    #[case(Opcode::ApproximatelyEquals, Expression::Integer(1), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::ApproximatelyEquals, Expression::Integer(1000), Expression::Float(1000.0005), Value::Bool(true))]
    #[case(Opcode::ApproximatelyEquals, Expression::Float(1000.002), Expression::Integer(1000), Value::Bool(false))]
    #[case(Opcode::ApproximatelyEquals, Expression::Float(1000.0), Expression::Float(1000.0005), Value::Bool(true))]
    #[case(Opcode::ApproximatelyEquals, Expression::Float(1000.0), Expression::Float(1000.002), Value::Bool(false))]
    #[case(Opcode::Equals, Expression::Bool(true), Expression::Bool(true), Value::Bool(true))]
    #[case(Opcode::NotEquals, Expression::Bool(true), Expression::Bool(false), Value::Bool(true))]
    #[case(Opcode::LessThan, Expression::Bool(false), Expression::Bool(true), Value::Bool(true))]
    #[case(Opcode::LessThanEquals, Expression::Bool(true), Expression::Bool(true), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Bool(true), Expression::Bool(false), Value::Bool(true))]
    #[case(Opcode::GreaterThanEquals, Expression::Bool(false), Expression::Bool(false), Value::Bool(true))]
    #[case(Opcode::ApproximatelyEquals, Expression::Bool(true), Expression::Bool(true), Value::Bool(true))]
    #[case(Opcode::ApproximatelyEquals, Expression::Bool(true), Expression::Bool(false), Value::Bool(false))]
    fn test_apply_binary_comparison_regular(
        #[case] op: Opcode,
        #[case] lhs: Expression,
        #[case] rhs: Expression,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: op,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Expression::Integer(1), Expression::Bool(true))]
    #[case(Expression::Bool(true), Expression::Integer(1))]
    #[case(Expression::Float(1.0), Expression::Bool(true))]
    #[case(Expression::Bool(true), Expression::Float(1.0))]
    fn test_apply_binary_comparison_operation_invalid_types(
        #[case] lhs: Expression,
        #[case] rhs: Expression,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(lhs),
            operator: Opcode::Equals,
            rhs: Box::new(rhs),
        });

        let result = eval(&expr, &variables);

        assert_eq!(
            result,
            Err(EvalError::InvalidType(
                "Cannot mix types for binary comparison".to_string()
            ))
        );
    }

    #[rstest]
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
    #[case(Opcode::BitwiseAnd)]
    #[case(Opcode::BitwiseOr)]
    #[case(Opcode::BitwiseXor)]
    #[case(Opcode::Degrees)]
    fn test_apply_binary_comparison_invalid_opcode(#[case] op: Opcode) {
        let result = apply_binary_comparison(&op, Value::Integer(1), Value::Integer(1));

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

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
    #[case(Opcode::Degrees)]
    #[case(Opcode::LogicalNot)]
    #[case(Opcode::BitwiseNot)]
    fn test_apply_binary_invalid_arity(#[case] op: Opcode) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::BinaryOperation {
            lhs: Box::new(Expression::Bool(true)),
            operator: op,
            rhs: Box::new(Expression::Bool(true)),
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(EvalError::InvalidArity));
    }
}
