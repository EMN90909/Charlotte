use std::{fs, path::PathBuf};
pub fn ensure_profile_dirs() -> Result<(), String> {
    let root = profile_root();
    for directory in [root.join("cache"), root.join("downloads"), root.join("sessions")] { fs::create_dir_all(directory).map_err(|error| error.to_string())?; }
    Ok(())
}
pub fn profile_root() -> PathBuf {
    std::env::var_os("CHARLOTTE_PROFILE_DIR").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".charlotte")
    })
}
