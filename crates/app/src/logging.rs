use crate::crash::RecentLogs;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Logs go to stderr, filtered by `RUST_LOG` (default `info`), and the last lines are kept
/// for crash reports. With the `profiling` feature, spans are also streamed to Tracy.
pub fn init() -> anyhow::Result<RecentLogs> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let recent = RecentLogs::default();
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(recent.clone()),
        );
    #[cfg(feature = "profiling")]
    let registry = registry.with(tracing_tracy::TracyLayer::default());
    registry.try_init()?;
    Ok(recent)
}
