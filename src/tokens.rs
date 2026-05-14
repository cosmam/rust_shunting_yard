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
use std::fmt;
use std::num::{FpCategory, ParseFloatError, ParseIntError};
use std::str::ParseBoolError;

/// Error produced while converting source text into tokens.
#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    /// A boolean literal could not be parsed as `bool`
    InvalidBool(String),
    /// An integer literal could not be parsed as an `i64`.
    InvalidInteger(String),
    /// A floating-point literal could not be parsed as a finite normal or zero `f64`.
    InvalidFloat(String),
    /// Source text did not match any token pattern.
    UnknownSymbol(String),
    /// Generic Logos error fallback.
    #[default]
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

impl fmt::Display for LexicalError {
    /// Format the lexical error as a short human-readable message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidToken => write!(f, "Invalid Token"),
            LexicalError::UnknownSymbol(c) => write!(f, "Unknown Symbol: {}", c),
            LexicalError::InvalidBool(c) => write!(f, "Invalid Bool: {}", c),
            LexicalError::InvalidInteger(c) => write!(f, "Invalid Integer: {}", c),
            LexicalError::InvalidFloat(c) => write!(f, "Invalid Float: {}", c),
        }
    }
}

impl std::error::Error for LexicalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_float_error_transformation() {
        let error_instance = "not_a_float".parse::<f64>().unwrap_err();
        let custom_err = LexicalError::from(error_instance);

        assert_eq!(
            custom_err,
            LexicalError::InvalidFloat("invalid float literal".to_string())
        );
    }

    #[test]
    fn test_display_lexical_error_token() {
        let lexical_error = LexicalError::InvalidToken;

        assert_eq!(format!("{}", lexical_error), "Invalid Token");
    }

    #[test]
    fn test_display_lexical_error_integer() {
        let lexical_error = LexicalError::InvalidInteger("Test".to_string());

        assert_eq!(format!("{}", lexical_error), "Invalid Integer: Test");
    }

    #[test]
    fn test_display_lexical_error_float() {
        let lexical_error = LexicalError::InvalidFloat("Test".to_string());

        assert_eq!(format!("{}", lexical_error), "Invalid Float: Test");
    }

    #[test]
    fn test_display_lexical_error_symbol() {
        let lexical_error = LexicalError::UnknownSymbol("Test".to_string());

        assert_eq!(format!("{}", lexical_error), "Unknown Symbol: Test");
    }
}
