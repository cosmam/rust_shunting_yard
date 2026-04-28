use logos::Logos;
use std::fmt; // to implement the Display trait later
use std::num::{ParseFloatError, ParseIntError, FpCategory};

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
    fn from_lexer(lex: &mut logos::Lexer<'_, Token>) -> Self {
        LexicalError::UnknownSymbol(lex.slice().to_string())
    }
}

fn parse_hex(lex: &mut logos::Lexer<Token>) -> Option<isize> {
    let slice = lex.slice();
    let cleaned = slice.strip_prefix("0x").unwrap_or(slice);
    isize::from_str_radix(cleaned, 16).ok()
}

fn parse_float(lex: &mut logos::Lexer<Token>) -> Result<f64, LexicalError> {
    let result = lex.slice().parse::<f64>()?;
    match result.classify() {
        FpCategory::Nan => Err(LexicalError::InvalidFloat("NaN".to_owned())),
        FpCategory::Infinite=> Err(LexicalError::InvalidFloat("Infinite".to_owned())),
        FpCategory::Subnormal=> Err(LexicalError::InvalidFloat("Subnormal".to_owned())),
        _ => Ok(result)
    }
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error(LexicalError, LexicalError::from_lexer))]
pub enum Token {
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

    #[token("^")]
    BitwiseXor,

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

    #[token("~")]
    BitwiseNot,

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

    #[token("&")]
    BitwiseAnd,

    #[token("|")]
    BitwiseOr,

    #[token("cos")]
    Cos,

    #[token("sin")]
    Sin,

    #[token("tan")]
    Tan,

    #[token("<")]
    LessThan,

    #[token(">")]
    GreaterThan,

    #[token(",")]
    Comma,

    #[regex("[0-9]+", |lex| lex.slice().parse::<isize>())]
    Integer(isize),

    #[regex(r"0x[[:xdigit:]]+", callback = parse_hex)]
    Hexadecimal(isize),

    #[regex(r"(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+|[0-9]+)(?:[eE][-+]?[0-9]+)|[-+]?(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+)", callback = parse_float)]
    Float(f64),

    #[regex(r"[_[:alpha:]][_\.\w\d]*(?:\[\d+\])?", |lex| lex.slice().to_owned())]
    Variable(String),

    Error(LexicalError),
}

impl fmt::Display for Token {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl fmt::Display for LexicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidToken => write!(f, "Invalid token"),
            LexicalError::UnknownSymbol(c) => write!(f, "Unknown Symbol: {}", c),
            LexicalError::InvalidInteger(c) => write!(f, "Invalid Integer: {}", c),
            LexicalError::InvalidFloat(c) => write!(f, "Invalid Float: {}", c),
        }
    }
}

impl std::error::Error for LexicalError {}