//! Logging (ARC-007): a daily-rotated file in the platform log folder, level `warn` by
//! default; `RUST_LOG` overrides it.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Log files kept before the oldest is deleted.
const KEEP_FILES: usize = 7;

/// Starts logging; keep the returned guard alive until exit so buffered lines are
/// written.
pub fn init() -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let dir = calc_config::log_dir();
    let file = std::fs::create_dir_all(&dir).ok().and_then(|()| {
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("sevenplus")
            .filename_suffix("log")
            .max_log_files(KEEP_FILES)
            .build(&dir)
            .ok()
    });
    let (file_layer, guard) = match file {
        Some(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                Some(fmt::layer().with_writer(writer).with_ansi(false)),
                Some(guard),
            )
        }
        None => (None, None),
    };
    let ok = file_layer.is_some();
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(fmt::layer().with_writer(std::io::stderr).with_target(false))
        .init();
    if !ok {
        tracing::warn!("cannot write logs to {}", dir.display());
    }
    guard
}
