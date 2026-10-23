mod cli;
mod logging;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    logging::init()?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        ?args,
        "starting Strata Mundi"
    );
    Ok(())
}
