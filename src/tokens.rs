#![deny(
    clippy::expect_used,
    clippy::todo,
    clippy::unimplemented,
    clippy::unwrap_used
)]

//! Token and lexical error definitions.
//!
//! # Overview
//!
//! [`Token`] defines the Logos lexer grammar for calculator expressions:
//! punctuation, operators, built-in function names, numeric literals, variable
//! names, and lexical error tokens. [`LexicalError`] records invalid numeric
//! conversions and unknown input that Logos cannot classify as a valid token.

use logos::Logos;
use logos_display::{Debug, Display};
use std::num::{FpCategory, ParseFloatError, ParseIntError};
use std::str::ParseBoolError;

/// Error produced while converting source text into tokens.
#[derive(Default, Debug, Clone, PartialEq, thiserror::Error)]
pub enum LexicalError {
    /// A boolean literal could not be parsed as `bool`
    #[error("Invalid Bool: {0}")]
    InvalidBool(String),
    /// An integer literal could not be parsed as an `i64`.
    #[error("Invalid Integer: {0}")]
    InvalidInteger(String),
    /// A floating-point literal could not be parsed as a finite normal or zero `f64`.
    #[error("Invalid Float: {0}")]
    InvalidFloat(String),
    /// Source text did not match any token pattern.
    #[error("Unknown Symbol: {0}")]
    UnknownSymbol(String),
    /// Generic Logos error fallback.
    #[default]
    #[error("Invalid Token")]
    InvalidToken,
}

impl From<ParseBoolError> for LexicalError {
    /// Convert an integer parse failure into a lexical integer error.
    fn from(err: ParseBoolError) -> Self {
        LexicalError::InvalidBool(err.to_string())
    }
}

impl From<ParseIntError> for LexicalError {
    /// Convert an integer parse failure into a lexical integer error.
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err.to_string())
    }
}

impl From<ParseFloatError> for LexicalError {
    /// Convert a floating-point parse failure into a lexical float error.
    fn from(err: ParseFloatError) -> Self {
        LexicalError::InvalidFloat(err.to_string())
    }
}

impl LexicalError {
    /// Build an unknown-symbol error from the current lexer slice.
    fn from_lexer<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Self {
        LexicalError::UnknownSymbol(lex.slice().to_string())
    }
}

/// Parse the current `0x...` lexer slice as a hexadecimal `i64`.
fn parse_bool<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Result<bool, LexicalError> {
    let result = lex.slice().parse::<bool>();
    match result {
        Ok(val) => Ok(val),
        Err(e) => Err(LexicalError::from(e)),
    }
}

/// Parse the current `0x...` lexer slice as a hexadecimal `i64`.
fn parse_hex<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Option<i64> {
    let slice = lex.slice();
    let cleaned = slice.strip_prefix("0x").unwrap_or(slice);
    i64::from_str_radix(cleaned, 16).ok()
}

/// Parse the current lexer slice as a finite, non-subnormal `f64`.
///
/// # Errors
///
/// Returns [`LexicalError::InvalidFloat`] when Rust cannot parse the slice or
/// when the parsed value is NaN, infinite, or subnormal.
fn parse_float<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Result<f64, LexicalError> {
    let result = lex.slice().parse::<f64>()?;
    match result.classify() {
        FpCategory::Nan => Err(LexicalError::InvalidFloat("NaN".to_owned())),
        FpCategory::Infinite => Err(LexicalError::InvalidFloat("Infinite".to_owned())),
        FpCategory::Subnormal => Err(LexicalError::InvalidFloat("Subnormal".to_owned())),
        _ => Ok(result),
    }
}

