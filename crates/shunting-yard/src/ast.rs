//! Abstract syntax tree types for parsed expressions.
//!
//! # Overview
//!
//! The parser produces [`Expression`] trees from the token stream. Leaf nodes
//! represent literals, variables, or parser/lexer errors, while compound nodes
//! record the parsed operator or function and the child expressions it applies
//! to.
//!
//! Lexer errors are forwarded to the parser and become part of the AST. Parsing
//! errors are likewise captured, such that the resulting AST can at a minimum
//! represent the valid parts.
//!
//! NOTE: A valid AST does not guarantee a successful evaluation

#![deny(
    clippy::expect_used,
    clippy::todo,
    clippy::unimplemented,
    clippy::unwrap_used
)]

use crate::tokens::LexicalError;

/// Parsed expression node.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression<'input> {
    /// Boolean literal.
    Bool(bool),
    /// Signed integer literal, including hexadecimal literals after conversion.
    Integer(i64),
    /// Floating-point literal.
    Float(f64),
    /// Variable reference borrowed from the original input text.
    Variable(&'input str),
    /// Binary operator expression with a left operand, operator, and right operand.
    BinaryOperation {
        lhs: Box<Expression<'input>>,
        operator: Opcode,
        rhs: Box<Expression<'input>>,
    },
    /// Unary operator expression with one operand.
    UnaryOperation {
        value: Box<Expression<'input>>,
        operator: Opcode,
    },
    /// Built-in function call with parsed argument expressions.
    Function {
        func: Func,
        arguments: Vec<Expression<'input>>,
    },
    /// Lexer error preserved as an expression so parsing can recover.
    LexicalError(LexicalError),
    /// Parser recovery placeholder for a syntactically invalid expression.
    Error,
}

/// Operator parsed from infix, prefix, or postfix syntax.
#[derive(Clone, Debug, PartialEq)]
pub enum Opcode {
    /// Equality comparison, parsed from `==`.
    Equals,
    /// Inequality comparison, parsed from `!=` or `/=`.
    NotEquals,
    /// Less-than-or-equal comparison, parsed from `<=`.
    LessThanEquals,
    /// Greater-than-or-equal comparison, parsed from `>=`.
    GreaterThanEquals,
    /// Approximate-equality comparison, parsed from `~=`.
    ApproximatelyEquals,
    /// Less-than comparison, parsed from `<`.
    LessThan,
    /// Greater-than comparison, parsed from `>`.
    GreaterThan,
    /// Exponentiation, parsed from `**`
    Power,
    /// Multiplication, parsed from `*`.
    Multiply,
    /// Division, parsed from `/`.
    Divide,
    /// Addition or unary plus, parsed from `+`.
    Plus,
    /// Subtraction or unary negation, parsed from `-`.
    Minus,
    /// Modulo, parsed from `%`
    Modulo,
    /// Left bitshift, parsed from `<<`.
    BitshiftLeft,
    /// Right bitshift, parsed from `>>`.
    BitshiftRight,
    /// Boolean and, parsed from `&&`.
    LogicalAnd,
    /// Boolean or, parsed from `||`.
    LogicalOr,
    /// Boolean not, parsed from prefix `!`.
    LogicalNot,
    /// Bitwise not, parsed from prefix `~`.
    BitwiseNot,
    /// Bitwise and, parsed from `&`.
    BitwiseAnd,
    /// Bitwise or, parsed from `|`.
    BitwiseOr,
    /// Bitwise xor, parsed from `^`.
    BitwiseXor,
    /// Postfix degree-to-radian conversion, parsed from `°`.
    Degrees,
}

/// Built-in function name parsed from a function call.
#[derive(Clone, Debug, PartialEq)]
pub enum Func {
    /// `min(...)`.
    Min,
    /// `max(...)`.
    Max,
    /// `pow(...)`.
    Power,
    /// `mod(...)`.
    Modulo,
    /// `rem(...)`.
    Remainder,
    /// `round(...)`.
    Round,
    /// `cos(...)`.
    Cos,
    /// `sin(...)`.
    Sin,
    /// `tan(...)`.
    Tan,
    /// `acos(...)`.
    ACos,
    /// `asin(...)`.
    ASin,
    /// `atan(...)`.
    ATan,
    /// `abs(...)`.
    Abs,
    /// `ln(...)`.
    Ln,
    /// `log(...)`.
    Log,
    /// `exp(...)`.
    Exp,
    /// `floor(...)`.
    Floor,
    /// `ceil(...)` or `ceiling(...)`.
    Ceiling,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::tokens::Token;
    use lalrpop_util::ParseError;
    use lalrpop_util::lalrpop_mod;
    use proptest::prelude::*;
    use rstest::*;
    lalrpop_mod!(
        #[allow(clippy::unwrap_used)]
        pub calc
    ); // Load the generated module

