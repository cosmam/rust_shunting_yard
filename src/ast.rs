#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Bool(bool),
    Integer(isize),
    Float(f64),
    Variable(String),
    BinaryOperation {
        lhs: Box<Expression>, 
        operator: Opcode, 
        rhs: Box<Expression>,
    },
    UnaryOperation {
        value: Box<Expression>, 
        operator: Opcode,
    },
    // Function(Func, Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Opcode {
    Equals,
    NotEquals,
    LessThanEquals,
    GreaterThanEquals,
    ApproximatelyEquals,
    Power,
    Multiply,
    Divide,
    BitwiseXor,
    Modulo,
    LogicalAnd,
    LogicalOr,
    BitshiftLeft,
    BitshiftRight,
    Degrees,
    LogicalNot,
    BitwiseNot,
    BitwiseAnd,
    BitwiseOr,
    LessThan,
    GreaterThan,
    Comma,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Func {
    Cos,
    Sin,
    Tan,
}