/// Lexical token recognized from calculator source text.
#[derive(Logos, Debug, Display, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error(LexicalError, LexicalError::from_lexer))]
pub enum Token<'source> {
    /// `(`.
    #[token("(")]
    LeftParen,

    /// `)`.
    #[token(")")]
    RightParen,

    /// `==`.
    #[token("==")]
    Equals,

    /// `!=` or `/=`.
    #[token("!=")]
    #[token("/=")]
    NotEquals,

    /// `<=`.
    #[token("<=")]
    LessThanEquals,

    /// `>=`.
    #[token(">=")]
    GreaterThanEquals,

    /// `~=`.
    #[token("~=")]
    ApproximatelyEquals,

    /// `<`.
    #[token("<")]
    LessThan,

    /// `>`.
    #[token(">")]
    GreaterThan,

    /// `+`.
    #[token("+")]
    Plus,

    /// `-`.
    #[token("-")]
    Minus,

    /// `**`.
    #[token("**")]
    Exponentiation,

    /// `*`.
    #[token("*")]
    Multiply,

    /// `/`.
    #[token("/")]
    Divide,

    /// `%`.
    #[token("%")]
    Modulo,

    /// `&&`.
    #[token("&&")]
    LogicalAnd,

    /// `||`.
    #[token("||")]
    LogicalOr,

    /// `<<`.
    #[token("<<")]
    BitshiftLeft,

    /// `>>`.
    #[token(">>")]
    BitshiftRight,

    /// `°`.
    #[token("°")]
    Degrees,

    /// `!`.
    #[token("!")]
    LogicalNot,

    /// `&`.
    #[token("&")]
    BitwiseAnd,

    /// `^`.
    #[token("^")]
    BitwiseXor,

    /// `~`.
    #[token("~")]
    BitwiseNot,

    /// `|`.
    #[token("|")]
    BitwiseOr,

    /// `cos`.
    #[token("cos")]
    Cos,

    /// `sin`.
    #[token("sin")]
    Sin,

    /// `tan`.
    #[token("tan")]
    Tan,

    /// `min`.
    #[token("min")]
    Minimum,

    /// `max`.
    #[token("max")]
    Maximum,

    /// `pow`.
    #[token("pow")]
    Power,

    /// `mod`.
    #[token("mod")]
    Mod,

    /// `rem`.
    #[token("rem")]
    Remainder,

    /// `round`.
    #[token("round")]
    Round,

    /// `acos`.
    #[token("acos")]
    ACos,

    /// `asin`.
    #[token("asin")]
    ASin,

    /// `atan`.
    #[token("atan")]
    ATan,

    /// `abs`.
    #[token("abs")]
    AbsoluteValue,

    /// `ln`.
    #[token("ln")]
    NaturalLog,

    /// `log`.
    #[token("log")]
    Log,

    /// `exp`.
    #[token("exp")]
    Euler,

    /// `floor`.
    #[token("floor")]
    Floor,

    /// `ceil` or `ceiling`.
    #[token("ceil")]
    #[token("ceiling")]
    Ceiling,

    /// `,`.
    #[token(",")]
    Comma,

    #[regex(r"(?i)(true|false)", callback = parse_bool, priority=4)]
    Bool(bool),

    /// Decimal integer literal parsed as `i64`.
    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>())]
    Integer(i64),

    /// Hexadecimal integer literal with a `0x` prefix parsed as `i64`.
    #[regex(r"0x[[:xdigit:]]+", callback = parse_hex)]
    Hexadecimal(i64),

    /// Floating-point literal parsed as `f64`.
    #[regex(r"(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+|[0-9]+)(?:[eE][-+]?[0-9]+)|(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+)", callback = parse_float)]
    #[regex(r"NaN|nan|NAN|NaN32|NaN64", callback = parse_float, priority=5)]
    Float(f64),

    /// Variable name, optionally including one numeric index suffix.
    #[regex(r"[_[:alpha:]][_\.\w\d]*(?:\[\d+\])?", |lex| lex.slice(), priority=3)]
    Variable(&'source str),

    /// Lexical error token inserted for input that did not match a valid token.
    Error(LexicalError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;
    use proptest::prelude::*;

    fn single_token(input: &str) -> Option<Result<Token<'_>, LexicalError>> {
        let mut lexer = Token::lexer(input);
        let token = lexer.next();
        assert_eq!(lexer.next(), None);
        token
    }

    fn function_tokens() -> impl Strategy<Value = (&'static str, Token<'static>)> {
        prop_oneof![
            Just(("cos", Token::Cos)),
            Just(("sin", Token::Sin)),
            Just(("tan", Token::Tan)),
            Just(("min", Token::Minimum)),
            Just(("max", Token::Maximum)),
            Just(("pow", Token::Power)),
            Just(("mod", Token::Mod)),
            Just(("rem", Token::Remainder)),
            Just(("round", Token::Round)),
            Just(("acos", Token::ACos)),
            Just(("asin", Token::ASin)),
            Just(("atan", Token::ATan)),
            Just(("abs", Token::AbsoluteValue)),
            Just(("ln", Token::NaturalLog)),
            Just(("log", Token::Log)),
            Just(("exp", Token::Euler)),
            Just(("floor", Token::Floor)),
            Just(("ceil", Token::Ceiling)),
            Just(("ceiling", Token::Ceiling)),
        ]
    }

    fn operator_tokens() -> impl Strategy<Value = (&'static str, Token<'static>)> {
        prop_oneof![
            Just(("(", Token::LeftParen)),
            Just((")", Token::RightParen)),
            Just(("==", Token::Equals)),
            Just(("!=", Token::NotEquals)),
            Just(("/=", Token::NotEquals)),
            Just(("<=", Token::LessThanEquals)),
            Just((">=", Token::GreaterThanEquals)),
            Just(("~=", Token::ApproximatelyEquals)),
            Just(("<", Token::LessThan)),
            Just((">", Token::GreaterThan)),
            Just(("+", Token::Plus)),
            Just(("-", Token::Minus)),
            Just(("**", Token::Exponentiation)),
            Just(("*", Token::Multiply)),
            Just(("/", Token::Divide)),
            Just(("%", Token::Modulo)),
            Just(("&&", Token::LogicalAnd)),
            Just(("||", Token::LogicalOr)),
            Just(("<<", Token::BitshiftLeft)),
            Just((">>", Token::BitshiftRight)),
            Just(("°", Token::Degrees)),
            Just(("!", Token::LogicalNot)),
            Just(("&", Token::BitwiseAnd)),
            Just(("^", Token::BitwiseXor)),
            Just(("~", Token::BitwiseNot)),
            Just(("|", Token::BitwiseOr)),
            Just((",", Token::Comma)),
        ]
    }

    proptest! {
        #[test]
        fn prop_lexical_error_display_includes_variant_and_message(message in "\\PC{0,32}") {
            prop_assert_eq!(
                format!("{}", LexicalError::InvalidBool(message.clone())),
                format!("Invalid Bool: {message}")
            );
            prop_assert_eq!(
                format!("{}", LexicalError::InvalidInteger(message.clone())),
                format!("Invalid Integer: {message}")
            );
            prop_assert_eq!(
                format!("{}", LexicalError::InvalidFloat(message.clone())),
                format!("Invalid Float: {message}")
            );
            prop_assert_eq!(
                format!("{}", LexicalError::UnknownSymbol(message.clone())),
                format!("Unknown Symbol: {message}")
            );
            prop_assert_eq!(format!("{}", LexicalError::InvalidToken), "Invalid Token");
        }

        #[test]
        fn prop_parse_errors_convert_to_lexical_errors(input in "[A-Za-z_][A-Za-z0-9_]{0,16}") {
            prop_assume!(input != "true" && input != "false");

            let bool_error = match input.parse::<bool>() {
                Ok(value) => {
                    prop_assert!(false, "{input:?} unexpectedly parsed as bool {value}");
                    return Ok(());
                }
                Err(error) => error,
            };
            let int_error = match input.parse::<i64>() {
                Ok(value) => {
                    prop_assert!(false, "{input:?} unexpectedly parsed as integer {value}");
                    return Ok(());
                }
                Err(error) => error,
            };
            let float_error = match input.parse::<f64>() {
                Ok(value) => {
                    prop_assert!(false, "{input:?} unexpectedly parsed as float {value}");
                    return Ok(());
                }
                Err(error) => error,
            };

            prop_assert_eq!(
                LexicalError::from(bool_error),
                LexicalError::InvalidBool("provided string was not `true` or `false`".to_string())
            );
            prop_assert_eq!(
                LexicalError::from(int_error),
                LexicalError::InvalidInteger("invalid digit found in string".to_string())
            );
            prop_assert_eq!(
                LexicalError::from(float_error),
                LexicalError::InvalidFloat("invalid float literal".to_string())
            );
        }

        #[test]
        fn prop_decimal_integer_lexes_as_integer(value in 0i64..=i64::MAX) {
            let input = value.to_string();

            prop_assert_eq!(single_token(&input), Some(Ok(Token::Integer(value))));
        }

        #[test]
        fn prop_hexadecimal_lexes_as_hexadecimal(value in 0i64..=i64::MAX) {
            let input = format!("0x{value:x}");

            prop_assert_eq!(single_token(&input), Some(Ok(Token::Hexadecimal(value))));
        }

        #[test]
        fn prop_plain_float_lexes_as_float(
            whole in 0u64..1_000_000,
            fraction in 0u32..1_000_000,
        ) {
            let input = format!("{whole}.{fraction:06}");
            let expected = match input.parse::<f64>() {
                Ok(value) => value,
                Err(error) => {
                    prop_assert!(false, "generated float literal {input:?} failed to parse: {error}");
                    0.0
                }
            };

            prop_assert_eq!(single_token(&input), Some(Ok(Token::Float(expected))));
        }

        #[test]
        fn prop_exponent_float_lexes_as_float(
            mantissa in 1u64..1_000_000,
            exponent in -20i32..20,
        ) {
            let input = format!("{mantissa}e{exponent}");
            let expected = match input.parse::<f64>() {
                Ok(value) => value,
                Err(error) => {
                    prop_assert!(false, "generated exponent literal {input:?} failed to parse: {error}");
                    0.0
                }
            };

            prop_assert_eq!(single_token(&input), Some(Ok(Token::Float(expected))));
        }

        #[test]
        fn prop_lowercase_bool_lexes_as_bool(value in any::<bool>()) {
            let input = value.to_string();

            prop_assert_eq!(single_token(&input), Some(Ok(Token::Bool(value))));
        }

        #[test]
        fn prop_uppercase_bool_reports_invalid_bool(value in any::<bool>()) {
            let input = value.to_string().to_uppercase();

            prop_assert!(matches!(
                single_token(&input),
                Some(Err(LexicalError::InvalidBool(_)))
            ));
        }

        #[test]
        fn prop_variable_names_lex_as_variables(name in "[a-zA-Z_][a-zA-Z0-9_\\.]{0,12}(\\[[0-9]{1,4}\\])?") {
            prop_assert_eq!(single_token(&name), Some(Ok(Token::Variable(name.as_str()))));
        }

        #[test]
        fn prop_function_names_lex_as_function_tokens((input, expected) in function_tokens()) {
            prop_assert_eq!(single_token(input), Some(Ok(expected)));
        }

        #[test]
        fn prop_operator_spellings_lex_as_operator_tokens((input, expected) in operator_tokens()) {
            prop_assert_eq!(single_token(input), Some(Ok(expected)));
        }

        #[test]
        fn prop_unknown_symbols_report_unknown_symbol(input in "[@#$?]{1}") {
            prop_assert_eq!(
                single_token(&input),
                Some(Err(LexicalError::UnknownSymbol(input.clone())))
            );
        }

        #[test]
        fn prop_large_decimal_reports_invalid_integer(
            value in 18_446_744_073_709_551_616u128..100_000_000_000_000_000_000u128,
        ) {
            let input = value.to_string();

            prop_assert!(matches!(
                single_token(&input),
                Some(Err(LexicalError::InvalidInteger(_)))
            ));
        }

        #[test]
        fn prop_non_finite_float_literals_report_invalid_float(input in prop_oneof![
            Just("NaN".to_string()),
            Just("nan".to_string()),
            Just("NAN".to_string()),
            Just("NaN32".to_string()),
            Just("NaN64".to_string()),
            Just("1e309".to_string()),
            Just("12.1e-320".to_string()),
        ]) {
            prop_assert!(matches!(
                single_token(&input),
                Some(Err(LexicalError::InvalidFloat(_)))
            ));
        }
    }
}
