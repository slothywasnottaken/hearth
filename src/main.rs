use crate::parser::Parser;

mod parser;

fn main() -> anyhow::Result<()> {
    let data = r#"let foo = "bar";"#;
    let parser = Parser::new(data).parse();

    println!("{parser:?}");
    Ok(())
}
