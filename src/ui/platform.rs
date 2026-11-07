//! Small, focus-free pieces of OS integration the kit does not wrap.
//!
//! Everything here is read-only or a window decoration call. Nothing starts a
//! process, opens a window or takes focus.

use crate::core::settings::Language;
use gpui_kit::Window;

/// Whether the OS is currently in dark appearance. Windows exposes the
/// `AppsUseLightTheme` personalisation value; other platforms fall back to the
/// window appearance reported by GPUI.
pub fn system_dark(fallback: bool) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_impl::system_dark().unwrap_or(fallback)
    }
    #[cfg(not(target_os = "windows"))]
    {
        fallback
    }
}

/// Whether the OS asks for reduced motion. Only Windows exposes the
/// "show animations" preference through a stable API; other platforms keep
/// their existing behaviour and default to animations enabled.
pub fn system_reduce_motion() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_impl::reduce_motion()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// The OS UI language, if the platform reports one.
pub fn system_language() -> Option<Language> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::locale_language()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Match the native caption/title bar to the painted scheme.
///
/// Windows keeps its DWM caption in the appearance chosen at window creation
/// unless the immersive dark-mode attribute is updated afterwards, so the
/// light theme would otherwise show a dark caption. On macOS the window
/// appearance is handled by `App::set_window_appearance`; Linux compositors
/// own the decoration and are left alone.
pub fn sync_caption(window: &Window, dark: bool) {
    #[cfg(target_os = "windows")]
    {
        windows_impl::set_caption_dark(window, dark);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, dark);
    }
}

/// `open_window` returns a handle rather than a borrowed window; run the
/// caption sync through the handle so startup can set it before the first
/// user interaction.
pub fn sync_caption_for_handle(
    handle: &gpui_kit::AnyWindowHandle,
    dark: bool,
    cx: &mut gpui_kit::App,
) {
    let _ = handle.update(cx, |_, window, _| sync_caption(window, dark));
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use raw_window_handle::RawWindowHandle;

    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_USE_IMMERSIVE_DARK_MODE_LEGACY: u32 = 19;
    const LOCALE_NAME_MAX_LENGTH: i32 = 85;
    const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as isize;
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            hkey: isize,
            sub_key: *const u16,
            value: *const u16,
            flags: u32,
            kind: *mut u32,
            data: *mut core::ffi::c_void,
            size: *mut u32,
        ) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(
            action: u32,
            param: u32,
            value: *mut core::ffi::c_void,
            win_ini: u32,
        ) -> i32;
        fn GetUserDefaultLocaleName(name: *mut u16, length: i32) -> i32;
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const core::ffi::c_void,
            size: u32,
        ) -> i32;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub(super) fn system_dark() -> Option<bool> {
        let sub_key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let value = wide("AppsUseLightTheme");
        let mut data: u32 = 1;
        let mut size = core::mem::size_of::<u32>() as u32;
        // SAFETY: both strings are null-terminated UTF-16 buffers that live for
        // the call, `data` is a valid DWORD destination and `size` describes
        // its byte length. RegGetValueW copies into them and retains nothing.
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                sub_key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_DWORD,
                core::ptr::null_mut(),
                (&raw mut data).cast(),
                &mut size,
            )
        };
        (result == 0).then_some(data == 0)
    }

    pub(super) fn reduce_motion() -> bool {
        let mut animations_enabled: i32 = 1;
        // SAFETY: `animations_enabled` is a valid, writable BOOL for the
        // duration of the call. SystemParametersInfoW reads the user's
        // "Show animations" preference and does not retain the pointer.
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                (&raw mut animations_enabled).cast(),
                0,
            )
        };
        ok != 0 && animations_enabled == 0
    }

    pub(super) fn locale_language() -> Option<Language> {
        let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
        // SAFETY: the buffer is valid for `LOCALE_NAME_MAX_LENGTH` UTF-16
        // units, which is the documented maximum including the terminator.
        let written =
            unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH) };
        if written <= 0 {
            return None;
        }
        let end = buffer
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(buffer.len());
        let locale = String::from_utf16_lossy(&buffer[..end]).to_ascii_lowercase();
        if locale.starts_with("zh") {
            Some(Language::ZhCn)
        } else if locale.is_empty() {
            None
        } else {
            Some(Language::EnUs)
        }
    }

    pub(super) fn set_caption_dark(window: &Window, dark: bool) {
        let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::Win32(win32) = handle.as_raw() else {
            return;
        };
        let hwnd = win32.hwnd.get();
        let value: i32 = i32::from(dark);
        // SAFETY: `hwnd` belongs to this window and is valid while `window`
        // is borrowed. `value` is an i32 BOOL as DwmSetWindowAttribute
        // expects for the immersive-dark-mode attributes. The call only
        // updates the non-client caption and does not retain the pointer.
        unsafe {
            let size = core::mem::size_of::<i32>() as u32;
            let pointer = (&raw const value).cast();
            if DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, pointer, size) != 0 {
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE_LEGACY,
                    pointer,
                    size,
                );
            }
        }
    }
}
