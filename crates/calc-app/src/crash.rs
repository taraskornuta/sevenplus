//! Crash reporting (NFR-011): a panic writes a crash log and tells the user, instead
//! of the window silently disappearing.

use std::backtrace::Backtrace;
use std::io::Write;

/// Installs the panic hook.
pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = format!(
            "SevenPlus {} crashed.\n\n{info}\n\nBacktrace:\n{}\n",
            env!("CARGO_PKG_VERSION"),
            Backtrace::force_capture()
        );
        tracing::error!("{report}");
        let dir = calc_config::log_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let path = dir.join(format!("crash-{stamp}.log"));
        let saved = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::File::create(&path))
            .and_then(|mut f| f.write_all(report.as_bytes()));
        let where_ = match saved {
            Ok(()) => format!("A crash report was saved to:\n{}", path.display()),
            Err(e) => format!("The crash report could not be saved: {e}"),
        };
        message(
            "SevenPlus Calculator",
            &format!("The calculator hit an unexpected error and has to close.\n\n{where_}"),
        );
        default(info);
    }));
}

/// Shows a modal error message using the platform's own dialog.
pub fn message(title: &str, text: &str) {
    crate::platform::message_box(title, text);
}
