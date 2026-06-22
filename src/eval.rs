//! Evaluation for parsed expressions.
//!
//! # Overview
//!
//! This module evaluates an [`Expression`] tree into a runtime [`Value`]. Literal
//! values evaluate directly, variables are resolved from a caller-provided
//! resolver, and compound expressions are evaluated recursively before their
//! operators or functions are applied.
//!
//! # Errors
//!
//! Evaluation returns [`EvalError`] when an expression cannot be evaluated, a
//! referenced variable is missing, or an operator is used with the wrong arity.

use crate::ast::{Expression, Func, Opcode};
use crate::{ArithmeticOp, EvalError, Value, VariableResolver};
use roundable::{Roundable, Tie};
use std::num::FpCategory;

const EPSILON: f64 = 0.000001;

fn invalid_arity(expected: &'static str, actual: usize) -> EvalError {
    EvalError::InvalidArity { expected, actual }
}

fn invalid_type(expected: &'static str, actual: &'static str) -> EvalError {
    EvalError::InvalidType { expected, actual }
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "bool",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
    }
}

fn checked_float(value: f64) -> Result<Value, EvalError> {
    checked_f64(value).map(Value::Float)
}

fn checked_f64(value: f64) -> Result<f64, EvalError> {
    match value.classify() {
        FpCategory::Nan | FpCategory::Infinite => Err(EvalError::NonFiniteFloat),
        FpCategory::Subnormal => Err(EvalError::SubnormalFloat),
        FpCategory::Zero | FpCategory::Normal => Ok(value),
    }
}

fn checked_add_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    lhs.checked_add(rhs)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Add,
        })
}

fn checked_sub_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    lhs.checked_sub(rhs)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Subtract,
        })
}

fn checked_mul_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    lhs.checked_mul(rhs)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Multiply,
        })
}

fn checked_neg_i64(value: i64) -> Result<Value, EvalError> {
    value
        .checked_neg()
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Negate,
        })
}

fn checked_div_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    if rhs == 0 {
        return Err(EvalError::DivisionByZero);
    }

    lhs.checked_div(rhs)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Divide,
        })
}

fn checked_rem_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    if rhs == 0 {
        return Err(EvalError::DivisionByZero);
    }

    lhs.checked_rem(rhs)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Modulo,
        })
}

fn checked_rem_euclid_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    if rhs == 0 {
        return Err(EvalError::DivisionByZero);
    }

    lhs.checked_rem_euclid(rhs)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Remainder,
        })
}

fn checked_shift_count(count: i64) -> Result<u32, EvalError> {
    let count_u32 = u32::try_from(count).map_err(|_| EvalError::InvalidShiftCount { count })?;

    if count_u32 >= i64::BITS {
        return Err(EvalError::InvalidShiftCount { count });
    }

    Ok(count_u32)
}

fn checked_shl_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    let shift = checked_shift_count(rhs)?;
    lhs.checked_shl(shift)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::ShiftLeft,
        })
}

fn checked_shr_i64(lhs: i64, rhs: i64) -> Result<Value, EvalError> {
    let shift = checked_shift_count(rhs)?;
    lhs.checked_shr(shift)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::ShiftRight,
        })
}

/// Evaluate an expression into a runtime value.
///
/// `expr` is evaluated recursively. Literal expression nodes become their
/// corresponding [`Value`] variants, variable nodes are looked up in
/// `resolver`, and compound nodes delegate to the appropriate unary, binary, or
/// function evaluator after their child expressions have been evaluated.
///
/// # Errors
///
/// Returns [`EvalError::UnknownVariable`] when `expr` references a variable that
/// is not present in `resolver`.
///
/// Returns [`EvalError::InvalidExpression`] when `expr` contains an
/// [`Expression::Error`] or [`Expression::LexicalError`] node.
///
/// Returns [`EvalError::InvalidArity`] when a unary or binary operator is used
/// in a position where that operator is not supported. The grammar should prevent
/// this, so this is mostly a defensive programming decision
pub fn eval<R>(expr: &Expression, mut resolver: R) -> Result<Value, EvalError>
where
    R: VariableResolver,
{
    eval_with_resolver(expr, &mut resolver)
}

fn eval_with_resolver<R>(expr: &Expression, resolver: &mut R) -> Result<Value, EvalError>
where
    R: VariableResolver + ?Sized,
{
    match expr {
        Expression::Bool(n) => Ok(Value::Bool(*n)),
        Expression::Integer(n) => Ok(Value::Integer(*n)),
        Expression::Float(n) => checked_float(*n),

        Expression::UnaryOperation { operator, value } => {
            let value = eval_with_resolver(value, resolver)?;
            apply_unary(operator, value)
        }

        Expression::BinaryOperation { lhs, operator, rhs } => {
            let left = eval_with_resolver(lhs, resolver)?;
            let right = eval_with_resolver(rhs, resolver)?;
            apply_binary(operator, left, right)
        }

        Expression::Function { func, arguments } => {
            let values = arguments
                .iter()
                .map(|v| eval_with_resolver(v, resolver))
                .collect::<Result<Vec<_>, _>>()?;

            apply_function(func, values)
        }

        Expression::Variable(name) => {
            let value = resolver.resolve(name)?;
            validate_value(value)
        }

        Expression::Error | Expression::LexicalError(_) => Err(EvalError::InvalidExpression),
    }
}

fn validate_value(value: Value) -> Result<Value, EvalError> {
    match value {
        Value::Bool(value) => Ok(Value::Bool(value)),
        Value::Integer(value) => Ok(Value::Integer(value)),
        Value::Float(value) => checked_float(value),
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
        | Opcode::LogicalOr => Err(invalid_arity("unary operator", 1)),
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
/// minus, or degrees. The unexpected-opcode branch is unreachable through
/// [`apply_unary`], which only routes unary math opcodes here.
fn apply_unary_math(op: &Opcode, val: Value) -> Result<Value, EvalError> {
    match (op, val) {
        (_, Value::Bool(_)) => Err(invalid_type("integer or float", "bool")),
        (Opcode::Plus, Value::Float(f)) => checked_float(f),
        (Opcode::Plus, value) => Ok(value),
        (Opcode::Minus, Value::Integer(i)) => checked_neg_i64(i),
        (Opcode::Minus, Value::Float(f)) => checked_float(-f),
        (Opcode::Degrees, Value::Integer(i)) => checked_float((i as f64).to_radians()),
        (Opcode::Degrees, Value::Float(f)) => checked_float(f.to_radians()),
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
        Value::Float(_) => Err(invalid_type("bool or integer", "float")),
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
        value @ (Value::Integer(_) | Value::Float(_)) => {
            Err(invalid_type("bool", value_type(&value)))
        }
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
        Opcode::Degrees | Opcode::BitwiseNot | Opcode::LogicalNot => {
            Err(invalid_arity("binary operator", 2))
        }
    }
}

/// Apply a binary comparison operator.
///
/// Integer and float pairs are promoted to floats before comparison. Boolean
/// pairs support equality and ordering using Rust's boolean ordering.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] when `op` is not a supported
/// comparison operator; that branch is unreachable through [`apply_binary`],
/// which only routes comparison opcodes here. Returns [`EvalError::InvalidType`]
/// when the operands are not the same type after integer/float promotion.
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
            let scale = l.abs().max(r.abs()).max(1.0);
            Ok(Value::Bool((l - r).abs() <= EPSILON * scale))
        }
        _ => Err(invalid_type("matching comparable types", "mixed types")),
    }
}

/// Promote mixed integer/float binary operands to floats.
///
/// Values that are already the same type are returned unchanged. Boolean values
/// are left untouched so the caller can decide whether they are valid for the
/// operation family.
fn convert_binary_values(op: &Opcode, lhs: Value, rhs: Value) -> (&Opcode, Value, Value) {
    match (lhs, rhs) {
        (Value::Integer(i), Value::Float(f)) => (op, Value::Float(i as f64), Value::Float(f)),
        (Value::Float(f), Value::Integer(i)) => (op, Value::Float(f), Value::Float(i as f64)),
        (lhs, rhs) => (op, lhs, rhs),
    }
}

/// Apply a binary arithmetic operator.
///
/// Supports power, multiplication, division, addition, subtraction, and modulo
/// for integer pairs and floating-point pairs. Mixed integer/float inputs are
/// promoted to floats before dispatch.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] for non-arithmetic operators.
/// That branch is unreachable through [`apply_binary`], which only routes
/// arithmetic opcodes here. Returns [`EvalError::InvalidType`] for boolean
/// operands. Arithmetic failures are returned as typed errors such as
/// [`EvalError::DivisionByZero`], [`EvalError::IntegerOverflow`],
/// [`EvalError::InvalidExponent`], [`EvalError::NonFiniteFloat`], or
/// [`EvalError::SubnormalFloat`].
fn apply_binary_math_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
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
        (Opcode::Power, Value::Integer(l), Value::Integer(r)) => checked_integer_power(l, r),
        (Opcode::Divide, Value::Integer(l), Value::Integer(r)) => checked_div_i64(l, r),
        (Opcode::Modulo, Value::Integer(l), Value::Integer(r)) => checked_rem_i64(l, r),
        (Opcode::Multiply, Value::Integer(l), Value::Integer(r)) => checked_mul_i64(l, r),
        (Opcode::Plus, Value::Integer(l), Value::Integer(r)) => checked_add_i64(l, r),
        (Opcode::Minus, Value::Integer(l), Value::Integer(r)) => checked_sub_i64(l, r),
        (Opcode::Power, Value::Float(l), Value::Float(r)) => checked_float(l.powf(r)),
        (Opcode::Divide, Value::Float(l), Value::Float(r)) => match r {
            0.0 => Err(EvalError::DivisionByZero),
            _ => checked_float(l / r),
        },
        (Opcode::Modulo, Value::Float(l), Value::Float(r)) => match r {
            0.0 => Err(EvalError::DivisionByZero),
            _ => checked_float(l % r),
        },
        (Opcode::Multiply, Value::Float(l), Value::Float(r)) => checked_float(l * r),
        (Opcode::Plus, Value::Float(l), Value::Float(r)) => checked_float(l + r),
        (Opcode::Minus, Value::Float(l), Value::Float(r)) => checked_float(l - r),
        // we already ensured there's no mixture of int and float, and handled other operators,
        // so the only other option is that one of the values is a bool
        _ => Err(invalid_type("integer or float", "bool")),
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
/// opcode; that branch is unreachable through [`apply_binary`], which only
/// routes bitwise binary opcodes here. Returns [`EvalError::InvalidType`] for
/// any float operand, and [`EvalError::InvalidType`] when the operands are
/// otherwise not the same supported type.
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
        (_, Value::Float(_), _) | (_, _, Value::Float(_)) => {
            Err(invalid_type("bool or integer", "float"))
        }
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
            invalid_type("matching bool or integer types", "mixed types"),
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
/// `op` is not a bitshift opcode; that branch is unreachable through
/// [`apply_binary`], which only routes bitshift opcodes here. Returns
/// [`EvalError::InvalidType`] if either operand is not an integer.
fn apply_bitshift_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    if let (Value::Integer(l), Value::Integer(r)) = (lhs, rhs) {
        match op {
            Opcode::BitshiftLeft => checked_shl_i64(l, r),
            Opcode::BitshiftRight => checked_shr_i64(l, r),
            _ => Err(EvalError::UnexpectedOpcode),
        }
    } else {
        Err(invalid_type("integer", "non-integer"))
    }
}

/// Apply a binary logical operator.
///
/// Supports boolean `&&` and `||` for pairs of boolean values.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] if both operands are booleans but
/// `op` is not a logical binary opcode; that branch is unreachable through
/// [`apply_binary`], which only routes logical binary opcodes here. Returns
/// [`EvalError::InvalidType`] if either operand is not a boolean.
fn apply_binary_logical_operation(op: &Opcode, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    if let (Value::Bool(l), Value::Bool(r)) = (lhs, rhs) {
        match op {
            Opcode::LogicalAnd => Ok(Value::Bool(l && r)),
            Opcode::LogicalOr => Ok(Value::Bool(l || r)),
            _ => Err(EvalError::UnexpectedOpcode),
        }
    } else {
        Err(invalid_type("bool", "non-bool"))
    }
}

