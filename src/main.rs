use log::{debug, error, info as inf, trace, warn};
use std::time::SystemTime;

use crate::parser::Parser;

mod parser;
mod tokenizer;

fn main() {
    setup_logger().unwrap();
    let data = r#"
        struct Bar {
            baz: i32,
        }

        let baz = "foo";

        let barf: [i32] = [1, 2, "foo",3];

        let foo = Bar {
        baz=1
        };

        "#;
    let parser = Parser::new(data).parse();

    inf!("{parser:?}");
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}
