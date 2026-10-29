#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
//! Charlotte Native owns the desktop process. Electron is not part of this executable.
mod app;
mod profile;
mod storage;
use cef::args::Args;
fn main() -> Result<(), String> {
    profile::ensure_profile_dirs()?;
    let storage = storage::Storage::open(&profile::profile_root().join("charlotte.sqlite"))
        .map_err(|error| format!("Unable to open Charlotte's local database: {error}"))?;
    if storage.setting("shields_enabled").map_err(|error| error.to_string())?.is_none() {
        storage.set_setting("shields_enabled", "true").map_err(|error| error.to_string())?;
    }
    let args = Args::new();
    let command_line = args.as_cmd_line().ok_or_else(|| "CEF could not parse the process command line".to_owned())?;
    let exit_code = app::run(args.as_main_args(), &command_line);
    if exit_code != 0 { return Err(format!("CEF exited with code {exit_code}")); }
    Ok(())
}