/************** Functions operations **************/

/// Apply a built-in function to evaluated argument values.
///
/// Dispatches each [`Func`] to the helper that implements that function's arity
/// and type rules.
///
/// # Errors
///
/// Returns any validation or math error produced by the selected function
/// family. The parser should only construct supported function opcodes, but the
/// lower-level helpers still use [`EvalError::UnexpectedOpcode`] defensively
/// when called directly with a function from the wrong family.
fn apply_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    match func {
        Func::Min | Func::Max => apply_n_nary_function(func, vals),
        Func::Round | Func::Floor | Func::Ceiling => apply_rounding_function(func, vals),
        Func::Power | Func::Modulo | Func::Remainder => apply_binary_function(func, vals),
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

/// Apply an n-nary numeric function such as min or max.
///
/// The input vector is first normalized by [`pare_vector_n_nary`], so normal
/// calls either contain all integers or all floats.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] when validation sees a boolean argument.
/// Returns [`EvalError::InvalidArity`] for min/max with no arguments. Returns
/// [`EvalError::UnexpectedOpcode`] when called directly with a function outside
/// the n-nary family; that branch is unreachable through [`apply_function`].
fn apply_n_nary_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    let vals = pare_vector_n_nary(vals)?;

    match func {
        Func::Min => apply_min_function(vals),
        Func::Max => apply_max_function(vals),
        Func::Power
        | Func::Modulo
        | Func::Remainder
        | Func::Round
        | Func::Floor
        | Func::Ceiling
        | Func::Cos
        | Func::Sin
        | Func::Tan
        | Func::ACos
        | Func::ASin
        | Func::ATan
        | Func::Abs
        | Func::Ln
        | Func::Log
        | Func::Exp => Err(EvalError::UnexpectedOpcode),
    }
}

/// Validate and normalize arguments for n-nary numeric functions.
///
/// Any number of arguments is accepted. If any argument is a float, all integer
/// arguments are promoted to floats so downstream min/max logic can operate on a
/// homogeneous vector.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] when any argument is a boolean.
fn pare_vector_n_nary(vals: Vec<Value>) -> Result<Vec<Value>, EvalError> {
    if vals.iter().any(|value| matches!(value, Value::Bool(_))) {
        return Err(invalid_type("integer or float", "bool"));
    }

    if vals.iter().any(|value| matches!(value, Value::Float(_))) {
        Ok(vals
            .into_iter()
            .map(|value| match value {
                Value::Integer(value) => Value::Float(value as f64),
                value => value,
            })
            .collect())
    } else {
        Ok(vals)
    }
}

/// Return the minimum value from a normalized numeric vector.
///
/// Integer inputs produce an integer result and float inputs produce a float
/// result. Through [`apply_n_nary_function`], the vector has already been
/// validated and normalized to a single numeric type.
///
/// # Errors
///
/// Returns [`EvalError::InvalidArity`] for an empty vector. Returns
/// [`EvalError::InvalidType`] for booleans or mixed numeric types; those branches
/// are defensive and should be unreachable unless this helper is called directly
/// without first calling [`pare_vector_n_nary`].
fn apply_min_function(vals: Vec<Value>) -> Result<Value, EvalError> {
    match vals.as_slice() {
        [] => Err(invalid_arity("at least 1", 0)),
        [Value::Integer(_), ..] => {
            let mut min = None;
            for value in vals {
                match value {
                    Value::Integer(value) => {
                        min = Some(min.map_or(value, |min: i64| min.min(value)))
                    }
                    // apply_n_nary_function calls pare_vector_n_nary first, so this
                    // branch is only reachable if apply_min_function is called directly.
                    Value::Bool(_) | Value::Float(_) => {
                        return Err(invalid_type("integer", value_type(&value)));
                    }
                }
            }
            min.map(Value::Integer)
                .ok_or(invalid_arity("at least 1", 0))
        }
        [Value::Float(_), ..] => {
            let mut min = None;
            for value in vals {
                match value {
                    Value::Float(value) => min = Some(min.map_or(value, |min: f64| min.min(value))),
                    // apply_n_nary_function calls pare_vector_n_nary first, so this
                    // branch is only reachable if apply_min_function is called directly.
                    Value::Bool(_) | Value::Integer(_) => {
                        return Err(invalid_type("float", value_type(&value)));
                    }
                }
            }
            min.map(checked_float)
                .unwrap_or_else(|| Err(invalid_arity("at least 1", 0)))
        }
        [Value::Bool(_), ..] => Err(invalid_type("integer or float", "bool")),
    }
}

/// Return the maximum value from a normalized numeric vector.
///
/// Integer inputs produce an integer result and float inputs produce a float
/// result. Through [`apply_n_nary_function`], the vector has already been
/// validated and normalized to a single numeric type.
///
/// # Errors
///
/// Returns [`EvalError::InvalidArity`] for an empty vector. Returns
/// [`EvalError::InvalidType`] for booleans or mixed numeric types; those branches
/// are defensive and should be unreachable unless this helper is called directly
/// without first calling [`pare_vector_n_nary`].
fn apply_max_function(vals: Vec<Value>) -> Result<Value, EvalError> {
    match vals.as_slice() {
        [] => Err(invalid_arity("at least 1", 0)),
        [Value::Integer(_), ..] => {
            let mut max = None;
            for value in vals {
                match value {
                    Value::Integer(value) => {
                        max = Some(max.map_or(value, |max: i64| max.max(value)))
                    }
                    // apply_n_nary_function calls pare_vector_n_nary first, so this
                    // branch is only reachable if apply_max_function is called directly.
                    Value::Bool(_) | Value::Float(_) => {
                        return Err(invalid_type("integer", value_type(&value)));
                    }
                }
            }
            max.map(Value::Integer)
                .ok_or(invalid_arity("at least 1", 0))
        }
        [Value::Float(_), ..] => {
            let mut max = None;
            for value in vals {
                match value {
                    Value::Float(value) => max = Some(max.map_or(value, |max: f64| max.max(value))),
                    // apply_n_nary_function calls pare_vector_n_nary first, so this
                    // branch is only reachable if apply_max_function is called directly.
                    Value::Bool(_) | Value::Integer(_) => {
                        return Err(invalid_type("float", value_type(&value)));
                    }
                }
            }
            max.map(checked_float)
                .unwrap_or_else(|| Err(invalid_arity("at least 1", 0)))
        }
        [Value::Bool(_), ..] => Err(invalid_type("integer or float", "bool")),
    }
}

/// Apply a rounding-family function.
///
/// Handles `round`, `floor`, and `ceiling` after validating that the argument
/// list contains one numeric value and an optional numeric precision.
///
/// # Errors
///
/// Returns validation errors from [`pare_vector_rounding`], math/type errors
/// from the selected rounding helper, or [`EvalError::UnexpectedOpcode`] when
/// called directly with a function outside the rounding family. The
/// unexpected-opcode branch is unreachable through [`apply_function`].
fn apply_rounding_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    let (value, precision) = pare_vector_rounding(vals)?;

    match func {
        Func::Round => apply_round_function(value, precision),
        Func::Floor => apply_floor_function(value, precision),
        Func::Ceiling => apply_ceiling_function(value, precision),
        Func::Min
        | Func::Max
        | Func::Power
        | Func::Modulo
        | Func::Remainder
        | Func::Cos
        | Func::Sin
        | Func::Tan
        | Func::ACos
        | Func::ASin
        | Func::ATan
        | Func::Abs
        | Func::Ln
        | Func::Log
        | Func::Exp => Err(EvalError::UnexpectedOpcode),
    }
}

/// Validate arguments for rounding-family functions.
///
/// Rounding functions accept one numeric value and an optional numeric precision.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] when either argument is a boolean. Returns
/// [`EvalError::InvalidArity`] when the argument count is not one or two.
fn pare_vector_rounding(vals: Vec<Value>) -> Result<(Value, Option<Value>), EvalError> {
    match vals.as_slice() {
        [Value::Bool(_)] | [Value::Bool(_), _] | [_, Value::Bool(_)] => {
            Err(invalid_type("integer or float", "bool"))
        }
        [value] => Ok((value.clone(), None)),
        [value, precision] => Ok((value.clone(), Some(precision.clone()))),
        _ => Err(invalid_arity("1 or 2", vals.len())),
    }
}

/// Apply round-to-precision semantics to a value.
///
/// Integer values with no precision or integer precision return integers. Float
/// values with integer precision return integers; any float precision returns a
/// float.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean inputs,
/// [`EvalError::InvalidPrecision`] for non-positive precision, or typed
/// overflow/non-finite errors from the numeric rounding helper.
fn apply_round_function(value: Value, precision: Option<Value>) -> Result<Value, EvalError> {
    match (value, precision) {
        (Value::Integer(value), None) => round_i64(value, 1).map(Value::Integer),
        (Value::Float(value), None) => round_f64(value, 1.0).and_then(checked_float),
        (Value::Integer(value), Some(Value::Integer(precision))) => {
            round_i64(value, precision).map(Value::Integer)
        }
        (Value::Integer(value), Some(Value::Float(precision))) => {
            round_f64(value as f64, precision).and_then(checked_float)
        }
        (Value::Float(value), Some(Value::Integer(precision))) => {
            round_f64_to_i64(value, precision).map(Value::Integer)
        }
        (Value::Float(value), Some(Value::Float(precision))) => {
            round_f64(value, precision).and_then(checked_float)
        }
        _ => Err(invalid_type("integer or float", "bool")),
    }
}

/// Round an integer to the nearest positive integer precision.
///
/// Ties are rounded upward according to [`Tie::Up`].
///
/// # Errors
///
/// Returns [`EvalError::InvalidPrecision`] when precision is non-positive or
/// [`EvalError::IntegerOverflow`] when the rounded integer would overflow.
fn round_i64(value: i64, precision: i64) -> Result<i64, EvalError> {
    if precision <= 0 {
        return Err(EvalError::InvalidPrecision);
    }

    value
        .try_round_to(precision, Tie::Up)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Round,
        })
}

/// Round a float to the nearest positive floating-point precision.
///
/// Ties are rounded upward according to [`Tie::Up`].
///
/// # Errors
///
/// Returns [`EvalError::InvalidPrecision`] when precision is non-positive or
/// [`EvalError::NonFiniteFloat`] when the operation cannot produce a finite
/// float, or [`EvalError::SubnormalFloat`] for subnormal results.
fn round_f64(value: f64, precision: f64) -> Result<f64, EvalError> {
    if precision <= 0.0 {
        return Err(EvalError::InvalidPrecision);
    }

    let rounded = value.try_round_to(precision, Tie::Up).unwrap_or(f64::NAN);

    checked_f64(rounded)
}

/// Round a float and convert the result to an integer.
///
/// # Errors
///
/// Returns errors from [`round_f64`]. Returns [`EvalError::IntegerOverflow`] if
/// the rounded value falls outside the `i64` range.
fn round_f64_to_i64(value: f64, precision: i64) -> Result<i64, EvalError> {
    let rounded = round_f64(value, precision as f64)?;

    checked_f64_to_i64(rounded, "round")
}

fn checked_f64_to_i64(value: f64, _operation: &str) -> Result<i64, EvalError> {
    const I64_MIN_AS_F64: f64 = i64::MIN as f64;
    const I64_MAX_EXCLUSIVE_AS_F64: f64 = -(i64::MIN as f64);

    checked_f64(value)?;

    if !(I64_MIN_AS_F64..I64_MAX_EXCLUSIVE_AS_F64).contains(&value) {
        return Err(EvalError::IntegerOverflow {
            op: ArithmeticOp::FloatToInteger,
        });
    }

    Ok(value as i64)
}

