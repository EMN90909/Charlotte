#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
//! Local Windows installer for the adjacent, already-built Charlotte.exe.
//! The historical binary only printed a plan and quit, which looked like a flash.
//! This bootstrapper now installs the actual executable and creates shortcuts.
//! Download/Win7 ESR channels remain separate; nothing is fetched without consent.
use charlotte_native::installer::{build_install_plan, render_installer_ui, InstallerConfig};
use charlotte_native::windows_compat::{WindowsArch, WindowsVersion};
use std::{env, fs, path::{Path, PathBuf}, process::{self, Command}};

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, flags: u32) -> i32;
}

#[cfg(windows)]
fn popup(message: &str, title: &str, flags: u32) -> i32 {
    use std::os::windows::ffi::OsStrExt;
    let wide = |s: &str| std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    // SAFETY: the buffers are null-terminated and live for the whole call.
    unsafe { MessageBoxW(std::ptr::null_mut(), wide(message).as_ptr(), wide(title).as_ptr(), flags) }
}
#[cfg(not(windows))]
fn popup(_message: &str, _title: &str, _flags: u32) -> i32 { 6 }

fn default_install_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("Programs").join("Charlotte"))
        .unwrap_or_else(|| env::temp_dir().join("Charlotte"))
}

fn copy_if_present(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() { return Ok(()); }
    if from == to { return Ok(()); }
    fs::copy(from, to)
        .map_err(|err| format!("Could not copy {}: {err}", from.display()))?;
    Ok(())
}

/// Wait for any running Charlotte.exe to release its Windows image mapping.
/// The user must close the old browser; we never kill it or lose open tabs.
#[cfg(windows)]
fn wait_until_browser_stops(path: &Path) -> Result<(), String> {
    if !path.is_file() { return Ok(()); }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        match fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => return Ok(()),
            Err(error) if std::time::Instant::now() >= deadline => {
                return Err(format!("Close all Charlotte windows before installing the update: {error}"));
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(450)),
        }
    }
}

#[cfg(not(windows))]
fn wait_until_browser_stops(_path: &Path) -> Result<(), String> { Ok(()) }

/// Never replace the executable via in-place copy; stage bytes first and
/// restore the old file if the final rename fails.
fn replace_executable(src: &Path, dest: &Path) -> Result<(), String> {
    let stage = dest.with_extension("pending");
    let backup = dest.with_extension("previous");
    if stage.exists() { fs::remove_file(&stage).map_err(|e| e.to_string())?; }
    fs::copy(src, &stage).map_err(|e| format!("Could not stage executable: {e}"))?;
    if backup.exists() { fs::remove_file(&backup).map_err(|e| e.to_string())?; }
    let had_old = dest.exists();
    if had_old { fs::rename(dest, &backup).map_err(|e| format!("Could not move old browser: {e}"))?; }
    if let Err(error) = fs::rename(&stage, dest) {
        if had_old { let _ = fs::rename(&backup, dest); }
        return Err(format!("Could not activate new browser: {error}"));
    }
    if had_old { let _ = fs::remove_file(&backup); }
    Ok(())
}

