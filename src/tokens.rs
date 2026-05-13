use logos::Logos;
use logos_display::{Debug, Display};
use std::fmt; // to implement the Display trait later
use std::num::{FpCategory, ParseFloatError, ParseIntError};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(String),
    InvalidFloat(String),
    UnknownSymbol(String),
    #[default]
    InvalidToken,
}

impl From<ParseIntError> for LexicalError {
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err.to_string())
    }
}

impl From<ParseFloatError> for LexicalError {
    fn from(err: ParseFloatError) -> Self {
        LexicalError::InvalidFloat(err.to_string())
    }
}

impl LexicalError {
    fn from_lexer<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Self {
        LexicalError::UnknownSymbol(lex.slice().to_string())
    }
}

fn parse_hex<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Option<isize> {
    let slice = lex.slice();
    let cleaned = slice.strip_prefix("0x").unwrap_or(slice);
    isize::from_str_radix(cleaned, 16).ok()
}

fn parse_float<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Result<f64, LexicalError> {
    let result = lex.slice().parse::<f64>()?;
    match result.classify() {
        FpCategory::Nan => Err(LexicalError::InvalidFloat("NaN".to_owned())),
        FpCategory::Infinite => Err(LexicalError::InvalidFloat("Infinite".to_owned())),
        FpCategory::Subnormal => Err(LexicalError::InvalidFloat("Subnormal".to_owned())),
        _ => Ok(result),
    }
}

#[derive(Logos, Debug, Display, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error(LexicalError, LexicalError::from_lexer))]
pub enum Token<'source> {
    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    #[token("==")]
    Equals,

    #[token("!=")]
    #[token("/=")]
    NotEquals,

    #[token("<=")]
    LessThanEquals,

    #[token(">=")]
    GreaterThanEquals,

    #[token("~=")]
    ApproximatelyEquals,

    #[token("<")]
    LessThan,

    #[token(">")]
    GreaterThan,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("**")]
    Exponentiation,

    #[token("*")]
    Multiply,

    #[token("/")]
    Divide,

    #[token("%")]
    Modulo,

    #[token("&&")]
    LogicalAnd,

    #[token("||")]
    LogicalOr,

    #[token("<<")]
    BitshiftLeft,

    #[token(">>")]
    BitshiftRight,

    #[token("°")]
    Degrees,

    #[token("!")]
    LogicalNot,

    #[token("&")]
    BitwiseAnd,

    #[token("^")]
    BitwiseXor,

    #[token("~")]
    BitwiseNot,

    #[token("|")]
    BitwiseOr,

    #[token("cos")]
    Cos,

    #[token("sin")]
    Sin,

    #[token("tan")]
    Tan,

    #[token("min")]
    Minimum,

    #[token("max")]
    Maximum,

    #[token("pow")]
    Power,

    #[token("mod")]
    Mod,

    #[token("rem")]
    Remainder,

    #[token("round")]
    Round,

    #[token("acos")]
    ACos,

    #[token("asin")]
    ASin,

    #[token("atan")]
    ATan,

    #[token("abs")]
    AbsoluteValue,

    #[token("ln")]
    NaturalLog,

    #[token("log")]
    Log,

    #[token("exp")]
    Euler,

    #[token("floor")]
    Floor,

    #[token("ceil")]
    #[token("ceiling")]
    Ceiling,

    #[token(",")]
    Comma,

    #[regex("[0-9]+", |lex| lex.slice().parse::<isize>())]
    Integer(isize),

    #[regex(r"0x[[:xdigit:]]+", callback = parse_hex)]
    Hexadecimal(isize),

    #[regex(r"(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+|[0-9]+)(?:[eE][-+]?[0-9]+)|(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+)", callback = parse_float)]
    #[regex(r"NaN|nan|NAN|NaN32|NaN64", callback = parse_float, priority=5)]
    Float(f64),

    #[regex(r"[_[:alpha:]][_\.\w\d]*(?:\[\d+\])?", |lex| lex.slice(), priority=3)]
    Variable(&'source str),

    Error(LexicalError),
}

impl fmt::Display for LexicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidToken => write!(f, "Invalid Token"),
            LexicalError::UnknownSymbol(c) => write!(f, "Unknown Symbol: {}", c),
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
