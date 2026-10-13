//! Windows: monitor work areas, the high-contrast setting and message boxes.

#![allow(unsafe_code)]

use calc_ui::{Platform, Rect};
use eframe::egui::Pos2;
use windows_sys::Win32::Foundation::{LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MONITORINFOF_PRIMARY, MessageBoxW, SPI_GETHIGHCONTRAST,
    SystemParametersInfoW,
};

pub struct Windows;

impl Windows {
    pub fn new() -> Windows {
        Windows
    }
}

unsafe extern "system" fn collect(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> windows_sys::core::BOOL {
    // SAFETY: `data` is the `&mut Vec` passed to EnumDisplayMonitors below, alive for
    // the whole enumeration.
    let out = unsafe { &mut *(data as *mut Vec<(bool, Rect)>) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a properly sized MONITORINFO.
    if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
        let r = info.rcWork;
        out.push((
            info.dwFlags & MONITORINFOF_PRIMARY != 0,
            Rect::from_min_max(
                Pos2::new(r.left as f32, r.top as f32),
                Pos2::new(r.right as f32, r.bottom as f32),
            ),
        ));
    }
    1
}

impl Platform for Windows {
    /// Monitor work areas (without the taskbar) in physical pixels, primary first.
    fn monitors(&self) -> Vec<Rect> {
        let mut found: Vec<(bool, Rect)> = Vec::new();
        // SAFETY: the callback only writes to `found`, which outlives the call.
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect),
                &mut found as *mut _ as LPARAM,
            );
        }
        found.sort_by_key(|(primary, _)| !primary);
        found.into_iter().map(|(_, r)| r).collect()
    }

    fn high_contrast(&self) -> bool {
        let mut hc = HIGHCONTRASTW {
            cbSize: size_of::<HIGHCONTRASTW>() as u32,
            dwFlags: 0,
            lpszDefaultScheme: std::ptr::null_mut(),
        };
        // SAFETY: SPI_GETHIGHCONTRAST fills a HIGHCONTRASTW of the given size.
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                hc.cbSize,
                &mut hc as *mut _ as *mut core::ffi::c_void,
                0,
            )
        };
        ok != 0 && hc.dwFlags & HCF_HIGHCONTRASTON != 0
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn message_box(title: &str, text: &str) {
    let (title, text) = (wide(title), wide(text));
    // SAFETY: both strings are NUL-terminated UTF-16 and outlive the call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
