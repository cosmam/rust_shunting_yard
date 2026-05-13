use crate::ast::{Expression, Func, Opcode};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Integer(isize),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvalError {
    InvalidArity,
    InvalidExpression,
    UnknownVariable(String)
}

pub fn eval(expr: &Expression, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    match expr {
        Expression::Bool(n) => Ok(Value::Bool(*n)),
        Expression::Integer(n) => Ok(Value::Integer(*n)),
        Expression::Float(n) => Ok(Value::Float(*n)),

        Expression::UnaryOperation { operator, value } => {
            let value = eval(value, variables)?;
            apply_unary(operator, value, variables)
        }

        Expression::BinaryOperation { lhs, operator, rhs } => {
            let left = eval(lhs, variables)?;
            let right = eval(rhs, variables)?;
            apply_binary(operator, left, right, variables)
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

fn apply_binary(
    op: &Opcode,
    lhs: Value,
    rhs: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    match op {
        Opcode::Equals
        | Opcode::NotEquals
        | Opcode::LessThan
        | Opcode::GreaterThan
        | Opcode::GreaterThanEquals
        | Opcode::LessThanEquals
        | Opcode::ApproximatelyEquals => apply_binary_comparison(op, lhs, rhs, variables),
        Opcode::Power
        | Opcode::Multiply
        | Opcode::Divide
        | Opcode::Plus
        | Opcode::Minus
        | Opcode::Modulo => apply_binary_arithmatic(op, lhs, rhs, variables),
        Opcode::BitwiseAnd | Opcode::BitwiseOr | Opcode::BitwiseXor => {
            apply_binary_bit_operation(op, lhs, rhs, variables)
        }
        Opcode::BitshiftLeft | Opcode::BitshiftRight => {
            apply_bitshift_operation(op, lhs, rhs, variables)
        }
        Opcode::LogicalAnd | Opcode::LogicalOr => {
            apply_binary_bit_operation(op, lhs, rhs, variables)
        }
        Opcode::Degrees
        | Opcode::Minus
        | Opcode::Modulo
        | Opcode::BitwiseNot
        | Opcode::LogicalNot => Err(EvalError::InvalidArity),
    }
}

fn apply_binary_comparison(
    op: &Opcode,
    lhs: Value,
    rhs: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_binary_arithmatic(
    op: &Opcode,
    lhs: Value,
    rhs: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_binary_bit_operation(
    op: &Opcode,
    lhs: Value,
    rhs: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_bitshift_operation(
    op: &Opcode,
    lhs: Value,
    rhs: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_binary_logical_operation(
    op: &Opcode,
    lhs: Value,
    rhs: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

/************** Unary operations **************/

fn apply_unary(
    op: &Opcode,
    val: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    match op {
        Opcode::Degrees | Opcode::Plus | Opcode::Minus | Opcode::Modulo => {
            apply_unary_arithmatic(op, val, variables)
        }
        Opcode::BitwiseNot => apply_bitwise_not(val, variables),
        Opcode::LogicalNot => apply_logical_not(val, variables),
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

fn apply_unary_arithmatic(
    op: &Opcode,
    val: Value,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_bitwise_not(val: Value, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

fn apply_logical_not(val: Value, variables: &HashMap<String, Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}

/************** Functions operations **************/

fn apply_function(func: &Func, vals: Vec<Value>) -> Result<Value, EvalError> {
    Ok(Value::Bool(false))
}
