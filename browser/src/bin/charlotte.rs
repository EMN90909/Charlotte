#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
//! Charlotte native desktop entry point. Report GUI initialization failures
//! visibly: the Windows subsystem has no console when launched by double click.

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, flags: u32) -> i32;
}

fn report_error(error: &str) {
    eprintln!("Charlotte startup failed: {error}");
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let message: Vec<u16> = std::ffi::OsStr::new(error).encode_wide().chain(Some(0)).collect();
        let title: Vec<u16> = std::ffi::OsStr::new("Charlotte Browser — startup failed")
            .encode_wide().chain(Some(0)).collect();
        // SAFETY: UTF-16 buffers are null-terminated and valid for the call.
        unsafe { MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0x10); }
    }
}

#[cfg(not(all(target_os = "windows", feature = "sandbox")))]
fn main() {
    if let Err(error) = charlotte_native::launch() {
        report_error(&error);
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "windows", feature = "sandbox"))]
fn main() {
    report_error("Use the packaged Charlotte bootstrap.exe to enable the Windows CEF sandbox.");
    std::process::exit(1);
}
