use tracing::info;

use crate::parser::Parser;

mod parser;

fn main() -> anyhow::Result<()> {
    let _guard = tracing::subscriber::set_default(tracing_subscriber::FmtSubscriber::new());
    info!("foo");
    let data = r#"
        struct Bar {
            baz: i32,
        }

        let foo = Bar {
        baz=0
        };"#;
    let parser = Parser::new(data).parse();

    println!("{parser:?}");
    Ok(())
}
