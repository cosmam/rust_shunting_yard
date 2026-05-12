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

    #[rstest]
    #[case("!", Opcode::LogicalNot)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("~", Opcode::BitwiseNot)]
    fn test_degrees_and_unary_three(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("{}1°", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::UnaryOperation {
                operator: other,
                value: Box::new(Expression::UnaryOperation {
                    operator: Opcode::Degrees,
                    value: Box::new(Expression::Integer(1)),
                }),
            }))
        );
    }

    #[rstest]
    #[case("!", Opcode::LogicalNot)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("~", Opcode::BitwiseNot)]
    fn test_power_and_unary_three_right(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1**{}2", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: Opcode::Power,
                rhs: Box::new(Expression::UnaryOperation {
                    operator: other,
                    value: Box::new(Expression::Integer(2)),
                }),
            }))
        );
    }

    #[rstest]
    #[case("!", Opcode::LogicalNot)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("~", Opcode::BitwiseNot)]
    fn test_power_and_unary_three_left(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("{}1**2", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::UnaryOperation {
                operator: other,
                value: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: Opcode::Power,
                    rhs: Box::new(Expression::Integer(2)),
                }),
            }))
        );
    }

    #[rstest]
    fn test_unary_three_and_four(
        #[values(("!", Opcode::LogicalNot), ("+", Opcode::Plus), ("-", Opcode::Minus), ("~", Opcode::BitwiseNot))]
        unary_op: (&str, Opcode),
        #[values(("*", Opcode::Multiply), ("/", Opcode::Divide), ("%", Opcode::Modulo))] binary_op: (&str, Opcode),
    ) {
        let input = format!("1{}{}2", binary_op.0, unary_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: binary_op.1,
                rhs: Box::new(Expression::UnaryOperation {
                    operator: unary_op.1,
                    value: Box::new(Expression::Integer(2)),
                }),
            }))
        );
    }

    #[rstest]
    #[case("*", Opcode::Multiply)]
    #[case("/", Opcode::Divide)]
    #[case("%", Opcode::Modulo)]
    fn test_binary_three_and_four_left(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1**2{}3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: Opcode::Power,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: other,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    #[case("*", Opcode::Multiply)]
    #[case("/", Opcode::Divide)]
    #[case("%", Opcode::Modulo)]
    fn test_binary_three_and_four_right(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1{}2**3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: other,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: Opcode::Power,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    fn test_binary_four_combos(
        #[values(("*", Opcode::Multiply), ("/", Opcode::Divide), ("%", Opcode::Modulo))] left_op: (
            &str,
            Opcode,
        ),
        #[values(("*", Opcode::Multiply), ("/", Opcode::Divide), ("%", Opcode::Modulo))] right_op: (
            &str,
            Opcode,
        ),
    ) {
        let input = format!("1{}2{}3", left_op.0, right_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: left_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: right_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_four_and_five_left(
        #[values(("*", Opcode::Multiply), ("/", Opcode::Divide), ("%", Opcode::Modulo))] higher_op: (&str, Opcode),
        #[values(("+", Opcode::Plus), ("-", Opcode::Minus))] lower_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", higher_op.0, lower_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: higher_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: lower_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_four_and_five_right(
        #[values(("*", Opcode::Multiply), ("/", Opcode::Divide), ("%", Opcode::Modulo))] higher_op: (&str, Opcode),
        #[values(("+", Opcode::Plus), ("-", Opcode::Minus))] lower_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", lower_op.0, higher_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: lower_op.1,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: higher_op.1,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    fn test_binary_five_combos(
        #[values(("+", Opcode::Plus), ("-", Opcode::Minus))] left_op: (&str, Opcode),
        #[values(("+", Opcode::Plus), ("-", Opcode::Minus))] right_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", left_op.0, right_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: left_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: right_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_five_and_six_left(
        #[values(("+", Opcode::Plus), ("-", Opcode::Minus))] higher_op: (&str, Opcode),
        #[values(("<<", Opcode::BitshiftLeft), (">>", Opcode::BitshiftRight))] lower_op: (
            &str,
            Opcode,
        ),
    ) {
        let input = format!("1{}2{}3", higher_op.0, lower_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: higher_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: lower_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_five_and_six_right(
        #[values(("+", Opcode::Plus), ("-", Opcode::Minus))] higher_op: (&str, Opcode),
        #[values(("<<", Opcode::BitshiftLeft), (">>", Opcode::BitshiftRight))] lower_op: (
            &str,
            Opcode,
        ),
    ) {
        let input = format!("1{}2{}3", lower_op.0, higher_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: lower_op.1,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: higher_op.1,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    fn test_binary_six_combos(
        #[values(("<<", Opcode::BitshiftLeft), (">>", Opcode::BitshiftRight))] left_op: (
            &str,
            Opcode,
        ),
        #[values(("<<", Opcode::BitshiftLeft), (">>", Opcode::BitshiftRight))] right_op: (
            &str,
            Opcode,
        ),
    ) {
        let input = format!("1{}2{}3", left_op.0, right_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: left_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: right_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_six_and_seven_left(
        #[values(("<<", Opcode::BitshiftLeft), (">>", Opcode::BitshiftRight))] higher_op: (
            &str,
            Opcode,
        ),
        #[values(("<", Opcode::LessThan), ("<=", Opcode::LessThanEquals), (">", Opcode::GreaterThan), (">=", Opcode::GreaterThanEquals))]
        lower_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", higher_op.0, lower_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: higher_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: lower_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_six_and_seven_right(
        #[values(("<<", Opcode::BitshiftLeft), (">>", Opcode::BitshiftRight))] higher_op: (
            &str,
            Opcode,
        ),
        #[values(("<", Opcode::LessThan), ("<=", Opcode::LessThanEquals), (">", Opcode::GreaterThan), (">=", Opcode::GreaterThanEquals))]
        lower_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", lower_op.0, higher_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: lower_op.1,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: higher_op.1,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    fn test_binary_seven_combos(
        #[values(("<", Opcode::LessThan), ("<=", Opcode::LessThanEquals), (">", Opcode::GreaterThan), (">=", Opcode::GreaterThanEquals))]
        left_op: (&str, Opcode),
        #[values(("<", Opcode::LessThan), ("<=", Opcode::LessThanEquals), (">", Opcode::GreaterThan), (">=", Opcode::GreaterThanEquals))]
        right_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", left_op.0, right_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: left_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: right_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    #[case("<", Opcode::LessThan)]
    #[case("<=", Opcode::LessThanEquals)]
    #[case(">", Opcode::GreaterThan)]
    #[case(">=", Opcode::GreaterThanEquals)]
    fn test_binary_seven_and_eight_left(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1{}2&3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: other,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: Opcode::BitwiseAnd,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    #[case("<", Opcode::LessThan)]
    #[case("<=", Opcode::LessThanEquals)]
    #[case(">", Opcode::GreaterThan)]
    #[case(">=", Opcode::GreaterThanEquals)]
    fn test_binary_seven_and_eight_right(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1&2{}3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: Opcode::BitwiseAnd,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: other,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    #[case("&", Opcode::BitwiseAnd, "^", Opcode::BitwiseXor)]
    #[case("^", Opcode::BitwiseXor, "|", Opcode::BitwiseOr)]
    #[case("|", Opcode::BitwiseOr, "&&", Opcode::LogicalAnd)]
    #[case("&&", Opcode::LogicalAnd, "||", Opcode::LogicalOr)]
    fn test_binary_operator_ladder_left(
        #[case] low_op_str: &str,
        #[case] low_opcode: Opcode,
        #[case] high_op_str: &str,
        #[case] high_opcode: Opcode,
    ) {
        let input = format!("1{}2{}3", low_op_str, high_op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: low_opcode,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: high_opcode,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    #[case("&", Opcode::BitwiseAnd, "^", Opcode::BitwiseXor)]
    #[case("^", Opcode::BitwiseXor, "|", Opcode::BitwiseOr)]
    #[case("|", Opcode::BitwiseOr, "&&", Opcode::LogicalAnd)]
    #[case("&&", Opcode::LogicalAnd, "||", Opcode::LogicalOr)]
    fn test_binary_operator_ladder_right(
        #[case] low_op_str: &str,
        #[case] low_opcode: Opcode,
        #[case] high_op_str: &str,
        #[case] high_opcode: Opcode,
    ) {
        let input = format!("1{}2{}3", high_op_str, low_op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: high_opcode,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: low_opcode,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    fn test_binary_operators_twelve_left(#[case] op_str: &str, #[case] opcode: Opcode) {
        let input = format!("1{}2||3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: opcode,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: Opcode::LogicalOr,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    fn test_binary_operators_twelve_right(#[case] op_str: &str, #[case] opcode: Opcode) {
        let input = format!("1||2{}3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: Opcode::LogicalOr,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: opcode,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    fn test_binary_twelve_combos(
        #[values(("==", Opcode::Equals), ("!=", Opcode::NotEquals), ("/=", Opcode::NotEquals), ("~=", Opcode::ApproximatelyEquals))]
        left_op: (&str, Opcode),
        #[values(("==", Opcode::Equals), ("!=", Opcode::NotEquals), ("/=", Opcode::NotEquals), ("~=", Opcode::ApproximatelyEquals))]
        right_op: (&str, Opcode),
    ) {
        let input = format!("1{}2{}3", left_op.0, right_op.0);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: left_op.1,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: right_op.1,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    /************ Function syntax tests *************/

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
    fn test_parse_function_multivalue(#[case] op_str: &str, #[case] expected: Func) {
        let input = format!("{}(10, 12.1, Test_Name)", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Box<Expression>> = vec![
            Box::new(Expression::Integer(10)),
            Box::new(Expression::Float(12.1)),
            Box::new(Expression::Variable("Test_Name".to_string())),
        ];

        assert_eq!(
            result,
            Ok(Box::new(Expression::Function {
                func: expected,
                arguments: values,
            }))
        );
    }

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
    fn test_parse_function_multivalue_trailing_comma(#[case] op_str: &str, #[case] expected: Func) {
        let input = format!("{}(10, 12.1, Test_Name, )", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Box<Expression>> = vec![
            Box::new(Expression::Integer(10)),
            Box::new(Expression::Float(12.1)),
            Box::new(Expression::Variable("Test_Name".to_string())),
        ];

        assert_eq!(
            result,
            Ok(Box::new(Expression::Function {
                func: expected,
                arguments: values,
            }))
        );
    }

    // error conditions
}