/// Apply floor-to-precision semantics to a value.
///
/// Integer values with no precision or integer precision return integers. Float
/// values with integer precision return integers; any float precision returns a
/// float.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean inputs,
/// [`EvalError::InvalidPrecision`] for non-positive precision, or typed
/// overflow/non-finite errors from the numeric floor helper.
fn apply_floor_function(value: Value, precision: Option<Value>) -> Result<Value, EvalError> {
    match (value, precision) {
        (Value::Integer(value), None) => floor_i64(value, 1).map(Value::Integer),
        (Value::Float(value), None) => floor_f64(value, 1.0).and_then(checked_float),
        (Value::Integer(value), Some(Value::Integer(precision))) => {
            floor_i64(value, precision).map(Value::Integer)
        }
        (Value::Integer(value), Some(Value::Float(precision))) => {
            floor_f64(value as f64, precision).and_then(checked_float)
        }
        (Value::Float(value), Some(Value::Integer(precision))) => {
            floor_f64_to_i64(value, precision).map(Value::Integer)
        }
        (Value::Float(value), Some(Value::Float(precision))) => {
            floor_f64(value, precision).and_then(checked_float)
        }
        _ => Err(invalid_type("integer or float", "bool")),
    }
}

/// Floor an integer to a positive integer precision.
///
/// Uses Euclidean division so negative numbers floor toward negative infinity
/// relative to the requested precision.
///
/// # Errors
///
/// Returns [`EvalError::InvalidPrecision`] when precision is non-positive or
/// [`EvalError::IntegerOverflow`] when the resulting integer multiple would
/// overflow.
fn floor_i64(value: i64, precision: i64) -> Result<i64, EvalError> {
    if precision <= 0 {
        return Err(EvalError::InvalidPrecision);
    }

    value
        .div_euclid(precision)
        .checked_mul(precision)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Floor,
        })
}

/// Floor a float to a positive floating-point precision.
///
/// # Errors
///
/// Returns [`EvalError::InvalidPrecision`] when precision is non-positive or
/// [`EvalError::NonFiniteFloat`] when the computed floor is not finite, or
/// [`EvalError::SubnormalFloat`] for subnormal results.
fn floor_f64(value: f64, precision: f64) -> Result<f64, EvalError> {
    if precision <= 0.0 {
        return Err(EvalError::InvalidPrecision);
    }

    let result = (value / precision).floor() * precision;
    checked_f64(result)
}

/// Floor a float and convert the result to an integer.
///
/// # Errors
///
/// Returns errors from [`floor_f64`]. Returns [`EvalError::IntegerOverflow`] if
/// the floored value falls outside the `i64` range.
fn floor_f64_to_i64(value: f64, precision: i64) -> Result<i64, EvalError> {
    let floored = floor_f64(value, precision as f64)?;

    checked_f64_to_i64(floored, "floor")
}

/// Apply ceiling-to-precision semantics to a value.
///
/// Integer values with no precision or integer precision return integers. Float
/// values with integer precision return integers; any float precision returns a
/// float.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean inputs,
/// [`EvalError::InvalidPrecision`] for non-positive precision, or typed
/// overflow/non-finite errors from the numeric ceiling helper.
fn apply_ceiling_function(value: Value, precision: Option<Value>) -> Result<Value, EvalError> {
    match (value, precision) {
        (Value::Integer(value), None) => ceiling_i64(value, 1).map(Value::Integer),
        (Value::Float(value), None) => ceiling_f64(value, 1.0).and_then(checked_float),
        (Value::Integer(value), Some(Value::Integer(precision))) => {
            ceiling_i64(value, precision).map(Value::Integer)
        }
        (Value::Integer(value), Some(Value::Float(precision))) => {
            ceiling_f64(value as f64, precision).and_then(checked_float)
        }
        (Value::Float(value), Some(Value::Integer(precision))) => {
            ceiling_f64_to_i64(value, precision).map(Value::Integer)
        }
        (Value::Float(value), Some(Value::Float(precision))) => {
            ceiling_f64(value, precision).and_then(checked_float)
        }
        _ => Err(invalid_type("integer or float", "bool")),
    }
}

/// Ceiling an integer to a positive integer precision.
///
/// # Errors
///
/// Returns [`EvalError::InvalidPrecision`] when precision is non-positive or
/// [`EvalError::IntegerOverflow`] when adding the precision to the floored base
/// would overflow.
fn ceiling_i64(value: i64, precision: i64) -> Result<i64, EvalError> {
    if precision <= 0 {
        return Err(EvalError::InvalidPrecision);
    }

    let base = floor_i64(value, precision)?;
    if base == value {
        Ok(base)
    } else {
        base.checked_add(precision)
            .ok_or(EvalError::IntegerOverflow {
                op: ArithmeticOp::Ceiling,
            })
    }
}

/// Ceiling a float to a positive floating-point precision.
///
/// # Errors
///
/// Returns [`EvalError::InvalidPrecision`] when precision is non-positive or
/// [`EvalError::NonFiniteFloat`] when the computed ceiling is not finite, or
/// [`EvalError::SubnormalFloat`] for subnormal results.
fn ceiling_f64(value: f64, precision: f64) -> Result<f64, EvalError> {
    if precision <= 0.0 {
        return Err(EvalError::InvalidPrecision);
    }

    let result = (value / precision).ceil() * precision;
    checked_f64(result)
}

/// Ceiling a float and convert the result to an integer.
///
/// # Errors
///
/// Returns errors from [`ceiling_f64`]. Returns [`EvalError::IntegerOverflow`]
/// if the ceiling value falls outside the `i64` range.
fn ceiling_f64_to_i64(value: f64, precision: i64) -> Result<i64, EvalError> {
    let ceiling = ceiling_f64(value, precision as f64)?;

    checked_f64_to_i64(ceiling, "ceiling")
}

/// Apply a two-argument numeric function.
///
/// Handles power, modulo, and Euclidean remainder after validating that exactly
/// two non-boolean arguments were supplied.
///
/// # Errors
///
/// Returns validation errors from [`pare_vector_binary`], math/type errors from
/// the selected helper, or [`EvalError::UnexpectedOpcode`] when called directly
/// with a function outside the binary-function family. The unexpected-opcode
/// branch is unreachable through [`apply_function`].
fn apply_binary_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    let (lhs, rhs) = pare_vector_binary(vals)?;

    match func {
        Func::Power => apply_power_function(lhs, rhs),
        Func::Modulo => apply_modulo_function(lhs, rhs),
        Func::Remainder => apply_remainder_function(lhs, rhs),
        Func::Min
        | Func::Max
        | Func::Round
        | Func::Cos
        | Func::Sin
        | Func::Tan
        | Func::ACos
        | Func::ASin
        | Func::ATan
        | Func::Abs
        | Func::Ln
        | Func::Log
        | Func::Exp
        | Func::Floor
        | Func::Ceiling => Err(EvalError::UnexpectedOpcode),
    }
}

/// Validate arguments for two-argument numeric functions.
///
/// # Errors
///
/// Returns [`EvalError::InvalidArity`] unless exactly two arguments are supplied.
/// Returns [`EvalError::InvalidType`] when either argument is a boolean.
fn pare_vector_binary(vals: Vec<Value>) -> Result<(Value, Value), EvalError> {
    let actual = vals.len();
    let [lhs, rhs]: [Value; 2] = vals.try_into().map_err(|_| invalid_arity("2", actual))?;

    if matches!(lhs, Value::Bool(_)) || matches!(rhs, Value::Bool(_)) {
        Err(invalid_type("integer or float", "bool"))
    } else {
        Ok((lhs, rhs))
    }
}

/// Apply exponentiation to two numeric values.
///
/// Integer bases with integer exponents use checked integer exponentiation.
/// Mixed integer/float inputs are evaluated as floats.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean inputs,
/// [`EvalError::InvalidExponent`] when an integer exponent cannot convert to
/// `u32`, or [`EvalError::IntegerOverflow`] when checked integer
/// exponentiation overflows.
fn apply_power_function(lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => checked_integer_power(l, r),
        (Value::Integer(l), Value::Float(r)) => checked_float((l as f64).powf(r)),
        (Value::Float(l), Value::Integer(r)) => checked_float(l.powf(r as f64)),
        (Value::Float(l), Value::Float(r)) => checked_float(l.powf(r)),
        _ => Err(invalid_type("integer or float", "bool")),
    }
}

fn checked_integer_power(base: i64, exponent: i64) -> Result<Value, EvalError> {
    let exponent: u32 = exponent
        .try_into()
        .map_err(|_| EvalError::InvalidExponent { exponent })?;

    base.checked_pow(exponent)
        .map(Value::Integer)
        .ok_or(EvalError::IntegerOverflow {
            op: ArithmeticOp::Power,
        })
}

/// Apply Rust remainder (`%`) semantics to two numeric values.
///
/// Mixed integer/float inputs are evaluated as floats.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean inputs.
fn apply_modulo_function(lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => checked_rem_i64(l, r),
        (Value::Integer(_), Value::Float(0.0)) | (Value::Float(_), Value::Float(0.0)) => {
            Err(EvalError::DivisionByZero)
        }
        (Value::Float(_), Value::Integer(0)) => Err(EvalError::DivisionByZero),
        (Value::Integer(l), Value::Float(r)) => checked_float((l as f64) % r),
        (Value::Float(l), Value::Integer(r)) => checked_float(l % (r as f64)),
        (Value::Float(l), Value::Float(r)) => checked_float(l % r),
        _ => Err(invalid_type("integer or float", "bool")),
    }
}

/// Apply Euclidean remainder semantics to two numeric values.
///
/// Mixed integer/float inputs are evaluated as floats.
///
/// # Errors
///
/// Returns [`EvalError::InvalidType`] for boolean inputs.
fn apply_remainder_function(lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    match (lhs, rhs) {
        (Value::Integer(l), Value::Integer(r)) => checked_rem_euclid_i64(l, r),
        (Value::Integer(_), Value::Float(0.0)) | (Value::Float(_), Value::Float(0.0)) => {
            Err(EvalError::DivisionByZero)
        }
        (Value::Float(_), Value::Integer(0)) => Err(EvalError::DivisionByZero),
        (Value::Integer(l), Value::Float(r)) => checked_float((l as f64).rem_euclid(r)),
        (Value::Float(l), Value::Integer(r)) => checked_float(l.rem_euclid(r as f64)),
        (Value::Float(l), Value::Float(r)) => checked_float(l.rem_euclid(r)),
        _ => Err(invalid_type("integer or float", "bool")),
    }
}

/// Apply a one-argument floating-point function.
///
/// Accepts integer or float arguments via [`pare_vector_unary`], which promotes
/// integers to floats before dispatching to the selected math function.
///
/// # Errors
///
/// Returns validation errors from [`pare_vector_unary`] or
/// [`EvalError::UnexpectedOpcode`] when called directly with a function outside
/// the unary-function family. The unexpected-opcode branch is unreachable
/// through [`apply_function`].
fn apply_unary_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    let value = pare_vector_unary(vals)?;

    match func {
        Func::Cos => apply_float_unary(value, f64::cos),
        Func::Sin => apply_float_unary(value, f64::sin),
        Func::Tan => apply_float_unary(value, f64::tan),
        Func::ACos => apply_float_unary(value, f64::acos),
        Func::ASin => apply_float_unary(value, f64::asin),
        Func::ATan => apply_float_unary(value, f64::atan),
        Func::Abs => apply_float_unary(value, f64::abs),
        Func::Ln => apply_float_unary(value, f64::ln),
        Func::Log => apply_float_unary(value, f64::log10),
        Func::Exp => apply_float_unary(value, f64::exp),
        Func::Min
        | Func::Max
        | Func::Power
        | Func::Modulo
        | Func::Remainder
        | Func::Round
        | Func::Floor
        | Func::Ceiling => Err(EvalError::UnexpectedOpcode),
    }
}

/// Validate arguments for one-argument floating-point functions.
///
/// Integers are promoted to floats so unary math functions can operate on a
/// homogeneous value type.
///
/// # Errors
///
/// Returns [`EvalError::InvalidArity`] unless exactly one argument is supplied.
/// Returns [`EvalError::InvalidType`] when the argument is a boolean.
fn pare_vector_unary(vals: Vec<Value>) -> Result<Value, EvalError> {
    let actual = vals.len();
    let [value]: [Value; 1] = vals.try_into().map_err(|_| invalid_arity("1", actual))?;

    match value {
        Value::Bool(_) => Err(invalid_type("integer or float", "bool")),
        Value::Integer(value) => checked_float(value as f64),
        Value::Float(value) => checked_float(value),
    }
}

