use logos::Logos;
use std::fmt; // to implement the Display trait later
use std::num::{ParseFloatError, ParseIntError};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(ParseIntError),
    InvalidFloat(ParseFloatError),
    UnknownSymbol(String),
    #[default]
    InvalidToken,
}

impl From<ParseIntError> for LexicalError {
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err)
    }
}

impl From<ParseFloatError> for LexicalError {
    fn from(err: ParseFloatError) -> Self {
        LexicalError::InvalidFloat(err)
    }
}

// TODO: add error handling for other symbols

fn parse_hex(lex: &mut logos::Lexer<Token>) -> Option<isize> {
    let slice = lex.slice();
    let cleaned = slice.strip_prefix("0x").unwrap_or(slice);
    isize::from_str_radix(cleaned, 16).ok()
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error = LexicalError)]
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

    #[regex("[0-9]+", |lex| lex.slice().parse::<isize>().unwrap())]
    Integer(isize),

    #[regex(r"0x[[:xdigit:]]+", callback = parse_hex)]
    Hexadecimal(isize),

    #[regex(r"(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+|[0-9]+)(?:[eE][-+]?[0-9]+)|[-+]?(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+)", |lex| lex.slice().parse::<f64>().unwrap())]
    Float(f64),

    #[regex(r"[_[:alpha:]][_\.\w\d]*(?:\[\d+\])?", |lex| lex.slice().to_owned())]
    Variable(String),
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