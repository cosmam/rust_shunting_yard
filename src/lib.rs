use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub shunting_yards); // synthesized by LALRPOP

mod lexer;

/************** Test Functions **************/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shunting_yards() {
        assert!(shunting_yards::TermParser::new().parse("22").is_ok());
        assert!(shunting_yards::TermParser::new().parse("(22)").is_ok());
        assert!(shunting_yards::TermParser::new().parse("((((22))))").is_ok());
        assert!(shunting_yards::TermParser::new().parse("((22)").is_err());
    }
}
