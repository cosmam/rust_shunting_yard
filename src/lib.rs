use std::error::{Error};
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub calc);

pub mod tokens;
pub mod lexer;
pub mod ast;

pub fn evaluate(text: &str) -> Result<(), Box<dyn Error>>
{
    let lexer = lexer::Lexer::new(text);
    let parser = calc::ExpressionParser::new();
    let ast = parser.parse(lexer)?;

    println!("{:?}", ast);
    Ok(())
}

/************** Test Functions **************/

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn shunting_yards() {
//         assert!(shunting_yards::TermParser::new().parse("22").is_ok());
//         assert!(shunting_yards::TermParser::new().parse("(22)").is_ok());
//         assert!(shunting_yards::TermParser::new().parse("((((22))))").is_ok());
//         assert!(shunting_yards::TermParser::new().parse("((22)").is_err());
//     }
// }
