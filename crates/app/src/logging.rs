use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Logs go to stderr, filtered by `RUST_LOG` (default `info`).
/// With the `profiling` feature, spans are also streamed to a Tracy client.
pub fn init() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr));
    #[cfg(feature = "profiling")]
    let registry = registry.with(tracing_tracy::TracyLayer::default());
    registry.try_init()?;
    Ok(())
}
