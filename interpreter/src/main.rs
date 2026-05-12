use clap::Parser as _;
use tracing::{info, level_filters::LevelFilter};

use parser::{ParseError, Parser};

mod vm;

#[inline]
fn setup_logger() {
    let _guard = tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(LevelFilter::INFO)
            .finish(),
    );
}

#[derive(clap::Parser, Debug)]
struct Cfg {
    input: String,
}

fn main() -> Result<(), ParseError> {
    setup_logger();

    let cfg = Cfg::parse();

    let data = std::fs::read_to_string(&cfg.input).unwrap();

    let parser = Parser::parse(&data)?;

    for n in parser.ast().nodes() {
        info!(?n);
    }

    Ok(())
}
