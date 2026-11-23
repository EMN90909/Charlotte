//! Out-of-process Charlotte updater. This executable checks releases independently
//! of the browser and never calls an update "installed" before a new EXE runs.
//! Usage: charlotte-updater --check | --install
//! --install requires an explicit user action and a SHA256-verified HTTPS EXE.
use charlotte_native::{profile, storage::Storage, updater};
use sha2::{Digest, Sha256};
use std::{env, fs::{self, File}, io::{Read, Write}, path::PathBuf, process::Command, time::Duration};

const MAX_INSTALLER_BYTES: u64 = 512 * 1024 * 1024;
const STATUS_KEY: &str = "updates.last_check_status";
const PENDING_KEY: &str = "updates.pending_notice_version";

fn update_status(store: &Storage, status: &str) {
    let _ = store.set_setting(STATUS_KEY, status);
    println!("{status}");
}

fn is_hex_sha256(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

fn verified_installer_url(value: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(value).map_err(|_| "Invalid update URL")?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("Installer must use an HTTPS URL".into());
    }
    if !parsed.path().to_ascii_lowercase().ends_with(".exe") {
        return Err("Only verified Windows installer .exe assets are currently supported".into());
    }
    Ok(parsed)
}

fn download_verified(update: &updater::BrowserUpdate) -> Result<PathBuf, String> {
    if !cfg!(windows) { return Err("Windows .exe updates are only supported on Windows".into()); }
    if !is_hex_sha256(&update.sha256) {
        return Err("Release must declare a 64-character SHA-256 before downloading".into());
    }
    let url = verified_installer_url(&update.download_url)?;
    let version: String = update.version.chars()
        .filter(|c| c.is_ascii_alphanumeric() || ['.', '-', '_'].contains(c))
        .collect();
    if version.is_empty() { return Err("Invalid release version".into()); }
    let dir = profile::profile_root().join("updates").join(version);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let partial = dir.join("charlotte-installer.download");
    let completed = dir.join("charlotte-installer-verified.exe");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build().map_err(|e| e.to_string())?;
    let mut response = client.get(url).send().map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > MAX_INSTALLER_BYTES) {
        return Err("Update exceeds the 512 MiB download limit".into());
    }
    let mut file = File::create(&partial).map_err(|e| e.to_string())?;
    let mut sha = Sha256::new();
    let mut size = 0u64;
    let mut buf = [0u8; 65536];
    let result = (|| -> Result<(), String> {
        loop {
            let n = response.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 { break; }
            size += n as u64;
            if size > MAX_INSTALLER_BYTES { return Err("Update download exceeds 512 MiB".into()); }
            sha.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
        file.sync_all().map_err(|e| e.to_string())?;
        let actual = format!("{:x}", sha.finalize());
        if !actual.eq_ignore_ascii_case(&update.sha256) {
            return Err("Update SHA-256 mismatch; refusing to launch installer".into());
        }
        Ok(())
    })();
    drop(file);
    if let Err(err) = result {
        let _ = fs::remove_file(&partial);
        return Err(err);
    }
    if completed.exists() { fs::remove_file(&completed).map_err(|e| e.to_string())?; }
    fs::rename(&partial, &completed).map_err(|e| e.to_string())?;
    Ok(completed)
}

fn run(install: bool) -> Result<(), String> {
    profile::ensure_profile_dirs()?;
    let store = Storage::open(&profile::profile_root().join("charlotte.sqlite"))
        .map_err(|e| e.to_string())?;
    if env::var("CHARLOTTE_OFFLINE_UPDATES").as_deref() == Ok("1") {
        update_status(&store, "Updater: offline mode; network check skipped");
        return Ok(());
    }
    let response = updater::check_server_updates(
        &updater::server_base_url(), updater::current_version())?;
    let latest = response.latest.filter(|release|
        response.update_available &&
        updater::compare_semver(&release.version, updater::current_version()) > 0 &&
        (release.platform == "all" || release.platform == updater::current_platform()) &&
        (release.arch == "all" || release.arch == updater::current_arch())
    );
    let Some(release) = latest else {
        update_status(&store, &format!("Up to date (v{})", updater::current_version()));
        return Ok(());
    };
    let _ = store.set_setting(PENDING_KEY, &release.version);
    if !install {
        update_status(&store, &format!("Charlotte v{} available — open Updates to install", release.version));
        return Ok(());
    }
    let installer = download_verified(&release)?;
    update_status(&store, &format!("Verified Charlotte v{} installer; launching for user approval", release.version));
    Command::new(&installer)
        .current_dir(installer.parent().ok_or("Installer directory unavailable")?)
        .spawn().map_err(|e| format!("Unable to start verified installer: {e}"))?;
    // Launch success != installation success. The next browser launch checks
    // its actual executable version; we deliberately do not claim installation.
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|v| v == "--help" || v == "-h") {
        println!("charlotte-updater --check | --install");
        return;
    }
    let install = args.iter().any(|v| v == "--install");
    if args.iter().any(|v| v != "--check" && v != "--install") {
        eprintln!("Unsupported updater argument; use --check or --install");
        std::process::exit(2);
    }
    if let Err(error) = run(install) {
        eprintln!("Charlotte updater: {error}");
        if let Ok(store) = Storage::open(&profile::profile_root().join("charlotte.sqlite")) {
            update_status(&store, &format!("Update deferred: {error}"));
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reject_non_https_and_packages_without_installer() {
        assert!(verified_installer_url("http://example.org/setup.exe").is_err());
        assert!(verified_installer_url("https://example.org/release.zip").is_err());
        assert!(verified_installer_url("https://example.org/Charlotte-Setup.exe").is_ok());
    }
    #[test] fn validate_hash_format() {
        assert!(is_hex_sha256(&"a".repeat(64)));
        assert!(!is_hex_sha256(""));
        assert!(!is_hex_sha256(&"x".repeat(64)));
    }
}
