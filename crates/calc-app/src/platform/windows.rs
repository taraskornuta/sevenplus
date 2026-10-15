//! Windows: monitor work areas, the high-contrast and transparency settings, the
//! Acrylic backdrop and message boxes.

#![allow(unsafe_code)]

use calc_ui::{Platform, RawDisplayHandle, RawWindowHandle, Rect};
use eframe::egui::Pos2;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DwmExtendFrameIntoClientArea,
    DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::Controls::MARGINS;
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

    /// Settings ▸ Personalization ▸ Colors ▸ Transparency effects is off.
    fn reduce_transparency(&self) -> bool {
        let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let name = wide("EnableTransparency");
        let mut value: u32 = 1;
        let mut size = size_of::<u32>() as u32;
        // SAFETY: the strings are NUL-terminated UTF-16; `value` holds a REG_DWORD.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut value as *mut u32 as *mut core::ffi::c_void,
                &mut size,
            )
        };
        status == ERROR_SUCCESS && value == 0
    }

    /// Acrylic behind the whole client area (Windows 11 22H2 and later; older
    /// versions ignore the attribute and show the tint without blur).
    fn set_blur(&self, window: RawWindowHandle, _display: RawDisplayHandle, on: bool) {
        let RawWindowHandle::Win32(h) = window else {
            return;
        };
        let hwnd = h.hwnd.get() as HWND;
        let extend = if on { -1 } else { 0 };
        let margins = MARGINS {
            cxLeftWidth: extend,
            cxRightWidth: extend,
            cyTopHeight: extend,
            cyBottomHeight: extend,
        };
        let backdrop = if on {
            DWMSBT_TRANSIENTWINDOW
        } else {
            DWMSBT_NONE
        };
        // SAFETY: `hwnd` is the live calculator window; both pointers outlive the calls
        // and the attribute size matches the i32 backdrop type.
        unsafe {
            DwmExtendFrameIntoClientArea(hwnd, &margins);
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                &backdrop as *const i32 as *const core::ffi::c_void,
                size_of::<i32>() as u32,
            );
        }
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
