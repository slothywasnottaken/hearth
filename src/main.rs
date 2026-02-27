use tracing::{info, level_filters::LevelFilter};

use crate::parser::{ParseError, Parser};

mod parser;
mod tokenizer;
mod types;

#[inline]
fn setup_logger() {
    let _guard = tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(LevelFilter::INFO)
            .finish(),
    );
}

fn main() -> Result<(), ParseError> {
    setup_logger();
    let parser = Parser::parse(
        r#"
        struct Bar {
        u: u32,
        }
        pub struct Foo {i: i32, bar: Bar, baz: string} 
        fn foo() i32 {
        let bar = Foo{i: 10, bar: Bar {u:10}, baz: "urmom"};
        }"#,
    )?;

    for val in parser.ast().nodes() {
        match val {
            parser::AstNode::Function(function_decl) => {
                for val in &function_decl.block {
                    info!(?val);
                }
            }
            t => panic!("{t:?}"),
        }
    }

    Ok(())
}
