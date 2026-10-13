//! Per-OS services behind [`calc_ui::Platform`]. `unsafe` is allowed only in this
//! module tree (ARC-002).

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{Linux as Native, message_box};
#[cfg(target_os = "macos")]
pub use macos::{Mac as Native, message_box};
#[cfg(windows)]
pub use windows::{Windows as Native, message_box};

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub type Native = calc_ui::NoPlatform;

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub fn message_box(title: &str, text: &str) {
    eprintln!("{title}: {text}");
}

pub fn new() -> Native {
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    return calc_ui::NoPlatform;
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    Native::new()
}
