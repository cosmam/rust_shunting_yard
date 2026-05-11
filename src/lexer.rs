use logos::{Logos, SpannedIter};
use crate::tokens::{LexicalError, Token}; // your Token enum, as above

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

pub struct Lexer<'input> {
    // instead of an iterator over characters, we have a token iterator
    token_stream: SpannedIter<'input, Token>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        // the Token::lexer() method is provided by the Logos trait
        Self {
            token_stream: Token::lexer(input).spanned(),
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.token_stream
        .next()
        .map(|(token, span)|
            match token {
                Ok(token) => Ok((span.start, token, span.end)),
                Err(err) => Ok((span.start, Token::Error(err), span.end)),
                // or specify your lexical error to parse error
            }
        )
    }
}

/************** Test Functions **************/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_parens() {
        let mut lex = Token::lexer("(123)");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 0..1);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(123))));
        assert_eq!(lex.span(), 1..4);
        assert_eq!(lex.slice(), "123");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_hexadecimal_parens() {
        let mut lex = Token::lexer("(0x1f)");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 0..1);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Hexadecimal(31))));
        assert_eq!(lex.span(), 1..5);
        assert_eq!(lex.slice(), "0x1f");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 5..6);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_double_parens() {
        let mut lex = Token::lexer("(12.3)");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 0..1);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Float(12.3))));
        assert_eq!(lex.span(), 1..5);
        assert_eq!(lex.slice(), "12.3");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 5..6);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_variable_parens() {
        let mut lex = Token::lexer("(Some_Name)");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 0..1);
        assert_eq!(lex.slice(), "(");

        assert_eq!(
            lex.next(),
            Some(Ok(Token::Variable("Some_Name".to_string())))
        );
        assert_eq!(lex.span(), 1..10);
        assert_eq!(lex.slice(), "Some_Name");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 10..11);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_is_equals() {
        let mut lex = Token::lexer("23== 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Equals)));
        assert_eq!(lex.span(), 2..4);
        assert_eq!(lex.slice(), "==");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_not_equals1() {
        let mut lex = Token::lexer("23 != 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::NotEquals)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "!=");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_not_equals2() {
        let mut lex = Token::lexer("23 /= 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::NotEquals)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "/=");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_less_than_equals() {
        let mut lex = Token::lexer("23 <= 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::LessThanEquals)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "<=");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_greater_than_equals() {
        let mut lex = Token::lexer("23 >= 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::GreaterThanEquals)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), ">=");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_approximately_equals() {
        let mut lex = Token::lexer("23 ~= 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::ApproximatelyEquals)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "~=");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_plus() {
        let mut lex = Token::lexer("23+14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Plus)));
        assert_eq!(lex.span(), 2..3);
        assert_eq!(lex.slice(), "+");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_minus() {
        let mut lex = Token::lexer("23-14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Minus)));
        assert_eq!(lex.span(), 2..3);
        assert_eq!(lex.slice(), "-");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_multiply() {
        let mut lex = Token::lexer("23* 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Multiply)));
        assert_eq!(lex.span(), 2..3);
        assert_eq!(lex.slice(), "*");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_divide() {
        let mut lex = Token::lexer("23 /14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Divide)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "/");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_exponentiation() {
        let mut lex = Token::lexer("23 ** 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Exponentiation)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "**");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_bitwise_xor() {
        let mut lex = Token::lexer("23 ^ 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::BitwiseXor)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "^");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_modulus() {
        let mut lex = Token::lexer("23% 14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Modulo)));
        assert_eq!(lex.span(), 2..3);
        assert_eq!(lex.slice(), "%");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_logical_and() {
        let mut lex = Token::lexer("23 &&14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::LogicalAnd)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "&&");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_logical_or() {
        let mut lex = Token::lexer("23 ||14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::LogicalOr)));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "||");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_bitshift_left() {
        let mut lex = Token::lexer("23   <<14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::BitshiftLeft)));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "<<");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 7..9);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_bitshift_right() {
        let mut lex = Token::lexer("23  >>    14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::BitshiftRight)));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), ">>");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 10..12);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_degrees() {
        let mut lex = Token::lexer("23°");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::Degrees)));
        assert_eq!(lex.span(), 2..4);
        assert_eq!(lex.slice(), "°");
    }

    #[test]
    fn test_min() {
        let mut lex = Token::lexer("min(12, 13, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Minimum)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "min");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(13))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "13");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 10..11);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 12..14);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 15..16);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_max() {
        let mut lex = Token::lexer("max(12, 13, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Maximum)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "max");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(13))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "13");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 10..11);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 12..14);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 15..16);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_pow() {
        let mut lex = Token::lexer("pow(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Power)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "pow");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_mod() {
        let mut lex = Token::lexer("mod(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Mod)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "mod");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_rem() {
        let mut lex = Token::lexer("rem(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Remainder)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "rem");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_round() {
        let mut lex = Token::lexer("round(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Round)));
        assert_eq!(lex.span(), 0..5);
        assert_eq!(lex.slice(), "round");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 5..6);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 8..9);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 10..12);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 13..14);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_acos() {
        let mut lex = Token::lexer("acos(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::ACos)));
        assert_eq!(lex.span(), 0..4);
        assert_eq!(lex.slice(), "acos");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 7..8);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 12..13);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_asin() {
        let mut lex = Token::lexer("asin(12, 14)");

        assert_eq!(lex.next(), Some(Ok(Token::ASin)));
        assert_eq!(lex.span(), 0..4);
        assert_eq!(lex.slice(), "asin");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 7..8);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_atan() {
        let mut lex = Token::lexer("atan(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::ATan)));
        assert_eq!(lex.span(), 0..4);
        assert_eq!(lex.slice(), "atan");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 7..8);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 12..13);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_cos() {
        let mut lex = Token::lexer("cos(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Cos)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "cos");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_sin() {
        let mut lex = Token::lexer("sin(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Sin)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "sin");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_tan() {
        let mut lex = Token::lexer("tan(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Tan)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "tan");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_abs() {
        let mut lex = Token::lexer("abs(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::AbsoluteValue)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "abs");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_log() {
        let mut lex = Token::lexer("log(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Log)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "log");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_exp() {
        let mut lex = Token::lexer("exp(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Euler)));
        assert_eq!(lex.span(), 0..3);
        assert_eq!(lex.slice(), "exp");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 3..4);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 4..6);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 6..7);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 11..12);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_floor() {
        let mut lex = Token::lexer("floor(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Floor)));
        assert_eq!(lex.span(), 0..5);
        assert_eq!(lex.slice(), "floor");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 5..6);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 6..8);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 8..9);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 10..12);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 13..14);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_ceil() {
        let mut lex = Token::lexer("ceil(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Ceiling)));
        assert_eq!(lex.span(), 0..4);
        assert_eq!(lex.slice(), "ceil");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 5..7);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 7..8);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 12..13);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_ceiling() {
        let mut lex = Token::lexer("ceiling(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::Ceiling)));
        assert_eq!(lex.span(), 0..7);
        assert_eq!(lex.slice(), "ceiling");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 7..8);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 8..10);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 10..11);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 12..14);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 15..16);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_ln() {
        let mut lex = Token::lexer("ln(12, 14 )");

        assert_eq!(lex.next(), Some(Ok(Token::NaturalLog)));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "ln");

        assert_eq!(lex.next(), Some(Ok(Token::LeftParen)));
        assert_eq!(lex.span(), 2..3);
        assert_eq!(lex.slice(), "(");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lex.span(), 3..5);
        assert_eq!(lex.slice(), "12");

        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.span(), 5..6);
        assert_eq!(lex.slice(), ",");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 7..9);
        assert_eq!(lex.slice(), "14");

        assert_eq!(lex.next(), Some(Ok(Token::RightParen)));
        assert_eq!(lex.span(), 10..11);
        assert_eq!(lex.slice(), ")");
    }

    #[test]
    fn test_less_than() {
        let mut lex = Token::lexer("23  <    14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::LessThan)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "<");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_bitwise_or() {
        let mut lex = Token::lexer("23  |    14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::BitwiseOr)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "|");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_bitwise_and() {
        let mut lex = Token::lexer("23  &    14");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 0..2);
        assert_eq!(lex.slice(), "23");

        assert_eq!(lex.next(), Some(Ok(Token::BitwiseAnd)));
        assert_eq!(lex.span(), 4..5);
        assert_eq!(lex.slice(), "&");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(14))));
        assert_eq!(lex.span(), 9..11);
        assert_eq!(lex.slice(), "14");
    }

    #[test]
    fn test_logical_not() {
        let mut lex = Token::lexer("!23");

        assert_eq!(lex.next(), Some(Ok(Token::LogicalNot)));
        assert_eq!(lex.span(), 0..1);
        assert_eq!(lex.slice(), "!");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 1..3);
        assert_eq!(lex.slice(), "23");
    }

    #[test]
    fn test_bitwise_not() {
        let mut lex = Token::lexer("~23");

        assert_eq!(lex.next(), Some(Ok(Token::BitwiseNot)));
        assert_eq!(lex.span(), 0..1);
        assert_eq!(lex.slice(), "~");

        assert_eq!(lex.next(), Some(Ok(Token::Integer(23))));
        assert_eq!(lex.span(), 1..3);
        assert_eq!(lex.slice(), "23");
    }
    
    #[test]
    fn test_new_bitwise_not() {
        let mut lex = Lexer::new("~23");

        assert_eq!(lex.next(), Some(Ok((0, Token::BitwiseNot, 1))));
        assert_eq!(lex.next(), Some(Ok((1, Token::Integer(23), 3))));
    }
    
    #[test]
    fn test_new_lexing_error_unknown_symbol() {
        let mut lex = Lexer::new("~23$");

        assert_eq!(lex.next(), Some(Ok((0, Token::BitwiseNot, 1))));
        assert_eq!(lex.next(), Some(Ok((1, Token::Integer(23), 3))));
        assert_eq!(lex.next(), Some(Ok((3, Token::Error(LexicalError::UnknownSymbol("$".to_string())), 4))));
    }
        
    #[test]
    fn test_new_lexing_error_parse_int() {
        let mut lex = Lexer::new("~12345678901234567890");

        assert_eq!(lex.next(), Some(Ok((0, Token::BitwiseNot, 1))));
        assert_eq!(lex.next(), Some(Ok((1, Token::Error(LexicalError::InvalidInteger("number too large to fit in target type".to_owned())), 21))));
    } 
       
    #[test]
    fn test_new_lexing_error_parse_float_infinite() {
        let mut lex = Lexer::new("~12.1e320");

        assert_eq!(lex.next(), Some(Ok((0, Token::BitwiseNot, 1))));
        assert_eq!(lex.next(), Some(Ok((1, Token::Error(LexicalError::InvalidFloat("Infinite".to_owned())), 9))));
    }     
       
    #[test]
    fn test_new_lexing_error_parse_float_subnormal() {
        let mut lex = Lexer::new("~12.1e-320");

        assert_eq!(lex.next(), Some(Ok((0, Token::BitwiseNot, 1))));
        assert_eq!(lex.next(), Some(Ok((1, Token::Error(LexicalError::InvalidFloat("Subnormal".to_owned())), 10))));
    } 
       
    #[test]
    fn test_new_lexing_error_parse_float_nan() {
        let mut lex = Lexer::new("~NAN");

        assert_eq!(lex.next(), Some(Ok((0, Token::BitwiseNot, 1))));
        assert_eq!(lex.next(), Some(Ok((1, Token::Error(LexicalError::InvalidFloat("NaN".to_owned())), 4))));
    } 

}