fn copy_resource_tree(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() { return Err(format!("Missing packaged resources: {}", source.display())); }
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for item in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = item.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        // Never traverse links from an untrusted adjacent directory.
        if ty.is_symlink() { return Err("Symlink in bundled resources is not allowed".into()); }
        let target = destination.join(entry.file_name());
        if ty.is_dir() {
            copy_resource_tree(&entry.path(), &target)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn make_shortcuts(exe: &Path, dir: &Path) -> Result<(), String> {
    // Passed through environment variables, not concatenated into PowerShell
    // literals (paths can include quotes or other shell characters).
    const SCRIPT: &str = r#"
$ErrorActionPreference='Stop'
$shell = New-Object -ComObject WScript.Shell
$desktop = [Environment]::GetFolderPath('Desktop')
$menu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
foreach($linkPath in @((Join-Path $desktop 'Charlotte Browser.lnk'), (Join-Path $menu 'Charlotte Browser.lnk'))) {
  $link = $shell.CreateShortcut($linkPath)
  $link.TargetPath = $env:CHARLOTTE_EXE
  $link.WorkingDirectory = $env:CHARLOTTE_DIR
  $link.IconLocation = $env:CHARLOTTE_EXE
  $link.Save()
}
"#;
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
        .env("CHARLOTTE_EXE", exe)
        .env("CHARLOTTE_DIR", dir)
        .status().map_err(|e| format!("Could not invoke shortcuts setup: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("Shortcut setup failed with {status}")) }
}
#[cfg(not(windows))]
fn make_shortcuts(_exe: &Path, _dir: &Path) -> Result<(), String> { Ok(()) }

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut os = WindowsVersion::Windows10;
    let mut arch = if cfg!(target_pointer_width = "32") { WindowsArch::X86 } else { WindowsArch::X64 };
    let mut esr = false;
    let mut silent = false;
    let mut plan_only = false;
    let mut print_html = false;
    let mut default_browser = false;
    let mut target = default_install_dir();
    for arg in &args {
        if arg == "--silent" || arg == "/S" { silent = true; }
        else if arg == "--esr" || arg == "--channel=esr" { esr = true; }
        else if arg == "--render-ui" { print_html = true; }
        else if arg == "--plan" { plan_only = true; }
        else if arg == "--default-browser" { default_browser = true; }
        else if let Some(v) = arg.strip_prefix("--install-dir=") { target = PathBuf::from(v); }
        else if let Some(v) = arg.strip_prefix("--arch=") {
            arch = if v == "x86" { WindowsArch::X86 } else { WindowsArch::X64 };
        } else if let Some(v) = arg.strip_prefix("--os=") {
            os = match v {
                "win7" => WindowsVersion::Windows7Sp1,
                "win8" => WindowsVersion::Windows8,
                "win8.1" => WindowsVersion::Windows8_1,
                "win11" => WindowsVersion::Windows11,
                _ => WindowsVersion::Windows10,
            };
        } else if arg == "--help" || arg == "-h" {
            println!("Usage: charlotte-installer [--silent] [--install-dir=PATH] [--plan] [--render-ui]");
            return Ok(());
        }
    }
    let mut cfg = InstallerConfig::default_for(os, arch);
    cfg.install_dir = target.clone();
    cfg.prefer_esr = esr || os.requires_esr_engine();
    cfg.silent = silent;
    cfg.register_default_browser = default_browser;
    let plan = build_install_plan(&cfg);
    if print_html {
        println!("{}", render_installer_ui(&plan));
        return Ok(());
    }
    if plan_only {
        println!("{}", serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?);
        return Ok(());
    }
    if plan.redirect_installer.is_some() {
        return Err("The Windows 7 ESR build is not yet ABI-compatible. Do not install a modern Charlotte.exe on Windows 7.".into());
    }
    let current = env::current_exe().map_err(|e| e.to_string())?;
    let bundle = current.parent().ok_or("Unable to find installer directory")?;
    let source = bundle.join("Charlotte.exe");
    if !source.is_file() {
        return Err(format!("Missing {}. Run the installer from the dist\\...\\win8-11 folder alongside Charlotte.exe.", source.display()));
    }
    if !silent {
        let message = format!("Install Charlotte Browser to:\n\n{}\n\nA real WebView2 runtime is required. Continue?", target.display());
        if popup(&message, "Install Charlotte Browser", 0x24) != 6 { return Ok(()); }
    }
    fs::create_dir_all(&target).map_err(|e| format!("Could not create install directory: {e}"))?;
    let installed_exe = target.join("Charlotte.exe");
    if installed_exe.exists() && !silent {
        popup("Before updating, save your tabs and close every Charlotte Browser window. The installer will wait up to 90 seconds for the browser to exit.", "Close Charlotte to Update", 0x40);
    }
    wait_until_browser_stops(&installed_exe)?;
    if source != installed_exe { replace_executable(&source, &installed_exe)?; }
    copy_if_present(&bundle.join("charlotte-settings.exe"), &target.join("charlotte-settings.exe"))?;
    copy_if_present(&bundle.join("installer_channels.json"), &target.join("installer_channels.json"))?;
    let updater = bundle.join("charlotte-updater.exe");
    if !updater.is_file() { return Err("Missing charlotte-updater.exe from release bundle".into()); }
    copy_if_present(&updater, &target.join("charlotte-updater.exe"))?;
    copy_resource_tree(&bundle.join("assets"), &target.join("assets"))?;
    copy_resource_tree(&bundle.join("resources"), &target.join("resources"))?;
    let manifest = serde_json::json!({
        "app": "Charlotte",
        "version": env!("CARGO_PKG_VERSION"),
        "installed_executable": installed_exe,
        "installer_mode": "local_bundle",
        "default_browser_set": false
    });
    fs::write(target.join("install_manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Could not write install manifest: {e}"))?;
    if let Err(e) = make_shortcuts(&installed_exe, &target) {
        eprintln!("Charlotte installed but shortcuts could not be created: {e}");
    }
    if default_browser {
        eprintln!("Use Windows Default Apps settings to select Charlotte; Windows does not allow silent takeover.");
    }
    if !silent {
        popup("Charlotte Browser was installed. Launching Charlotte now.", "Charlotte Installed", 0x40);
        Command::new(&installed_exe).current_dir(&target)
            .spawn().map_err(|e| format!("Installed, but unable to launch Charlotte: {e}"))?;
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Charlotte Installer: {err}");
        popup(&err, "Charlotte Installation Failed", 0x10);
        process::exit(2);
    }
}
