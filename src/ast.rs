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
    use crate::lexer;
    use lalrpop_util::lalrpop_mod;
    use rstest::*;
    lalrpop_mod!(pub calc); // Load the generated module

    /************ Single-symbol parsing tests *************/

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

        assert_eq!(result, Ok(Box::new(Expression::Float(expected))));
    }

    #[test]
    fn test_parse_integer() {
        let lexer = lexer::Lexer::new("146");
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(result, Ok(Box::new(Expression::Integer(146))));
    }

    #[test]
    fn test_parse_hex() {
        let lexer = lexer::Lexer::new("0xfe");
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(result, Ok(Box::new(Expression::Integer(254))));
    }

    #[test]
    fn test_parse_variable() {
        let lexer = lexer::Lexer::new("some_name[1]");
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::Variable("some_name[1]".to_string())))
        );
    }

    #[rstest]
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("<=", Opcode::LessThanEquals)]
    #[case(">=", Opcode::GreaterThanEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    #[case("**", Opcode::Power)]
    #[case("*", Opcode::Multiply)]
    #[case("/", Opcode::Divide)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("^", Opcode::BitwiseXor)]
    #[case("%", Opcode::Modulo)]
    #[case("&&", Opcode::LogicalAnd)]
    #[case("||", Opcode::LogicalOr)]
    #[case("<<", Opcode::BitshiftLeft)]
    #[case(">>", Opcode::BitshiftRight)]
    #[case("&", Opcode::BitwiseAnd)]
    #[case("|", Opcode::BitwiseOr)]
    #[case("<", Opcode::LessThan)]
    #[case(">", Opcode::GreaterThan)]
    fn test_parse_binary_operators(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("1 {} 2", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: expected,
                rhs: Box::new(Expression::Integer(2))
            }))
        );
    }

    #[rstest]
    #[case("~", Opcode::BitwiseNot)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("!", Opcode::LogicalNot)]
    fn test_parse_unary_operators_left(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("{}3.1", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::UnaryOperation {
                operator: expected,
                value: Box::new(Expression::Float(3.1))
            }))
        );
    }

    #[rstest]
    #[case("°", Opcode::Degrees)]
    fn test_parse_unary_operators_right(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("3.1{}", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::UnaryOperation {
                operator: expected,
                value: Box::new(Expression::Float(3.1))
            }))
        );
    }

    // functions can take a list, but that's a different test
    #[rstest]
    #[case("min", Func::Min)]
    #[case("max", Func::Max)]
    #[case("pow", Func::Power)]
    #[case("mod", Func::Modulo)]
    #[case("rem", Func::Remainder)]
    #[case("round", Func::Round)]
    #[case("acos", Func::ACos)]
    #[case("asin", Func::ASin)]
    #[case("atan", Func::ATan)]
    #[case("abs", Func::Abs)]
    #[case("ln", Func::Ln)]
    #[case("log", Func::Log)]
    #[case("exp", Func::Exp)]
    #[case("floor", Func::Floor)]
    #[case("ceil", Func::Ceiling)]
    #[case("ceiling", Func::Ceiling)]
    #[case("cos", Func::Cos)]
    #[case("sin", Func::Sin)]
    #[case("tan", Func::Tan)]
    fn test_parse_function_names(#[case] op_str: &str, #[case] expected: Func) {
        let input = format!("{}(10)", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Box<Expression>> = vec![Box::new(Expression::Integer(10))];

        assert_eq!(
            result,
            Ok(Box::new(Expression::Function {
                func: expected,
                arguments: values,
            }))
        );
    }

    /************ Associativity tests *************/

    #[rstest]
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("<=", Opcode::LessThanEquals)]
    #[case(">=", Opcode::GreaterThanEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    #[case("*", Opcode::Multiply)]
    #[case("/", Opcode::Divide)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("%", Opcode::Modulo)]
    #[case("&&", Opcode::LogicalAnd)]
    #[case("||", Opcode::LogicalOr)]
    #[case("<<", Opcode::BitshiftLeft)]
    #[case(">>", Opcode::BitshiftRight)]
    #[case("&", Opcode::BitwiseAnd)]
    #[case("|", Opcode::BitwiseOr)]
    #[case("<", Opcode::LessThan)]
    #[case(">", Opcode::GreaterThan)]
    fn test_is_left_associative(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("1 {} 2 {} 3", op_str, op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: expected.clone(),
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: expected.clone(),
                rhs: Box::new(Expression::Integer(3)),
            }))
        );
    }

    #[rstest]
    #[case("**", Opcode::Power)]
    fn test_is_right_associative(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("1 {} 2 {} 3", op_str, op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: expected.clone(),
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: expected.clone(),
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }))
        );
    }

    /************ Precedence tests *************/

    // expressions in function
    // error conditions
}
