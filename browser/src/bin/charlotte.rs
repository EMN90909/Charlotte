#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
//! Windows sandbox releases are launched using the packaged CEF bootstrap.
#[cfg(not(all(target_os = "windows", feature = "sandbox")))]
fn main() -> Result<(), String> { charlotte_native::launch() }

#[cfg(all(target_os = "windows", feature = "sandbox"))]
fn main() -> Result<(), &'static str> {
    Err("Run packaged Charlotte bootstrap.exe to enable the Windows CEF sandbox")
}
