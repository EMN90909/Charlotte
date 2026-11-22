//! Charlotte Custom Native Installer CLI / Bootstrapper entry point
//! for Windows 8, 8.1, 10, and 11 (32-bit & 64-bit, Standard & ESR modes).
//! Windows 7 SP1 systems are automatically directed to the Inno Setup Win7 ESR installer.

use charlotte_native::installer::{
    build_install_plan, render_installer_ui, InstallerConfig,
};
use charlotte_native::windows_compat::{WindowsArch, WindowsVersion};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut os = WindowsVersion::Windows11;
    let mut arch = if cfg!(target_pointer_width = "32") {
        WindowsArch::X86
    } else {
        WindowsArch::X64
    };
    let mut prefer_esr = false;
    let mut silent = false;
    let mut register_default = false;
    let mut custom_dir: Option<PathBuf> = None;
    let mut print_html = false;

    for arg in &args {
        if arg == "--silent" || arg == "/S" {
            silent = true;
        } else if arg == "--esr" || arg == "--channel=esr" {
            prefer_esr = true;
        } else if arg == "--default-browser" {
            register_default = true;
        } else if arg == "--render-ui" {
            print_html = true;
        } else if let Some(val) = arg.strip_prefix("--arch=") {
            arch = match val.to_lowercase().as_str() {
                "x86" | "32" | "32-bit" => WindowsArch::X86,
                "arm64" => WindowsArch::Arm64,
                _ => WindowsArch::X64,
            };
        } else if let Some(val) = arg.strip_prefix("--os=") {
            os = match val.to_lowercase().as_str() {
                "win7" | "windows7" => WindowsVersion::Windows7Sp1,
                "win8" | "windows8" => WindowsVersion::Windows8,
                "win8.1" | "windows8.1" => WindowsVersion::Windows8_1,
                "win10" | "windows10" => WindowsVersion::Windows10,
                _ => WindowsVersion::Windows11,
            };
        } else if let Some(val) = arg.strip_prefix("--install-dir=") {
            custom_dir = Some(PathBuf::from(val));
        }
    }

    let mut cfg = InstallerConfig::default_for(os, arch);
    if prefer_esr {
        cfg.prefer_esr = true;
    }
    cfg.silent = silent;
    cfg.register_default_browser = register_default;
    if let Some(dir) = custom_dir {
        cfg.install_dir = dir;
    }

    let plan = build_install_plan(&cfg);
    if print_html {
        println!("{}", render_installer_ui(&plan));
        return;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&plan).expect("serialize installer plan")
    );
}
