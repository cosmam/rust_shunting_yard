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
    Function {
        func: Func, 
        arguments: Vec<Box<Expression>>,
    },
    Error,
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
    Plus,
    Minus,
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
    Min,
    Max,
    Power,
    Modulo,
    Remainder,
    Round,
    Cos,
    Sin,
    Tan,
    ACos,
    ASin,
    ATan,
    Abs,
    Ln,
    Log,
    Exp,
    Floor,
    Ceiling,
}
