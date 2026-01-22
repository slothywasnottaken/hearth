use tracing::info;

use crate::parser::Parser;

mod parser;
mod tokenizer;

fn main() -> anyhow::Result<()> {
    let _guard = tracing::subscriber::set_default(tracing_subscriber::FmtSubscriber::new());
    let data = r#"
        struct Bar {
            baz: i32,
        }

        let baz = "foo";

        let foo = Bar {
        baz=1
        };
        "#;
    let parser = Parser::new(data).parse();

    info!(?parser);
    Ok(())
}
