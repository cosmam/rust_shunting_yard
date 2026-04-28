use std::error::{Error};
use lalrpop_util::lalrpop_mod;

pub mod tokens;
pub mod lexer;
pub mod ast;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(clippy::pedantic)]
    #[allow(dead_code)]
    pub calc
);

pub fn evaluate(text: &str) -> Result<(), Box<dyn Error>>
{
    let lexer = lexer::Lexer::new(text);
    let parser = calc::ExpressionParser::new();

    let mut errors = Vec::new();
    let ast = parser.parse(&mut errors, lexer)?;

    println!("{:?}", ast);
    Ok(())
}