    fn parse_expression(
        input: &str,
    ) -> Result<Box<Expression<'_>>, ParseError<usize, Token<'_>, LexicalError>> {
        let lexer = lexer::Lexer::new(input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();

        parser.parse(&mut errors, lexer)
    }

    fn variable_name() -> impl Strategy<Value = String> {
        "[A-Z_][a-zA-Z0-9_]{0,12}"
    }

    fn binary_ops() -> impl Strategy<Value = (&'static str, Opcode)> {
        prop_oneof![
            Just(("==", Opcode::Equals)),
            Just(("!=", Opcode::NotEquals)),
            Just(("/=", Opcode::NotEquals)),
            Just(("<=", Opcode::LessThanEquals)),
            Just((">=", Opcode::GreaterThanEquals)),
            Just(("~=", Opcode::ApproximatelyEquals)),
            Just(("<", Opcode::LessThan)),
            Just((">", Opcode::GreaterThan)),
            Just(("**", Opcode::Power)),
            Just(("*", Opcode::Multiply)),
            Just(("/", Opcode::Divide)),
            Just(("+", Opcode::Plus)),
            Just(("-", Opcode::Minus)),
            Just(("%", Opcode::Modulo)),
            Just(("<<", Opcode::BitshiftLeft)),
            Just((">>", Opcode::BitshiftRight)),
            Just(("&&", Opcode::LogicalAnd)),
            Just(("||", Opcode::LogicalOr)),
            Just(("&", Opcode::BitwiseAnd)),
            Just(("|", Opcode::BitwiseOr)),
            Just(("^", Opcode::BitwiseXor)),
        ]
    }

    fn left_associative_binary_ops() -> impl Strategy<Value = (&'static str, Opcode)> {
        prop_oneof![
            Just(("==", Opcode::Equals)),
            Just(("!=", Opcode::NotEquals)),
            Just(("/=", Opcode::NotEquals)),
            Just(("<=", Opcode::LessThanEquals)),
            Just((">=", Opcode::GreaterThanEquals)),
            Just(("~=", Opcode::ApproximatelyEquals)),
            Just(("<", Opcode::LessThan)),
            Just((">", Opcode::GreaterThan)),
            Just(("*", Opcode::Multiply)),
            Just(("/", Opcode::Divide)),
            Just(("+", Opcode::Plus)),
            Just(("-", Opcode::Minus)),
            Just(("%", Opcode::Modulo)),
            Just(("<<", Opcode::BitshiftLeft)),
            Just((">>", Opcode::BitshiftRight)),
            Just(("&&", Opcode::LogicalAnd)),
            Just(("||", Opcode::LogicalOr)),
            Just(("&", Opcode::BitwiseAnd)),
            Just(("|", Opcode::BitwiseOr)),
            Just(("^", Opcode::BitwiseXor)),
        ]
    }

    fn additive_ops() -> impl Strategy<Value = (&'static str, Opcode)> {
        prop_oneof![Just(("+", Opcode::Plus)), Just(("-", Opcode::Minus)),]
    }

    fn multiplicative_ops() -> impl Strategy<Value = (&'static str, Opcode)> {
        prop_oneof![
            Just(("*", Opcode::Multiply)),
            Just(("/", Opcode::Divide)),
            Just(("%", Opcode::Modulo)),
        ]
    }

    fn prefix_unary_ops() -> impl Strategy<Value = (&'static str, Opcode)> {
        prop_oneof![
            Just(("!", Opcode::LogicalNot)),
            Just(("+", Opcode::Plus)),
            Just(("-", Opcode::Minus)),
            Just(("~", Opcode::BitwiseNot)),
        ]
    }

    fn functions() -> impl Strategy<Value = (&'static str, Func)> {
        prop_oneof![
            Just(("min", Func::Min)),
            Just(("max", Func::Max)),
            Just(("pow", Func::Power)),
            Just(("mod", Func::Modulo)),
            Just(("rem", Func::Remainder)),
            Just(("round", Func::Round)),
            Just(("cos", Func::Cos)),
            Just(("sin", Func::Sin)),
            Just(("tan", Func::Tan)),
            Just(("acos", Func::ACos)),
            Just(("asin", Func::ASin)),
            Just(("atan", Func::ATan)),
            Just(("abs", Func::Abs)),
            Just(("ln", Func::Ln)),
            Just(("log", Func::Log)),
            Just(("exp", Func::Exp)),
            Just(("floor", Func::Floor)),
            Just(("ceil", Func::Ceiling)),
            Just(("ceiling", Func::Ceiling)),
        ]
    }

    proptest! {
        #[test]
        fn prop_parse_integer_literal(value in 0i64..=i64::MAX) {
            let input = value.to_string();

            prop_assert_eq!(parse_expression(&input), Ok(Box::new(Expression::Integer(value))));
        }

        #[test]
        fn prop_parse_hex_literal(value in 0i64..=i64::MAX) {
            let input = format!("0x{value:x}");

            prop_assert_eq!(parse_expression(&input), Ok(Box::new(Expression::Integer(value))));
        }

        #[test]
        fn prop_parse_float_literal(whole in 0u64..1_000_000, fraction in 0u32..1_000_000) {
            let input = format!("{whole}.{fraction:06}");
            let expected = match input.parse::<f64>() {
                Ok(value) => value,
                Err(error) => {
                    prop_assert!(false, "generated float literal {input:?} failed to parse: {error}");
                    0.0
                }
            };

            prop_assert_eq!(parse_expression(&input), Ok(Box::new(Expression::Float(expected))));
        }

        #[test]
        fn prop_parse_bool_literal(value in any::<bool>()) {
            let input = value.to_string();

            prop_assert_eq!(parse_expression(&input), Ok(Box::new(Expression::Bool(value))));
        }

        #[test]
        fn prop_parse_variable_reference(name in variable_name()) {
            prop_assert_eq!(
                parse_expression(&name),
                Ok(Box::new(Expression::Variable(name.as_str())))
            );
        }

        #[test]
        fn prop_parse_parenthesized_integer(value in 0i64..1_000_000) {
            let input = format!("({value})");

            prop_assert_eq!(parse_expression(&input), Ok(Box::new(Expression::Integer(value))));
        }

        #[test]
        fn prop_parse_binary_operator((op, opcode) in binary_ops()) {
            let input = format!("1 {op} 2");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: opcode,
                    rhs: Box::new(Expression::Integer(2)),
                }))
            );
        }

        #[test]
        fn prop_parse_prefix_unary_operator((op, opcode) in prefix_unary_ops()) {
            let input = format!("{op}2");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::UnaryOperation {
                    operator: opcode,
                    value: Box::new(Expression::Integer(2)),
                }))
            );
        }

        #[test]
        fn prop_parse_postfix_degrees(value in 0i64..1_000_000) {
            let input = format!("{value}°");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::UnaryOperation {
                    operator: Opcode::Degrees,
                    value: Box::new(Expression::Integer(value)),
                }))
            );
        }

        #[test]
        fn prop_parse_single_argument_function((name, func) in functions(), value in 0i64..1_000_000) {
            let input = format!("{name}({value})");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::Function {
                    func,
                    arguments: vec![Expression::Integer(value)],
                }))
            );
        }

        #[test]
        fn prop_parse_multi_argument_function(
            (name, func) in functions(),
            a in 0i64..1_000,
            b in 0i64..1_000,
            c in 0i64..1_000,
        ) {
            let input = format!("{name}({a}, {b}, {c})");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::Function {
                    func,
                    arguments: vec![
                        Expression::Integer(a),
                        Expression::Integer(b),
                        Expression::Integer(c),
                    ],
                }))
            );
        }

        #[test]
        fn prop_left_associative_binary_operator((op, opcode) in left_associative_binary_ops()) {
            let input = format!("1 {op} 2 {op} 3");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::BinaryOperation {
                        lhs: Box::new(Expression::Integer(1)),
                        operator: opcode.clone(),
                        rhs: Box::new(Expression::Integer(2)),
                    }),
                    operator: opcode,
                    rhs: Box::new(Expression::Integer(3)),
                }))
            );
        }

        #[test]
        fn prop_power_is_right_associative(_unit in Just(())) {
            prop_assert_eq!(
                parse_expression("1 ** 2 ** 3"),
                Ok(Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: Opcode::Power,
                    rhs: Box::new(Expression::BinaryOperation {
                        lhs: Box::new(Expression::Integer(2)),
                        operator: Opcode::Power,
                        rhs: Box::new(Expression::Integer(3)),
                    }),
                }))
            );
        }

        #[test]
        fn prop_multiplicative_precedence_on_rhs(
            (add_op, add_opcode) in additive_ops(),
            (mul_op, mul_opcode) in multiplicative_ops(),
        ) {
            let input = format!("1 {add_op} 2 {mul_op} 3");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: add_opcode,
                    rhs: Box::new(Expression::BinaryOperation {
                        lhs: Box::new(Expression::Integer(2)),
                        operator: mul_opcode,
                        rhs: Box::new(Expression::Integer(3)),
                    }),
                }))
            );
        }

        #[test]
        fn prop_multiplicative_precedence_on_lhs(
            (mul_op, mul_opcode) in multiplicative_ops(),
            (add_op, add_opcode) in additive_ops(),
        ) {
            let input = format!("1 {mul_op} 2 {add_op} 3");

            prop_assert_eq!(
                parse_expression(&input),
                Ok(Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::BinaryOperation {
                        lhs: Box::new(Expression::Integer(1)),
                        operator: mul_opcode,
                        rhs: Box::new(Expression::Integer(2)),
                    }),
                    operator: add_opcode,
                    rhs: Box::new(Expression::Integer(3)),
                }))
            );
        }

        #[test]
        fn prop_lexical_error_token_becomes_expression(input in "[@#$?]{1}") {
            prop_assert!(matches!(
                parse_expression(&input),
                Ok(expr) if matches!(*expr, Expression::LexicalError(_))
            ));
        }
    }

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

        let values: Vec<Expression> = vec![Expression::Integer(10)];

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
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    fn test_binary_relational_and_equality_left(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1<2{}3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: Opcode::LessThan,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: other,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    fn test_binary_relational_and_equality_right(#[case] op_str: &str, #[case] other: Opcode) {
        let input = format!("1{}2<3", op_str);
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
                    operator: Opcode::LessThan,
                    rhs: Box::new(Expression::Integer(3)),
                }),
            }),)
        );
    }

    #[rstest]
    #[case("==", Opcode::Equals, "&", Opcode::BitwiseAnd)]
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
    #[case("==", Opcode::Equals, "&", Opcode::BitwiseAnd)]
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
    fn test_binary_equality_and_logical_or_left(#[case] op_str: &str, #[case] opcode: Opcode) {
        let input = format!("1{}2||3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(1)),
                    operator: opcode,
                    rhs: Box::new(Expression::Integer(2)),
                }),
                operator: Opcode::LogicalOr,
                rhs: Box::new(Expression::Integer(3)),
            }),)
        );
    }

    #[rstest]
    #[case("==", Opcode::Equals)]
    #[case("!=", Opcode::NotEquals)]
    #[case("/=", Opcode::NotEquals)]
    #[case("~=", Opcode::ApproximatelyEquals)]
    fn test_binary_equality_and_logical_or_right(#[case] op_str: &str, #[case] opcode: Opcode) {
        let input = format!("1||2{}3", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: Opcode::LogicalOr,
                rhs: Box::new(Expression::BinaryOperation {
                    lhs: Box::new(Expression::Integer(2)),
                    operator: opcode,
                    rhs: Box::new(Expression::Integer(3)),
                }),
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

        let values: Vec<Expression> = vec![
            Expression::Integer(10),
            Expression::Float(12.1),
            Expression::Variable("Test_Name"),
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

        let values: Vec<Expression> = vec![
            Expression::Integer(10),
            Expression::Float(12.1),
            Expression::Variable("Test_Name"),
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
    fn test_parse_function_multiexpression(#[case] op_str: &str, #[case] expected: Func) {
        let input = format!("{}(10 + 12.1)", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Expression> = vec![Expression::BinaryOperation {
            lhs: Box::new(Expression::Integer(10)),
            operator: Opcode::Plus,
            rhs: Box::new(Expression::Float(12.1)),
        }];

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
    fn test_parse_function_multimultiexpression(#[case] op_str: &str, #[case] expected: Func) {
        let input = format!("{}(10 + 12.1, 10 ** Test_Var)", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Expression> = vec![
            Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(10)),
                operator: Opcode::Plus,
                rhs: Box::new(Expression::Float(12.1)),
            },
            Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(10)),
                operator: Opcode::Power,
                rhs: Box::new(Expression::Variable("Test_Var")),
            },
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
    fn test_is_binary_ops_function(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("1 {} cos(1.1)", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Expression> = vec![Expression::Float(1.1)];

        assert_eq!(
            result,
            Ok(Box::new(Expression::BinaryOperation {
                lhs: Box::new(Expression::Integer(1)),
                operator: expected,
                rhs: Box::new(Expression::Function {
                    func: Func::Cos,
                    arguments: values,
                }),
            }))
        );
    }

    #[rstest]
    #[case("!", Opcode::LogicalNot)]
    #[case("+", Opcode::Plus)]
    #[case("-", Opcode::Minus)]
    #[case("~", Opcode::BitwiseNot)]
    fn test_unary_operators_with_functions(#[case] op_str: &str, #[case] expected: Opcode) {
        let input = format!("{}cos(1.1)", op_str);
        let lexer = lexer::Lexer::new(&input);
        let parser = calc::ExpressionParser::new();
        let mut errors = Vec::new();
        let result = parser.parse(&mut errors, lexer);

        let values: Vec<Expression> = vec![Expression::Float(1.1)];

        assert_eq!(
            result,
            Ok(Box::new(Expression::UnaryOperation {
                operator: expected,
                value: Box::new(Expression::Function {
                    func: Func::Cos,
                    arguments: values,
                }),
            }))
        );
    }

    // error conditions
}