/// Apply a floating-point unary operation to a normalized value.
///
/// # Errors
///
/// Returns [`EvalError::UnexpectedOpcode`] for non-float inputs. That branch is
/// defensive and should be unreachable through [`apply_unary_function`], because
/// [`pare_vector_unary`] converts integers to floats and rejects booleans.
fn apply_float_unary(val: Value, op: fn(f64) -> f64) -> Result<Value, EvalError> {
    match val {
        Value::Float(value) => checked_float(op(value)),
        Value::Bool(_) | Value::Integer(_) => Err(EvalError::UnexpectedOpcode),
    }
}

// Some of the tests here are defensive programming; the AST will not
// come out with a binary operator in a unary operation. But if that ever
// changes in the future, as a whole or for a particular operator, this
// will result in failing tests, which is what we want
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::*;
    use std::collections::HashMap;

    // in general, we'll test from the public eval function. The exceptions
    // are redundant/defensive error handling, which we're testing as a way
    // to catch regressions/changed assumptions

    fn value_to_expression(value: Value) -> Expression<'static> {
        match value {
            Value::Bool(value) => Expression::Bool(value),
            Value::Integer(value) => Expression::Integer(value),
            Value::Float(value) => Expression::Float(value),
        }
    }

    fn small_i64() -> impl Strategy<Value = i64> {
        -1_000_000i64..1_000_000
    }

    fn tiny_i64() -> impl Strategy<Value = i64> {
        -1_000i64..1_000
    }

    fn nonzero_tiny_i64() -> impl Strategy<Value = i64> {
        tiny_i64().prop_filter("nonzero integer", |value| *value != 0)
    }

    fn small_f64() -> impl Strategy<Value = f64> {
        (-1_000_000.0f64..1_000_000.0)
            .prop_filter("normal or zero float", |value| valid_float(*value))
    }

    fn tiny_f64() -> impl Strategy<Value = f64> {
        (-1_000.0f64..1_000.0).prop_filter("normal or zero float", |value| valid_float(*value))
    }

    fn nonzero_tiny_f64() -> impl Strategy<Value = f64> {
        tiny_f64().prop_filter("nonzero float", |value| *value != 0.0)
    }

    fn positive_f64() -> impl Strategy<Value = f64> {
        (0.000001f64..1_000_000.0).prop_filter("normal or zero float", |value| valid_float(*value))
    }

    fn unit_f64() -> impl Strategy<Value = f64> {
        (-1.0f64..1.0).prop_filter("normal or zero float", |value| valid_float(*value))
    }

    fn valid_float(value: f64) -> bool {
        matches!(value.classify(), FpCategory::Zero | FpCategory::Normal)
    }

    fn generated_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            any::<bool>().prop_map(Value::Bool),
            small_i64().prop_map(Value::Integer),
            small_f64().prop_map(Value::Float),
        ]
    }

    /************ Test helper tests *************/

    #[test]
    fn test_value_to_expression_bool() {
        let result = value_to_expression(Value::Bool(true));

        assert_eq!(result, Expression::Bool(true));
    }

    /************ Expression dispatch tests *************/

    #[test]
    fn test_eval_variable_known() {
        let mut variables: HashMap<String, Value> = HashMap::new();
        variables.insert("Test_Name".to_string(), Value::Integer(42));
        let expr = Box::new(Expression::Variable("Test_Name"));

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(Value::Integer(42)));
    }

    #[rstest]
    #[case(f64::INFINITY, EvalError::NonFiniteFloat)]
    #[case(f64::NEG_INFINITY, EvalError::NonFiniteFloat)]
    #[case(f64::NAN, EvalError::NonFiniteFloat)]
    #[case(f64::MIN_POSITIVE / 2.0, EvalError::SubnormalFloat)]
    fn test_eval_variable_rejects_invalid_float(#[case] value: f64, #[case] expected: EvalError) {
        let mut variables: HashMap<String, Value> = HashMap::new();
        variables.insert("Test_Name".to_string(), Value::Float(value));
        let expr = Box::new(Expression::Variable("Test_Name"));

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(expected));
    }

    #[rstest]
    #[case(Expression::Error)]
    #[case(Expression::LexicalError(crate::tokens::LexicalError::InvalidToken))]
    fn test_eval_invalid_expression(#[case] expr: Expression) {
        let variables: HashMap<String, Value> = HashMap::new();

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(EvalError::InvalidExpression));
    }

    #[rstest]
    #[case(Expression::Error)]
    #[case(Expression::LexicalError(crate::tokens::LexicalError::InvalidToken))]
    fn test_eval_function_argument_invalid_expression(#[case] argument: Expression) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::Function {
            func: Func::Min,
            arguments: vec![Expression::Integer(1), argument],
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Err(EvalError::InvalidExpression));
    }

    /************ Unary operation tests *************/

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

        assert!(matches!(result, Err(EvalError::InvalidArity { .. })));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    /************ Binary operation tests *************/

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

        assert!(matches!(result, Err(EvalError::InvalidArity { .. })));
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
    #[case(Opcode::LessThan, Expression::Integer(2), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::LessThan, Expression::Integer(1), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThan, Expression::Float(2.0), Expression::Integer(1), Value::Bool(false))]
    #[case(Opcode::LessThan, Expression::Float(1.0), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThan, Expression::Float(2.0), Expression::Float(2.0), Value::Bool(false))]
    #[case(Opcode::LessThanEquals, Expression::Integer(2), Expression::Integer(2), Value::Bool(true))]
    #[case(Opcode::LessThanEquals, Expression::Integer(2), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::LessThanEquals, Expression::Float(2.0), Expression::Integer(1), Value::Bool(false))]
    #[case(Opcode::LessThanEquals, Expression::Float(1.0), Expression::Float(2.0), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Integer(2), Expression::Integer(1), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Integer(2), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::GreaterThan, Expression::Integer(2), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Float(1.0), Expression::Integer(2), Value::Bool(false))]
    #[case(Opcode::GreaterThan, Expression::Float(2.0), Expression::Float(1.0), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Float(2.0), Expression::Float(2.0), Value::Bool(false))]
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
    #[case(Opcode::LessThan, Expression::Bool(false), Expression::Bool(false), Value::Bool(false))]
    #[case(Opcode::LessThanEquals, Expression::Bool(true), Expression::Bool(true), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Bool(true), Expression::Bool(false), Value::Bool(true))]
    #[case(Opcode::GreaterThan, Expression::Bool(false), Expression::Bool(false), Value::Bool(false))]
    #[case(Opcode::GreaterThan, Expression::Bool(true), Expression::Bool(true), Value::Bool(false))]
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

    proptest! {
        #[test]
        fn prop_eval_literals_return_their_values(value in generated_value()) {
            let expression = value_to_expression(value.clone());

            prop_assert_eq!(eval(&expression, &HashMap::new()), Ok(value));
        }

        #[test]
        fn prop_eval_known_variable_returns_bound_value(value in generated_value()) {
            let mut variables = HashMap::new();
            variables.insert("x".to_string(), value.clone());

            prop_assert_eq!(eval(&Expression::Variable("x"), &variables), Ok(value));
        }

        #[test]
        fn prop_eval_dispatch_error_paths(_unit in Just(())) {
            prop_assert_eq!(eval(&Expression::Error, &HashMap::new()), Err(EvalError::InvalidExpression));
            prop_assert_eq!(
                eval(&Expression::LexicalError(crate::tokens::LexicalError::InvalidToken), &HashMap::new()),
                Err(EvalError::InvalidExpression)
            );
            prop_assert_eq!(
                eval(&Expression::Variable("missing"), &HashMap::new()),
                Err(EvalError::UnknownVariable("missing".to_string()))
            );
            prop_assert_eq!(
                eval(
                    &Expression::UnaryOperation {
                        operator: Opcode::Plus,
                        value: Box::new(Expression::Variable("missing")),
                    },
                    &HashMap::new(),
                ),
                Err(EvalError::UnknownVariable("missing".to_string()))
            );
            prop_assert_eq!(
                eval(
                    &Expression::BinaryOperation {
                        lhs: Box::new(Expression::Variable("missing")),
                        operator: Opcode::Plus,
                        rhs: Box::new(Expression::Integer(1)),
                    },
                    &HashMap::new(),
                ),
                Err(EvalError::UnknownVariable("missing".to_string()))
            );
            prop_assert_eq!(
                eval(
                    &Expression::Function {
                        func: Func::Min,
                        arguments: vec![Expression::Integer(1), Expression::Error],
                    },
                    &HashMap::new(),
                ),
                Err(EvalError::InvalidExpression)
            );
        }

        #[test]
        fn prop_unary_error_paths(_unit in Just(())) {
            assert!(matches!(
                apply_unary(&Opcode::LogicalOr, Value::Integer(1)),
                Err(EvalError::InvalidArity { .. })
            ));
            assert!(matches!(
                apply_unary_math(&Opcode::Plus, Value::Bool(true)),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_unary_math(&Opcode::Equals, Value::Integer(1)),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_bitwise_not(Value::Float(1.0)),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_logical_not(Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
        }

        #[test]
        fn prop_integer_unary_math_matches_rust(value in small_i64()) {
            prop_assert_eq!(
                apply_unary(&Opcode::Plus, Value::Integer(value)),
                Ok(Value::Integer(value))
            );
            prop_assert_eq!(
                apply_unary(&Opcode::Minus, Value::Integer(value)),
                Ok(Value::Integer(-value))
            );
            prop_assert_eq!(
                apply_unary(&Opcode::Degrees, Value::Integer(value)),
                Ok(Value::Float((value as f64).to_radians()))
            );
        }

        #[test]
        fn prop_float_unary_math_matches_rust(value in small_f64()) {
            prop_assert_eq!(
                apply_unary(&Opcode::Plus, Value::Float(value)),
                checked_float(value)
            );
            prop_assert_eq!(
                apply_unary(&Opcode::Minus, Value::Float(value)),
                checked_float(-value)
            );
            prop_assert_eq!(
                apply_unary(&Opcode::Degrees, Value::Float(value)),
                checked_float(value.to_radians())
            );
        }

        #[test]
        fn prop_bitwise_and_logical_not_match_rust(value in any::<bool>(), integer in any::<i64>()) {
            prop_assert_eq!(
                apply_unary(&Opcode::BitwiseNot, Value::Bool(value)),
                Ok(Value::Bool(!value))
            );
            prop_assert_eq!(
                apply_unary(&Opcode::LogicalNot, Value::Bool(value)),
                Ok(Value::Bool(!value))
            );
            prop_assert_eq!(
                apply_unary(&Opcode::BitwiseNot, Value::Integer(integer)),
                Ok(Value::Integer(!integer))
            );
        }

        #[test]
        fn prop_integer_binary_math_matches_rust(lhs in tiny_i64(), rhs in nonzero_tiny_i64()) {
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Plus, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs + rhs))
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Minus, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs - rhs))
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Multiply, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs * rhs))
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Divide, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs / rhs))
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Modulo, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs % rhs))
            );
        }

        #[test]
        fn prop_integer_power_matches_checked_pow(base in -10i64..10, exponent in 0i64..10) {
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Power, Value::Integer(base), Value::Integer(exponent)),
                base.checked_pow(exponent as u32)
                    .map(Value::Integer)
                    .ok_or_else(|| EvalError::IntegerOverflow { op: ArithmeticOp::Power })
            );
        }

        #[test]
        fn prop_float_binary_math_matches_rust(lhs in tiny_f64(), rhs in nonzero_tiny_f64()) {
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Plus, Value::Float(lhs), Value::Float(rhs)),
                checked_float(lhs + rhs)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Minus, Value::Float(lhs), Value::Float(rhs)),
                checked_float(lhs - rhs)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Multiply, Value::Float(lhs), Value::Float(rhs)),
                checked_float(lhs * rhs)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Divide, Value::Float(lhs), Value::Float(rhs)),
                checked_float(lhs / rhs)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Modulo, Value::Float(lhs), Value::Float(rhs)),
                checked_float(lhs % rhs)
            );
        }

        #[test]
        fn prop_mixed_numeric_binary_math_promotes_to_float(lhs in tiny_i64(), rhs in nonzero_tiny_f64()) {
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Plus, Value::Integer(lhs), Value::Float(rhs)),
                checked_float(lhs as f64 + rhs)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Multiply, Value::Float(rhs), Value::Integer(lhs)),
                checked_float(rhs * lhs as f64)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Divide, Value::Integer(lhs), Value::Float(rhs)),
                checked_float(lhs as f64 / rhs)
            );
        }

        #[test]
        fn prop_binary_error_paths(_unit in Just(())) {
            prop_assert_eq!(
                apply_binary(&Opcode::ApproximatelyEquals, Value::Integer(1), Value::Integer(1)),
                Ok(Value::Bool(true))
            );
            prop_assert_eq!(
                apply_binary(&Opcode::BitwiseAnd, Value::Integer(1), Value::Integer(3)),
                Ok(Value::Integer(1))
            );
            prop_assert_eq!(
                apply_binary(&Opcode::BitshiftLeft, Value::Integer(1), Value::Integer(2)),
                Ok(Value::Integer(4))
            );
            prop_assert_eq!(
                apply_binary(&Opcode::LogicalAnd, Value::Bool(true), Value::Bool(false)),
                Ok(Value::Bool(false))
            );
            assert!(matches!(
                apply_binary(&Opcode::Degrees, Value::Integer(1), Value::Integer(1)),
                Err(EvalError::InvalidArity { .. })
            ));
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::Plus, Value::Integer(1), Value::Integer(1)),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_binary_comparison(&Opcode::Equals, Value::Integer(1), Value::Bool(true)),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Divide, Value::Integer(1), Value::Integer(0)),
                Err(EvalError::DivisionByZero)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Modulo, Value::Integer(1), Value::Integer(0)),
                Err(EvalError::DivisionByZero)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Divide, Value::Float(1.0), Value::Float(0.0)),
                Err(EvalError::DivisionByZero)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Modulo, Value::Float(1.0), Value::Float(0.0)),
                Err(EvalError::DivisionByZero)
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Power, Value::Float(2.0), Value::Float(3.0)),
                Ok(Value::Float(8.0))
            );
            prop_assert_eq!(
                apply_binary_math_operation(&Opcode::Equals, Value::Integer(1), Value::Integer(1)),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_binary_math_operation(&Opcode::Plus, Value::Bool(true), Value::Bool(false)),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::Plus, Value::Integer(1), Value::Integer(1)),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_binary_bit_operation(&Opcode::BitwiseAnd, Value::Float(1.0), Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_binary_bit_operation(&Opcode::BitwiseAnd, Value::Bool(true), Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_bitshift_operation(&Opcode::Plus, Value::Integer(1), Value::Integer(1)),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_bitshift_operation(&Opcode::BitshiftLeft, Value::Bool(true), Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_binary_logical_operation(&Opcode::Plus, Value::Bool(true), Value::Bool(false)),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_binary_logical_operation(&Opcode::LogicalAnd, Value::Integer(1), Value::Bool(false)),
                Err(EvalError::InvalidType { .. })
            ));
        }

        #[test]
        fn prop_integer_comparisons_match_rust_ordering(lhs in any::<i64>(), rhs in any::<i64>()) {
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::LessThan, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs < rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::LessThanEquals, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs <= rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::GreaterThan, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs > rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::GreaterThanEquals, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs >= rhs))
            );
        }

        #[test]
        fn prop_integer_equality_comparisons_match_rust(lhs in any::<i64>(), rhs in any::<i64>()) {
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::Equals, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs == rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::NotEquals, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs != rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::ApproximatelyEquals, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Bool(lhs == rhs))
            );
        }

        #[test]
        fn prop_bool_comparisons_match_rust_ordering(lhs in any::<bool>(), rhs in any::<bool>()) {
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::LessThan, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(!lhs & rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::LessThanEquals, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs <= rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::GreaterThan, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs & !rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::GreaterThanEquals, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs >= rhs))
            );
        }

        #[test]
        fn prop_bool_equality_comparisons_match_rust(lhs in any::<bool>(), rhs in any::<bool>()) {
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::Equals, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs == rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::NotEquals, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs != rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::ApproximatelyEquals, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs == rhs))
            );
        }

        #[test]
        fn prop_float_comparisons_match_rust_ordering(lhs in small_f64(), rhs in small_f64()) {
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::Equals, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool(lhs == rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::NotEquals, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool(lhs != rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::LessThan, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool(lhs < rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::LessThanEquals, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool(lhs <= rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::GreaterThan, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool(lhs > rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::GreaterThanEquals, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool(lhs >= rhs))
            );
            prop_assert_eq!(
                apply_binary_comparison(&Opcode::ApproximatelyEquals, Value::Float(lhs), Value::Float(rhs)),
                Ok(Value::Bool({
                    let scale = lhs.abs().max(rhs.abs()).max(1.0);
                    (lhs - rhs).abs() <= EPSILON * scale
                }))
            );
        }

        #[test]
        fn prop_bitwise_integer_operations_match_rust(lhs in any::<i64>(), rhs in any::<i64>()) {
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::BitwiseAnd, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs & rhs))
            );
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::BitwiseOr, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs | rhs))
            );
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::BitwiseXor, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs ^ rhs))
            );
        }

        #[test]
        fn prop_bitwise_bool_operations_match_rust(lhs in any::<bool>(), rhs in any::<bool>()) {
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::BitwiseAnd, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs & rhs))
            );
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::BitwiseOr, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs | rhs))
            );
            prop_assert_eq!(
                apply_binary_bit_operation(&Opcode::BitwiseXor, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs ^ rhs))
            );
        }

        #[test]
        fn prop_bitshift_operations_match_rust(lhs in any::<i64>(), rhs in 0i64..63) {
            prop_assert_eq!(
                apply_bitshift_operation(&Opcode::BitshiftLeft, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs << rhs))
            );
            prop_assert_eq!(
                apply_bitshift_operation(&Opcode::BitshiftRight, Value::Integer(lhs), Value::Integer(rhs)),
                Ok(Value::Integer(lhs >> rhs))
            );
        }

        #[test]
        fn prop_logical_binary_operations_match_rust(lhs in any::<bool>(), rhs in any::<bool>()) {
            prop_assert_eq!(
                apply_binary_logical_operation(&Opcode::LogicalAnd, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs && rhs))
            );
            prop_assert_eq!(
                apply_binary_logical_operation(&Opcode::LogicalOr, Value::Bool(lhs), Value::Bool(rhs)),
                Ok(Value::Bool(lhs || rhs))
            );
        }

        #[test]
        fn prop_min_max_integer_vectors_match_rust(values in proptest::collection::vec(small_i64(), 1..32)) {
            let arguments = values.iter().copied().map(Value::Integer).collect::<Vec<_>>();
            let min = values.iter().copied().min().unwrap();
            let max = values.iter().copied().max().unwrap();

            prop_assert_eq!(apply_function(&Func::Min, arguments.clone()), Ok(Value::Integer(min)));
            prop_assert_eq!(apply_function(&Func::Max, arguments), Ok(Value::Integer(max)));
        }

        #[test]
        fn prop_min_max_mixed_numeric_vectors_promote_to_float(
            integers in proptest::collection::vec(small_i64(), 1..16),
            float in small_f64(),
        ) {
            let mut arguments = integers.iter().copied().map(Value::Integer).collect::<Vec<_>>();
            arguments.push(Value::Float(float));
            let expected_values = integers
                .iter()
                .copied()
                .map(|value| value as f64)
                .chain(std::iter::once(float))
                .collect::<Vec<_>>();
            let min = expected_values
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let max = expected_values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);

            prop_assert_eq!(apply_function(&Func::Min, arguments.clone()), Ok(Value::Float(min)));
            prop_assert_eq!(apply_function(&Func::Max, arguments), Ok(Value::Float(max)));
        }

        #[test]
        fn prop_function_validation_error_paths(_unit in Just(())) {
            assert!(matches!(apply_function(&Func::Min, vec![]), Err(EvalError::InvalidArity { .. })));
            assert!(matches!(
                apply_function(&Func::Min, vec![Value::Bool(true)]),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_n_nary_function(&Func::Power, vec![Value::Integer(1), Value::Integer(2)]),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_min_function(vec![Value::Integer(1), Value::Float(1.0)]),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_min_function(vec![Value::Float(1.0), Value::Integer(1)]),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_min_function(vec![Value::Bool(true)]),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_max_function(vec![Value::Integer(1), Value::Float(1.0)]),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_max_function(vec![Value::Float(1.0), Value::Integer(1)]),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_max_function(vec![Value::Bool(true)]),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_rounding_function(&Func::Min, vec![Value::Integer(1)]),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                pare_vector_rounding(vec![Value::Bool(true)]),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                pare_vector_rounding(vec![Value::Integer(1), Value::Integer(1), Value::Integer(1)]),
                Err(EvalError::InvalidArity { .. })
            ));
            assert!(matches!(
                apply_round_function(Value::Bool(true), None),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_floor_function(Value::Bool(true), None),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_ceiling_function(Value::Bool(true), None),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_function(&Func::Round, vec![Value::Integer(11), Value::Integer(10)]),
                Ok(Value::Integer(10))
            );
            prop_assert_eq!(
                apply_function(&Func::Floor, vec![Value::Integer(11), Value::Integer(10)]),
                Ok(Value::Integer(10))
            );
            prop_assert_eq!(
                apply_function(&Func::Ceiling, vec![Value::Integer(11), Value::Integer(10)]),
                Ok(Value::Integer(20))
            );
            prop_assert_eq!(
                apply_rounding_function(&Func::Round, vec![Value::Integer(11), Value::Integer(10)]),
                Ok(Value::Integer(10))
            );
            prop_assert_eq!(
                apply_rounding_function(&Func::Floor, vec![Value::Integer(11), Value::Integer(10)]),
                Ok(Value::Integer(10))
            );
            prop_assert_eq!(
                apply_rounding_function(&Func::Ceiling, vec![Value::Integer(11), Value::Integer(10)]),
                Ok(Value::Integer(20))
            );
            prop_assert_eq!(
                pare_vector_rounding(vec![Value::Integer(11), Value::Integer(10)]),
                Ok((Value::Integer(11), Some(Value::Integer(10))))
            );
            prop_assert_eq!(
                apply_binary_function(&Func::Min, vec![Value::Integer(1), Value::Integer(2)]),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_binary_function(&Func::Power, vec![Value::Integer(1)]),
                Err(EvalError::InvalidArity { .. })
            ));
            assert!(matches!(
                apply_binary_function(&Func::Power, vec![Value::Bool(true), Value::Integer(2)]),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_unary_function(&Func::Min, vec![Value::Integer(1)]),
                Err(EvalError::UnexpectedOpcode)
            );
            assert!(matches!(
                apply_unary_function(&Func::Cos, vec![]),
                Err(EvalError::InvalidArity { .. })
            ));
            assert!(matches!(
                apply_unary_function(&Func::Cos, vec![Value::Bool(true)]),
                Err(EvalError::InvalidType { .. })
            ));
            prop_assert_eq!(
                apply_float_unary(Value::Integer(1), f64::cos),
                Err(EvalError::UnexpectedOpcode)
            );
        }

        #[test]
        fn prop_integer_rounding_functions_match_expected_precision(value in small_i64(), precision in 1i64..1_000) {
            let floored = value.div_euclid(precision) * precision;
            let ceiling = if floored == value { floored } else { floored + precision };
            let rounded = value.try_round_to(precision, Tie::Up).unwrap();

            prop_assert_eq!(
                apply_round_function(Value::Integer(value), Some(Value::Integer(precision))),
                Ok(Value::Integer(rounded))
            );
            prop_assert_eq!(
                apply_floor_function(Value::Integer(value), Some(Value::Integer(precision))),
                Ok(Value::Integer(floored))
            );
            prop_assert_eq!(
                apply_ceiling_function(Value::Integer(value), Some(Value::Integer(precision))),
                Ok(Value::Integer(ceiling))
            );
        }

        #[test]
        fn prop_rounding_functions_cover_no_precision_and_float_precision(value in -10_000.0f64..10_000.0, precision in 0.1f64..100.0) {
            let integer = value as i64;

            prop_assert_eq!(
                apply_round_function(Value::Integer(integer), None),
                round_i64(integer, 1).map(Value::Integer)
            );
            prop_assert_eq!(
                apply_floor_function(Value::Integer(integer), None),
                floor_i64(integer, 1).map(Value::Integer)
            );
            prop_assert_eq!(
                apply_ceiling_function(Value::Integer(integer), None),
                ceiling_i64(integer, 1).map(Value::Integer)
            );
            prop_assert_eq!(
                apply_round_function(Value::Float(value), None),
                round_f64(value, 1.0).map(Value::Float)
            );
            prop_assert_eq!(
                apply_floor_function(Value::Float(value), None),
                floor_f64(value, 1.0).map(Value::Float)
            );
            prop_assert_eq!(
                apply_ceiling_function(Value::Float(value), None),
                ceiling_f64(value, 1.0).map(Value::Float)
            );
            prop_assert_eq!(
                apply_round_function(Value::Integer(value as i64), Some(Value::Float(precision))),
                round_f64(value as i64 as f64, precision).map(Value::Float)
            );
            prop_assert_eq!(
                apply_floor_function(Value::Integer(value as i64), Some(Value::Float(precision))),
                floor_f64(value as i64 as f64, precision).map(Value::Float)
            );
            prop_assert_eq!(
                apply_ceiling_function(Value::Integer(value as i64), Some(Value::Float(precision))),
                ceiling_f64(value as i64 as f64, precision).map(Value::Float)
            );
            prop_assert_eq!(
                apply_round_function(Value::Float(value), Some(Value::Float(precision))),
                round_f64(value, precision).map(Value::Float)
            );
            prop_assert_eq!(
                apply_floor_function(Value::Float(value), Some(Value::Float(precision))),
                floor_f64(value, precision).map(Value::Float)
            );
            prop_assert_eq!(
                apply_ceiling_function(Value::Float(value), Some(Value::Float(precision))),
                ceiling_f64(value, precision).map(Value::Float)
            );
            prop_assert_eq!(
                apply_round_function(Value::Float(value), Some(Value::Integer(1))),
                round_f64_to_i64(value, 1).map(Value::Integer)
            );
            prop_assert_eq!(
                apply_floor_function(Value::Float(value), Some(Value::Integer(1))),
                floor_f64_to_i64(value, 1).map(Value::Integer)
            );
            prop_assert_eq!(
                apply_ceiling_function(Value::Float(value), Some(Value::Integer(1))),
                ceiling_f64_to_i64(value, 1).map(Value::Integer)
            );
        }

        #[test]
        fn prop_rounding_functions_reject_nonpositive_integer_precision(value in small_i64(), precision in -1_000i64..=0) {
            assert!(matches!(
                apply_round_function(Value::Integer(value), Some(Value::Integer(precision))),
                Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })
            ));
            assert!(matches!(
                apply_floor_function(Value::Integer(value), Some(Value::Integer(precision))),
                Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })
            ));
            assert!(matches!(
                apply_ceiling_function(Value::Integer(value), Some(Value::Integer(precision))),
                Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })
            ));
        }

        #[test]
        fn prop_float_rounding_rejects_nonpositive_precision(value in small_f64(), precision in -1_000.0f64..=0.0) {
            assert!(matches!(round_f64(value, precision), Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })));
            assert!(matches!(floor_f64(value, precision), Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })));
            assert!(matches!(ceiling_f64(value, precision), Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })));
            assert!(matches!(round_f64(f64::INFINITY, 1.0), Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })));
            assert!(matches!(floor_f64(f64::INFINITY, 1.0), Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })));
            assert!(matches!(ceiling_f64(f64::INFINITY, 1.0), Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })));
        }

        #[test]
        fn prop_float_to_integer_conversion_checks_bounds(value in small_f64()) {
            prop_assert_eq!(checked_f64_to_i64(value, "test"), Ok(value as i64));
            assert!(matches!(
                checked_f64_to_i64(f64::INFINITY, "test"),
                Err(EvalError::IntegerOverflow { .. } | EvalError::InvalidPrecision | EvalError::NonFiniteFloat | EvalError::SubnormalFloat | EvalError::DivisionByZero | EvalError::InvalidExponent { .. })
            ));
        }

        #[test]
        fn prop_binary_functions_match_rust(lhs in tiny_i64(), rhs in nonzero_tiny_i64()) {
            prop_assert_eq!(
                apply_function(&Func::Modulo, vec![Value::Integer(lhs), Value::Integer(rhs)]),
                Ok(Value::Integer(lhs % rhs))
            );
            prop_assert_eq!(
                apply_function(&Func::Remainder, vec![Value::Integer(lhs), Value::Integer(rhs)]),
                Ok(Value::Integer(lhs.rem_euclid(rhs)))
            );
        }

        #[test]
        fn prop_binary_functions_mixed_numeric_and_invalid_types(lhs in tiny_i64(), rhs in nonzero_tiny_f64()) {
            let integer_power_base = lhs.abs();
            let float_power_base = rhs.abs();

            prop_assert_eq!(
                apply_function(&Func::Power, vec![Value::Integer(integer_power_base), Value::Float(rhs)]),
                checked_float((integer_power_base as f64).powf(rhs))
            );
            prop_assert_eq!(
                apply_function(&Func::Power, vec![Value::Float(float_power_base), Value::Integer(lhs)]),
                checked_float(float_power_base.powf(lhs as f64))
            );
            prop_assert_eq!(
                apply_function(&Func::Power, vec![Value::Float(float_power_base), Value::Float(rhs)]),
                checked_float(float_power_base.powf(rhs))
            );
            prop_assert_eq!(
                apply_function(&Func::Modulo, vec![Value::Integer(lhs), Value::Float(rhs)]),
                checked_float((lhs as f64) % rhs)
            );
            prop_assert_eq!(
                apply_function(&Func::Modulo, vec![Value::Float(rhs), Value::Integer(1)]),
                checked_float(rhs % 1.0)
            );
            prop_assert_eq!(
                apply_function(&Func::Modulo, vec![Value::Float(rhs), Value::Float(rhs)]),
                checked_float(rhs % rhs)
            );
            prop_assert_eq!(
                apply_function(&Func::Remainder, vec![Value::Integer(lhs), Value::Float(rhs)]),
                checked_float((lhs as f64).rem_euclid(rhs))
            );
            prop_assert_eq!(
                apply_function(&Func::Remainder, vec![Value::Float(rhs), Value::Integer(1)]),
                checked_float(rhs.rem_euclid(1.0))
            );
            prop_assert_eq!(
                apply_function(&Func::Remainder, vec![Value::Float(rhs), Value::Float(rhs)]),
                checked_float(rhs.rem_euclid(rhs))
            );
            assert!(matches!(
                apply_power_function(Value::Bool(true), Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_modulo_function(Value::Bool(true), Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
            assert!(matches!(
                apply_remainder_function(Value::Bool(true), Value::Integer(1)),
                Err(EvalError::InvalidType { .. })
            ));
        }

        #[test]
        fn prop_power_function_matches_checked_pow(base in -10i64..10, exponent in 0i64..10) {
            prop_assert_eq!(
                apply_function(&Func::Power, vec![Value::Integer(base), Value::Integer(exponent)]),
                base.checked_pow(exponent as u32)
                    .map(Value::Integer)
                    .ok_or_else(|| EvalError::IntegerOverflow { op: ArithmeticOp::Power })
            );
        }

        #[test]
        fn prop_unary_float_functions_match_rust(value in -10.0f64..10.0) {
            prop_assert_eq!(apply_function(&Func::Cos, vec![Value::Float(value)]), checked_float(value.cos()));
            prop_assert_eq!(apply_function(&Func::Sin, vec![Value::Float(value)]), checked_float(value.sin()));
            prop_assert_eq!(apply_function(&Func::Tan, vec![Value::Float(value)]), checked_float(value.tan()));
            prop_assert_eq!(apply_function(&Func::ATan, vec![Value::Float(value)]), checked_float(value.atan()));
            prop_assert_eq!(apply_function(&Func::Abs, vec![Value::Float(value)]), checked_float(value.abs()));
            prop_assert_eq!(apply_function(&Func::Exp, vec![Value::Float(value)]), checked_float(value.exp()));
        }

        #[test]
        fn prop_unary_domain_limited_float_functions_match_rust(unit in unit_f64(), positive in positive_f64()) {
            prop_assert_eq!(apply_function(&Func::ACos, vec![Value::Float(unit)]), checked_float(unit.acos()));
            prop_assert_eq!(apply_function(&Func::ASin, vec![Value::Float(unit)]), checked_float(unit.asin()));
            prop_assert_eq!(apply_function(&Func::Ln, vec![Value::Float(positive)]), checked_float(positive.ln()));
            prop_assert_eq!(apply_function(&Func::Log, vec![Value::Float(positive)]), checked_float(positive.log10()));
        }
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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
            Err(EvalError::InvalidExponent {
                exponent: 5_000_000_000
            })
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
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Power
            })
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

        assert_eq!(result, Err(EvalError::DivisionByZero));
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

        assert_eq!(result, Err(EvalError::DivisionByZero));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
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

    /************ N-nary function tests *************/

    #[rstest]
    #[case(
        Func::Min,
        vec![Value::Integer(3), Value::Integer(-1), Value::Integer(2)],
        Value::Integer(-1)
    )]
    #[case(
        Func::Max,
        vec![Value::Integer(3), Value::Integer(-1), Value::Integer(2)],
        Value::Integer(3)
    )]
    #[case(
        Func::Min,
        vec![Value::Float(3.5), Value::Float(-1.25), Value::Float(2.0)],
        Value::Float(-1.25)
    )]
    #[case(
        Func::Max,
        vec![Value::Float(3.5), Value::Float(-1.25), Value::Float(2.0)],
        Value::Float(3.5)
    )]
    #[case(
        Func::Min,
        vec![Value::Integer(3), Value::Float(-1.25), Value::Integer(2)],
        Value::Float(-1.25)
    )]
    #[case(
        Func::Max,
        vec![Value::Integer(3), Value::Float(-1.25), Value::Integer(2)],
        Value::Float(3.0)
    )]
    #[case(Func::Min, vec![Value::Integer(3)], Value::Integer(3))]
    #[case(Func::Max, vec![Value::Float(3.5)], Value::Float(3.5))]
    fn test_eval_n_nary_function_regular(
        #[case] func: Func,
        #[case] arguments: Vec<Value>,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let arguments = arguments.into_iter().map(value_to_expression).collect();
        let expr = Box::new(Expression::Function { func, arguments });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Func::Min)]
    #[case(Func::Max)]
    fn test_apply_n_nary_function_empty_invalid_arity(#[case] func: Func) {
        let result = apply_n_nary_function(&func, vec![]);

        assert!(matches!(result, Err(EvalError::InvalidArity { .. })));
    }

    #[rstest]
    #[case(Func::Min)]
    #[case(Func::Max)]
    fn test_apply_n_nary_function_invalid_type_from_validation(#[case] func: Func) {
        let result = apply_n_nary_function(&func, vec![Value::Integer(1), Value::Bool(true)]);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(Func::Power)]
    #[case(Func::Modulo)]
    #[case(Func::Remainder)]
    #[case(Func::Round)]
    #[case(Func::Floor)]
    #[case(Func::Ceiling)]
    #[case(Func::Cos)]
    #[case(Func::Sin)]
    #[case(Func::Tan)]
    #[case(Func::ACos)]
    #[case(Func::ASin)]
    #[case(Func::ATan)]
    #[case(Func::Abs)]
    #[case(Func::Ln)]
    #[case(Func::Log)]
    #[case(Func::Exp)]
    fn test_apply_n_nary_function_unsupported_func(#[case] func: Func) {
        let result = apply_n_nary_function(&func, vec![Value::Integer(1)]);

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[rstest]
    #[case(vec![], Ok(vec![]))]
    #[case(
        vec![Value::Integer(1)],
        Ok(vec![Value::Integer(1)])
    )]
    #[case(
        vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
        Ok(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)])
    )]
    #[case(
        vec![Value::Float(1.5)],
        Ok(vec![Value::Float(1.5)])
    )]
    #[case(
        vec![Value::Integer(1), Value::Float(2.5), Value::Integer(3)],
        Ok(vec![Value::Float(1.0), Value::Float(2.5), Value::Float(3.0)])
    )]
    #[case(
        vec![Value::Bool(true)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    #[case(
        vec![Value::Integer(1), Value::Bool(true), Value::Float(3.0)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    fn test_pare_vector_n_nary(
        #[case] vals: Vec<Value>,
        #[case] expected: Result<Vec<Value>, EvalError>,
    ) {
        let result = pare_vector_n_nary(vals);

        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(
        vec![Value::Integer(1), Value::Float(2.0)],
        "Min expected all integer values"
    )]
    #[case(
        vec![Value::Integer(1), Value::Bool(true)],
        "Min expected all integer values"
    )]
    #[case(
        vec![Value::Float(1.0), Value::Integer(2)],
        "Min expected all float values"
    )]
    #[case(
        vec![Value::Float(1.0), Value::Bool(true)],
        "Min expected all float values"
    )]
    #[case(
        vec![Value::Bool(true), Value::Integer(1)],
        "N-nary functions not defined for bool"
    )]
    fn test_apply_min_function_invalid_type(#[case] vals: Vec<Value>, #[case] _message: &str) {
        let result = apply_min_function(vals);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(
        vec![Value::Integer(1), Value::Float(2.0)],
        "Max expected all integer values"
    )]
    #[case(
        vec![Value::Integer(1), Value::Bool(true)],
        "Max expected all integer values"
    )]
    #[case(
        vec![Value::Float(1.0), Value::Integer(2)],
        "Max expected all float values"
    )]
    #[case(
        vec![Value::Float(1.0), Value::Bool(true)],
        "Max expected all float values"
    )]
    #[case(
        vec![Value::Bool(true), Value::Integer(1)],
        "N-nary functions not defined for bool"
    )]
    fn test_apply_max_function_invalid_type(#[case] vals: Vec<Value>, #[case] _message: &str) {
        let result = apply_max_function(vals);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    /************ Rounding function tests *************/

    #[rstest]
    #[case(Func::Min)]
    #[case(Func::Max)]
    #[case(Func::Power)]
    #[case(Func::Modulo)]
    #[case(Func::Remainder)]
    #[case(Func::Cos)]
    #[case(Func::Sin)]
    #[case(Func::Tan)]
    #[case(Func::ACos)]
    #[case(Func::ASin)]
    #[case(Func::ATan)]
    #[case(Func::Abs)]
    #[case(Func::Ln)]
    #[case(Func::Log)]
    #[case(Func::Exp)]
    fn test_apply_rounding_function_unsupported_func(#[case] func: Func) {
        let result = apply_rounding_function(&func, vec![Value::Integer(1)]);

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[test]
    fn test_apply_rounding_function_invalid_arguments() {
        let result = apply_rounding_function(&Func::Round, vec![]);

        assert!(matches!(result, Err(EvalError::InvalidArity { .. })));
    }

    #[rstest]
    #[case(
        vec![Value::Integer(1)],
        Ok((Value::Integer(1), None))
    )]
    #[case(
        vec![Value::Float(1.5)],
        Ok((Value::Float(1.5), None))
    )]
    #[case(
        vec![Value::Integer(1), Value::Integer(2)],
        Ok((Value::Integer(1), Some(Value::Integer(2))))
    )]
    #[case(
        vec![Value::Integer(1), Value::Float(2.5)],
        Ok((Value::Integer(1), Some(Value::Float(2.5))))
    )]
    #[case(
        vec![Value::Float(1.5), Value::Integer(2)],
        Ok((Value::Float(1.5), Some(Value::Integer(2))))
    )]
    #[case(
        vec![Value::Float(1.5), Value::Float(2.5)],
        Ok((Value::Float(1.5), Some(Value::Float(2.5))))
    )]
    #[case(vec![], Err(EvalError::InvalidArity { expected: "1 or 2", actual: 0 }))]
    #[case(
        vec![Value::Integer(1), Value::Float(2.0), Value::Float(3.0)],
        Err(EvalError::InvalidArity { expected: "1 or 2", actual: 3 })
    )]
    #[case(
        vec![Value::Bool(true)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    #[case(
        vec![Value::Bool(true), Value::Float(2.0)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    #[case(
        vec![Value::Integer(1), Value::Bool(true)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    fn test_pare_vector_rounding(
        #[case] vals: Vec<Value>,
        #[case] expected: Result<(Value, Option<Value>), EvalError>,
    ) {
        let result = pare_vector_rounding(vals);

        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(Value::Integer(314), None, Value::Integer(314))]
    #[case(Value::Float(314.1), None, Value::Float(314.0))]
    #[case(Value::Integer(314), Some(Value::Integer(10)), Value::Integer(310))]
    #[case(Value::Integer(314), Some(Value::Float(100.0)), Value::Float(300.0))]
    #[case(Value::Float(314.1), Some(Value::Integer(10)), Value::Integer(310))]
    #[case(Value::Float(314.1), Some(Value::Float(100.0)), Value::Float(300.0))]
    fn test_eval_round_function_regular(
        #[case] value: Value,
        #[case] precision: Option<Value>,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let mut arguments = vec![value_to_expression(value)];
        if let Some(precision) = precision {
            arguments.push(value_to_expression(precision));
        }
        let expr = Box::new(Expression::Function {
            func: Func::Round,
            arguments,
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Value::Integer(314), Some(Value::Integer(0)))]
    #[case(Value::Integer(314), Some(Value::Float(0.0)))]
    #[case(Value::Float(314.1), Some(Value::Integer(0)))]
    #[case(Value::Float(314.1), Some(Value::Float(0.0)))]
    #[case(Value::Integer(314), Some(Value::Integer(-10)))]
    #[case(Value::Float(314.1), Some(Value::Float(-10.0)))]
    fn test_apply_round_function_non_positive_precision(
        #[case] value: Value,
        #[case] precision: Option<Value>,
    ) {
        let result = apply_round_function(value, precision);

        assert_eq!(result, Err(EvalError::InvalidPrecision));
    }

    #[test]
    fn test_apply_round_function_float_to_integer_overflow() {
        let result = apply_round_function(Value::Float(f64::MAX), Some(Value::Integer(10)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::FloatToInteger
            })
        );
    }

    #[rstest]
    #[case(
        apply_round_function as fn(Value, Option<Value>) -> Result<Value, EvalError>,
        "round"
    )]
    #[case(
        apply_floor_function as fn(Value, Option<Value>) -> Result<Value, EvalError>,
        "floor"
    )]
    #[case(
        apply_ceiling_function as fn(Value, Option<Value>) -> Result<Value, EvalError>,
        "ceiling"
    )]
    fn test_float_to_integer_conversion_negative_overflow(
        #[case] apply: fn(Value, Option<Value>) -> Result<Value, EvalError>,
        #[case] _operation: &str,
    ) {
        let result = apply(Value::Float(f64::MIN), Some(Value::Integer(1)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::FloatToInteger
            })
        );
    }

    #[rstest]
    #[case(
        apply_round_function as fn(Value, Option<Value>) -> Result<Value, EvalError>,
        "round"
    )]
    #[case(
        apply_floor_function as fn(Value, Option<Value>) -> Result<Value, EvalError>,
        "floor"
    )]
    #[case(
        apply_ceiling_function as fn(Value, Option<Value>) -> Result<Value, EvalError>,
        "ceiling"
    )]
    fn test_float_to_integer_conversion_upper_bound_overflow(
        #[case] apply: fn(Value, Option<Value>) -> Result<Value, EvalError>,
        #[case] _operation: &str,
    ) {
        let result = apply(Value::Float(i64::MAX as f64), Some(Value::Integer(1)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::FloatToInteger
            })
        );
    }

    #[rstest]
    #[case(
        apply_round_function as fn(Value, Option<Value>) -> Result<Value, EvalError>
    )]
    #[case(
        apply_floor_function as fn(Value, Option<Value>) -> Result<Value, EvalError>
    )]
    #[case(
        apply_ceiling_function as fn(Value, Option<Value>) -> Result<Value, EvalError>
    )]
    fn test_float_to_integer_conversion_lower_bound_allowed(
        #[case] apply: fn(Value, Option<Value>) -> Result<Value, EvalError>,
    ) {
        let result = apply(Value::Float(i64::MIN as f64), Some(Value::Integer(1)));

        assert_eq!(result, Ok(Value::Integer(i64::MIN)));
    }

    #[test]
    fn test_round_f64_float_overflow() {
        let result = round_f64(f64::INFINITY, 0.5);

        assert_eq!(result, Err(EvalError::NonFiniteFloat));
    }

    #[test]
    fn test_apply_round_function_integer_overflow() {
        let result = apply_round_function(Value::Integer(i64::MAX), Some(Value::Integer(10)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Round
            })
        );
    }

    #[rstest]
    #[case(Value::Bool(true), None)]
    #[case(Value::Bool(true), Some(Value::Integer(2)))]
    #[case(Value::Bool(true), Some(Value::Float(2.0)))]
    #[case(Value::Integer(2), Some(Value::Bool(true)))]
    #[case(Value::Float(2.0), Some(Value::Bool(true)))]
    fn test_apply_round_function_bool_invalid_type(
        #[case] value: Value,
        #[case] precision: Option<Value>,
    ) {
        let result = apply_round_function(value, precision);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(Value::Integer(314), None, Value::Integer(314))]
    #[case(Value::Float(314.9), None, Value::Float(314.0))]
    #[case(Value::Integer(314), Some(Value::Integer(10)), Value::Integer(310))]
    #[case(Value::Integer(314), Some(Value::Float(100.0)), Value::Float(300.0))]
    #[case(Value::Float(314.9), Some(Value::Integer(10)), Value::Integer(310))]
    #[case(Value::Float(314.9), Some(Value::Float(100.0)), Value::Float(300.0))]
    fn test_eval_floor_function_regular(
        #[case] value: Value,
        #[case] precision: Option<Value>,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let mut arguments = vec![value_to_expression(value)];
        if let Some(precision) = precision {
            arguments.push(value_to_expression(precision));
        }
        let expr = Box::new(Expression::Function {
            func: Func::Floor,
            arguments,
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Value::Bool(true), None)]
    #[case(Value::Bool(true), Some(Value::Integer(2)))]
    #[case(Value::Bool(true), Some(Value::Float(2.0)))]
    #[case(Value::Integer(2), Some(Value::Bool(true)))]
    #[case(Value::Float(2.0), Some(Value::Bool(true)))]
    fn test_apply_floor_function_bool_invalid_type(
        #[case] value: Value,
        #[case] precision: Option<Value>,
    ) {
        let result = apply_floor_function(value, precision);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(Value::Integer(314), Some(Value::Integer(0)))]
    #[case(Value::Integer(314), Some(Value::Float(0.0)))]
    #[case(Value::Float(314.1), Some(Value::Integer(0)))]
    #[case(Value::Float(314.1), Some(Value::Float(0.0)))]
    #[case(Value::Integer(314), Some(Value::Integer(-10)))]
    #[case(Value::Float(314.1), Some(Value::Float(-10.0)))]
    fn test_apply_floor_function_non_positive_precision(
        #[case] value: Value,
        #[case] precision: Option<Value>,
    ) {
        let result = apply_floor_function(value, precision);

        assert_eq!(result, Err(EvalError::InvalidPrecision));
    }

    #[test]
    fn test_apply_floor_function_float_to_integer_overflow() {
        let result = apply_floor_function(Value::Float(f64::MAX), Some(Value::Integer(10)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::FloatToInteger
            })
        );
    }

    #[test]
    fn test_floor_i64_integer_overflow() {
        let result = floor_i64(i64::MIN, 3);

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Floor
            })
        );
    }

    #[test]
    fn test_floor_f64_float_overflow() {
        let result = floor_f64(f64::MAX, 0.5);

        assert_eq!(result, Err(EvalError::NonFiniteFloat));
    }

    #[test]
    fn test_ceiling_i64_propagates_floor_overflow() {
        let result = ceiling_i64(i64::MIN, 3);

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Floor
            })
        );
    }

    #[rstest]
    #[case(Value::Integer(314), None, Value::Integer(314))]
    #[case(Value::Float(314.1), None, Value::Float(315.0))]
    #[case(Value::Integer(314), Some(Value::Integer(10)), Value::Integer(320))]
    #[case(Value::Integer(314), Some(Value::Float(100.0)), Value::Float(400.0))]
    #[case(Value::Float(314.1), Some(Value::Integer(10)), Value::Integer(320))]
    #[case(Value::Float(314.1), Some(Value::Float(100.0)), Value::Float(400.0))]
    fn test_eval_ceiling_function_regular(
        #[case] value: Value,
        #[case] precision: Option<Value>,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let mut arguments = vec![value_to_expression(value)];
        if let Some(precision) = precision {
            arguments.push(value_to_expression(precision));
        }
        let expr = Box::new(Expression::Function {
            func: Func::Ceiling,
            arguments,
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Value::Bool(true), None)]
    #[case(Value::Bool(true), Some(Value::Integer(2)))]
    #[case(Value::Bool(true), Some(Value::Float(2.0)))]
    #[case(Value::Integer(2), Some(Value::Bool(true)))]
    #[case(Value::Float(2.0), Some(Value::Bool(true)))]
    fn test_apply_ceiling_function_bool_invalid_type(
        #[case] value: Value,
        #[case] precision: Option<Value>,
    ) {
        let result = apply_ceiling_function(value, precision);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(Value::Integer(314), Some(Value::Integer(0)))]
    #[case(Value::Integer(314), Some(Value::Float(0.0)))]
    #[case(Value::Float(314.1), Some(Value::Integer(0)))]
    #[case(Value::Float(314.1), Some(Value::Float(0.0)))]
    #[case(Value::Integer(314), Some(Value::Integer(-10)))]
    #[case(Value::Float(314.1), Some(Value::Float(-10.0)))]
    fn test_apply_ceiling_function_non_positive_precision(
        #[case] value: Value,
        #[case] precision: Option<Value>,
    ) {
        let result = apply_ceiling_function(value, precision);

        assert_eq!(result, Err(EvalError::InvalidPrecision));
    }

    #[test]
    fn test_apply_ceiling_function_integer_overflow() {
        let result = apply_ceiling_function(Value::Integer(i64::MAX), Some(Value::Integer(10)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Ceiling
            })
        );
    }

    #[test]
    fn test_apply_ceiling_function_float_to_integer_overflow() {
        let result = apply_ceiling_function(Value::Float(f64::MAX), Some(Value::Integer(10)));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::FloatToInteger
            })
        );
    }

    #[test]
    fn test_ceiling_f64_float_overflow() {
        let result = ceiling_f64(f64::MAX, 0.5);

        assert_eq!(result, Err(EvalError::NonFiniteFloat));
    }

    /************ Binary function tests *************/

    #[rustfmt::skip]
    #[rstest]
    #[case(Func::Power, Expression::Integer(2), Expression::Integer(3), Value::Integer(8))]
    #[case(Func::Power, Expression::Integer(2), Expression::Float(3.0), Value::Float(8.0))]
    #[case(Func::Power, Expression::Float(2.0), Expression::Integer(3), Value::Float(8.0))]
    #[case(Func::Power, Expression::Float(2.0), Expression::Float(3.0), Value::Float(8.0))]
    #[case(Func::Modulo, Expression::Integer(13), Expression::Integer(5), Value::Integer(3))]
    #[case(Func::Modulo, Expression::Integer(13), Expression::Float(5.0), Value::Float(3.0))]
    #[case(Func::Modulo, Expression::Float(13.0), Expression::Integer(5), Value::Float(3.0))]
    #[case(Func::Modulo, Expression::Float(13.0), Expression::Float(5.0), Value::Float(3.0))]
    #[case(Func::Remainder, Expression::Integer(13), Expression::Integer(5), Value::Integer(3))]
    #[case(Func::Remainder, Expression::Integer(13), Expression::Float(5.0), Value::Float(3.0))]
    #[case(Func::Remainder, Expression::Float(13.0), Expression::Integer(5), Value::Float(3.0))]
    #[case(Func::Remainder, Expression::Float(13.0), Expression::Float(5.0), Value::Float(3.0))]
    fn test_apply_binary_function_regular(
        #[case] func: Func,
        #[case] lhs: Expression,
        #[case] rhs: Expression,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let expr = Box::new(Expression::Function {
            func,
            arguments: vec![lhs, rhs],
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Func::Min)]
    #[case(Func::Max)]
    #[case(Func::Round)]
    #[case(Func::Cos)]
    #[case(Func::Sin)]
    #[case(Func::Tan)]
    #[case(Func::ACos)]
    #[case(Func::ASin)]
    #[case(Func::ATan)]
    #[case(Func::Abs)]
    #[case(Func::Ln)]
    #[case(Func::Log)]
    #[case(Func::Exp)]
    #[case(Func::Floor)]
    #[case(Func::Ceiling)]
    fn test_apply_binary_function_unsupported_func(#[case] func: Func) {
        let result = apply_binary_function(&func, vec![Value::Integer(1), Value::Integer(2)]);

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[test]
    fn test_apply_binary_function_invalid_arguments() {
        let result = apply_binary_function(&Func::Power, vec![Value::Integer(1)]);

        assert!(matches!(result, Err(EvalError::InvalidArity { .. })));
    }

    #[rstest]
    #[case(
        vec![Value::Integer(1), Value::Integer(2)],
        Ok((Value::Integer(1), Value::Integer(2)))
    )]
    #[case(
        vec![Value::Integer(1), Value::Float(2.0)],
        Ok((Value::Integer(1), Value::Float(2.0)))
    )]
    #[case(vec![], Err(EvalError::InvalidArity { expected: "2", actual: 0 }))]
    #[case(vec![Value::Integer(1)], Err(EvalError::InvalidArity { expected: "2", actual: 1 }))]
    #[case(
        vec![Value::Integer(1), Value::Float(2.0), Value::Float(3.0)],
        Err(EvalError::InvalidArity { expected: "2", actual: 3 })
    )]
    #[case(
        vec![Value::Bool(true), Value::Float(2.0)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    #[case(
        vec![Value::Integer(1), Value::Bool(true)],
        Err(EvalError::InvalidType { expected: "integer or float", actual: "bool" })
    )]
    fn test_pare_vector_binary(
        #[case] vals: Vec<Value>,
        #[case] expected: Result<(Value, Value), EvalError>,
    ) {
        let result = pare_vector_binary(vals);

        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(Value::Bool(true), Value::Bool(false))]
    #[case(Value::Bool(true), Value::Integer(2))]
    #[case(Value::Integer(2), Value::Bool(true))]
    #[case(Value::Bool(true), Value::Float(2.0))]
    #[case(Value::Float(2.0), Value::Bool(true))]
    fn test_apply_power_function_bool_invalid_type(#[case] lhs: Value, #[case] rhs: Value) {
        let result = apply_power_function(lhs, rhs);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[test]
    fn test_apply_power_function_integer_exponent_too_large() {
        let result = apply_power_function(Value::Integer(10), Value::Integer(5_000_000_000));

        assert_eq!(
            result,
            Err(EvalError::InvalidExponent {
                exponent: 5_000_000_000
            })
        );
    }

    #[test]
    fn test_apply_power_function_integer_overflow() {
        let result =
            apply_power_function(Value::Integer(1_000_000_000), Value::Integer(1_000_000_000));

        assert_eq!(
            result,
            Err(EvalError::IntegerOverflow {
                op: ArithmeticOp::Power
            })
        );
    }

    #[rstest]
    #[case(Value::Bool(true), Value::Bool(false))]
    #[case(Value::Bool(true), Value::Integer(2))]
    #[case(Value::Integer(2), Value::Bool(true))]
    #[case(Value::Bool(true), Value::Float(2.0))]
    #[case(Value::Float(2.0), Value::Bool(true))]
    fn test_apply_modulo_function_bool_invalid_type(#[case] lhs: Value, #[case] rhs: Value) {
        let result = apply_modulo_function(lhs, rhs);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(Value::Bool(true), Value::Bool(false))]
    #[case(Value::Bool(true), Value::Integer(2))]
    #[case(Value::Integer(2), Value::Bool(true))]
    #[case(Value::Bool(true), Value::Float(2.0))]
    #[case(Value::Float(2.0), Value::Bool(true))]
    fn test_apply_remainder_function_bool_invalid_type(#[case] lhs: Value, #[case] rhs: Value) {
        let result = apply_remainder_function(lhs, rhs);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    /************ Unary function tests *************/

    #[rstest]
    #[case(Func::Cos, Value::Integer(0), Value::Float(1.0))]
    #[case(Func::Cos, Value::Float(0.0), Value::Float(1.0))]
    #[case(Func::Sin, Value::Integer(0), Value::Float(0.0))]
    #[case(Func::Sin, Value::Float(0.0), Value::Float(0.0))]
    #[case(Func::Tan, Value::Integer(0), Value::Float(0.0))]
    #[case(Func::Tan, Value::Float(0.0), Value::Float(0.0))]
    #[case(Func::ACos, Value::Integer(1), Value::Float(0.0))]
    #[case(Func::ACos, Value::Float(1.0), Value::Float(0.0))]
    #[case(Func::ASin, Value::Integer(0), Value::Float(0.0))]
    #[case(Func::ASin, Value::Float(0.0), Value::Float(0.0))]
    #[case(Func::ATan, Value::Integer(0), Value::Float(0.0))]
    #[case(Func::ATan, Value::Float(0.0), Value::Float(0.0))]
    #[case(Func::Abs, Value::Integer(-1), Value::Float(1.0))]
    #[case(Func::Abs, Value::Float(-1.0), Value::Float(1.0))]
    #[case(Func::Ln, Value::Integer(1), Value::Float(0.0))]
    #[case(Func::Ln, Value::Float(1.0), Value::Float(0.0))]
    #[case(Func::Log, Value::Integer(1), Value::Float(0.0))]
    #[case(Func::Log, Value::Float(1.0), Value::Float(0.0))]
    #[case(Func::Exp, Value::Integer(0), Value::Float(1.0))]
    #[case(Func::Exp, Value::Float(0.0), Value::Float(1.0))]
    fn test_unary_function_regular(
        #[case] func: Func,
        #[case] value: Value,
        #[case] expected: Value,
    ) {
        let variables: HashMap<String, Value> = HashMap::new();
        let argument = match value {
            Value::Bool(value) => Expression::Bool(value),
            Value::Integer(value) => Expression::Integer(value),
            Value::Float(value) => Expression::Float(value),
        };
        let expr = Box::new(Expression::Function {
            func,
            arguments: vec![argument],
        });

        let result = eval(&expr, &variables);

        assert_eq!(result, Ok(expected));
    }

    #[rstest]
    #[case(Func::Cos)]
    #[case(Func::Sin)]
    #[case(Func::Tan)]
    #[case(Func::ACos)]
    #[case(Func::ASin)]
    #[case(Func::ATan)]
    #[case(Func::Abs)]
    #[case(Func::Ln)]
    #[case(Func::Log)]
    #[case(Func::Exp)]
    fn test_apply_unary_function_bool_invalid_type(#[case] func: Func) {
        let result = apply_unary_function(&func, vec![Value::Bool(true)]);

        assert!(matches!(result, Err(EvalError::InvalidType { .. })));
    }

    #[rstest]
    #[case(Func::Cos)]
    #[case(Func::Sin)]
    #[case(Func::Tan)]
    #[case(Func::ACos)]
    #[case(Func::ASin)]
    #[case(Func::ATan)]
    #[case(Func::Abs)]
    #[case(Func::Ln)]
    #[case(Func::Log)]
    #[case(Func::Exp)]
    fn test_apply_unary_function_invalid_arity(#[case] func: Func) {
        let result = apply_unary_function(&func, vec![Value::Float(1.0), Value::Float(2.0)]);

        assert!(matches!(result, Err(EvalError::InvalidArity { .. })));
    }

    #[rstest]
    #[case(Func::Min)]
    #[case(Func::Max)]
    #[case(Func::Power)]
    #[case(Func::Modulo)]
    #[case(Func::Remainder)]
    #[case(Func::Round)]
    #[case(Func::Floor)]
    #[case(Func::Ceiling)]
    fn test_apply_unary_function_unsupported_func(#[case] func: Func) {
        let result = apply_unary_function(&func, vec![Value::Float(1.0)]);

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }

    #[rstest]
    #[case(Value::Bool(true))]
    #[case(Value::Integer(-1))]
    fn test_apply_float_unary(#[case] value: Value) {
        let result = apply_float_unary(value, f64::abs);

        assert_eq!(result, Err(EvalError::UnexpectedOpcode));
    }
}
