#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
//! Charlotte Native owns the desktop process. Electron is not part of this executable.
mod app;
mod profile;
use cef::args::Args;
fn main() -> Result<(), String> {
    profile::ensure_profile_dirs()?;
    let args = Args::new();
    let command_line = args.as_cmd_line().ok_or_else(|| "CEF could not parse the process command line".to_owned())?;
    let exit_code = app::run(args.as_main_args(), &command_line);
    if exit_code != 0 { return Err(format!("CEF exited with code {exit_code}")); }
    Ok(())
}
