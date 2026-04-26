use std::error::{Error};
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(clippy::pedantic)]
    #[allow(dead_code)]
    pub calc
);

pub mod tokens;
pub mod lexer;
pub mod ast;

pub fn evaluate(text: &str) -> Result<(), Box<dyn Error>>
{
    let lexer = lexer::Lexer::new(text);
    let parser = calc::ExpressionParser::new();

    let mut errors = Vec::new();
    let ast = parser.parse(&mut errors, lexer)?;

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
