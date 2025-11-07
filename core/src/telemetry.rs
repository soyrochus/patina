use anyhow::Result;
use std::sync::OnceLock;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

static SUBSCRIBER_GUARD: OnceLock<()> = OnceLock::new();

/// Initialize the global tracing subscriber for the Patina workspace.
///
/// The initialization is idempotent so that unit tests and binaries can call it
/// multiple times without panicking.
pub fn init_tracing(filter: EnvFilter) -> Result<()> {
    if SUBSCRIBER_GUARD.get().is_some() {
        return Ok(());
    }

    let subscriber = Registry::default().with(filter).with(fmt::layer());
    tracing::subscriber::set_global_default(subscriber)?;
    SUBSCRIBER_GUARD.set(()).ok();

    Ok(())
}
