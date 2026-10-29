#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
//! Charlotte Native owns the desktop process. Electron is not part of this executable.
mod app;
mod profile;
mod storage;
#[allow(dead_code)]
mod shield;
#[allow(dead_code)]
mod services;
use cef::args::Args;
fn main() -> Result<(), String> {
    // CEF API compatibility must be selected before any other CEF call.
    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    let command_line = args.as_cmd_line().ok_or_else(|| "CEF could not parse the process command line".to_owned())?;
    let exit_code = app::run(args.as_main_args(), &command_line);
    if exit_code != 0 { return Err(format!("CEF exited with code {exit_code}")); }
    Ok(())
}
