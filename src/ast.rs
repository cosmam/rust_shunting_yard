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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use crate::lexer;
    use lalrpop_util::lalrpop_mod;
    lalrpop_mod!(pub calc); // Load the generated module

    #[rstest]
	#[case("1.", 1.0)]
	#[case("1.1", 1.1)]
	#[case(".1", 0.1)]
	#[case("2.1e2", 210.0)]
	#[case("2.1e+2", 210.0)]
	#[case("2.1e-2", 0.021)]
	#[case("2.1E2", 210.0)]
	#[case("2.1E+2", 210.0)]
	#[case("2.1E-2", 0.021)]	
	#[case("3e2", 300.0)]
	#[case("3e+2", 300.0)]
	#[case("3e-2", 0.03)]
	#[case("3E2", 300.0)]
	#[case("3E+2", 300.0)]
	#[case("3E-2", 0.03)]
	#[case(".4e2", 40.0)]
	#[case(".4e+2", 40.0)]
	#[case(".4e-2", 0.004)]
	#[case(".4E2", 40.0)]
	#[case(".4E+2", 40.0)]
	#[case(".4E-2", 0.004)] 
    fn test_parse_floats(#[case] input: &str, #[case] expected: f64) {
        let lexer = lexer::Lexer::new(input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::Float(expected)))
        );
    }

}