use crate::{
    parser::{ParseError, Parser},
    tokenizer::Tokenizer,
};

mod parser;
mod tokenizer;
mod types;

fn main() -> Result<(), ParseError> {
    let _ = Parser::parse(Tokenizer::tokenize(""))?;

    Ok(())
}
