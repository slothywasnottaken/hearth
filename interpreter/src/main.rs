use tracing::{info, level_filters::LevelFilter};

use parser::{ParseError, Parser};

#[inline]
fn setup_logger() {
    let _guard = tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish(),
    );
}

fn main() -> Result<(), ParseError> {
    setup_logger();
    let parser = Parser::parse(
        r#"
        fn foo(i: string) i32 {
        let baz = "bar_foo_baz";
            else {
            let bar = "foo";
                else {
                let baz = bar;
                }
            }
        }"#,
    )?;

    match &parser.ast().nodes()[0] {
        parser::AstNode::Function(function_decl) => {
            info!("{:#?}", function_decl.block());
        }
        t => panic!("{t:?}"),
    }

    Ok(())
}
