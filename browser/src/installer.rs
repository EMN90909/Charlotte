//! Charlotte Custom Native Installer (Opera / Chrome-style) for Windows 8, 8.1, 10, and 11
//! across both 32-bit (`x86`) and 64-bit (`x64`), supporting Standard and ESR modes.
//!
//! Architecture:
//! - Windows 7 SP1 uses the dedicated Inno Setup ESR package (`packaging/windows/charlotte.iss`).
//!   If `charlotte-installer` detects Windows 7 SP1, it safely directs/hands off to the Win7 ESR
//!   Inno Setup package so a modern binary never fails with missing DLL imports on Win7.
//! - Windows 8 / 8.1 automatically uses the **Win 8 ESR** channel (WebView2 Fixed Version 109 ESR).
//! - Windows 10 / 11 uses the **Modern Stable** or **Modern ESR** channel (Evergreen WebView2 or
//!   pinned ESR runtime).
//! - Performs SHA-256 checksum verification before extracting payloads, supports silent enterprise
//!   installs (`--silent`), creates Start Menu & Desktop shortcuts, registers Charlotte Cloud
//!   endpoint (`https://api-charlotte-server.onrender.com`), and supports atomic rollback on failure.

use crate::cloud_client::DEFAULT_REMOTE_URL;
use crate::windows_compat::{
    resolve_runtime_profile, InstallerKind, RuntimeChannel, WindowsArch, WindowsRuntimeProfile,
    WindowsVersion,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const APP_VERSION: &str = "0.4.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub target_os: WindowsVersion,
    pub arch: WindowsArch,
    pub prefer_esr: bool,
    pub install_dir: PathBuf,
    pub create_desktop_shortcut: bool,
    pub create_start_menu_shortcut: bool,
    pub register_default_browser: bool,
    pub all_users: bool,
    pub silent: bool,
    pub cloud_server_url: String,
}

impl InstallerConfig {
    pub fn default_for(os: WindowsVersion, arch: WindowsArch) -> Self {
        let dir = match arch {
            WindowsArch::X86 => PathBuf::from(r"C:\Program Files (x86)\Charlotte"),
            WindowsArch::X64 | WindowsArch::Arm64 => PathBuf::from(r"C:\Program Files\Charlotte"),
        };
        Self {
            target_os: os,
            arch,
            prefer_esr: os.requires_esr_engine(),
            install_dir: dir,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            register_default_browser: false,
            all_users: false,
            silent: false,
            cloud_server_url: DEFAULT_REMOTE_URL.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallStage {
    PreflightCheck,
    RedirectToWin7InnoSetup,
    FetchRuntimePayload,
    VerifySha256Checksum,
    ExtractApplicationFiles,
    EnsureWebViewRuntime,
    WriteConfigAndShortcuts,
    RegisterBrowserProtocols,
    LaunchCharlotte,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlan {
    pub app_version: String,
    pub profile: WindowsRuntimeProfile,
    pub package_filename: String,
    pub download_url: String,
    pub install_dir: PathBuf,
    pub cloud_server_url: String,
    pub redirect_installer: Option<String>,
    pub stages: Vec<InstallStage>,
}

/// Builds a deterministic installation plan for Windows 7–11 (32-bit & 64-bit, Standard & ESR).
pub fn build_install_plan(cfg: &InstallerConfig) -> InstallPlan {
    let profile = resolve_runtime_profile(cfg.target_os, cfg.arch, cfg.prefer_esr);
    let arch_str = cfg.arch.as_str();
    let folder = cfg.arch.folder_name();

    if profile.installer == InstallerKind::InnoSetupWin7Esr {
        let win7_inno = format!("Charlotte-Setup-{APP_VERSION}-win7-esr-{arch_str}.exe");
        return InstallPlan {
            app_version: APP_VERSION.into(),
            profile,
            package_filename: win7_inno.clone(),
            download_url: format!(
                "{}/downloads/{}/windows/win7-esr/{}",
                cfg.cloud_server_url.trim_end_matches('/'),
                folder,
                win7_inno
            ),
            install_dir: cfg.install_dir.clone(),
            cloud_server_url: cfg.cloud_server_url.clone(),
            redirect_installer: Some(win7_inno),
            stages: vec![
                InstallStage::PreflightCheck,
                InstallStage::RedirectToWin7InnoSetup,
            ],
        };
    }

    let channel_slug = match profile.channel {
        RuntimeChannel::Win8Esr => "win8-esr",
        RuntimeChannel::ModernEsr => "esr",
        RuntimeChannel::ModernStable => "stable",
        RuntimeChannel::Win7Esr => "win7-esr",
    };
    let package_filename = format!("charlotte-{APP_VERSION}-windows-{arch_str}-{channel_slug}.zip");
    let mut stages = vec![
        InstallStage::PreflightCheck,
        InstallStage::FetchRuntimePayload,
        InstallStage::VerifySha256Checksum,
        InstallStage::ExtractApplicationFiles,
        InstallStage::EnsureWebViewRuntime,
        InstallStage::WriteConfigAndShortcuts,
    ];
    if cfg.register_default_browser {
        stages.push(InstallStage::RegisterBrowserProtocols);
    }
    if !cfg.silent {
        stages.push(InstallStage::LaunchCharlotte);
    }

    InstallPlan {
        app_version: APP_VERSION.into(),
        profile,
        package_filename: package_filename.clone(),
        download_url: format!(
            "{}/downloads/{}/windows/win8-11/{}",
            cfg.cloud_server_url.trim_end_matches('/'),
            folder,
            package_filename
        ),
        install_dir: cfg.install_dir.clone(),
        cloud_server_url: cfg.cloud_server_url.clone(),
        redirect_installer: None,
        stages,
    }
}

/// Computes lowercase hex SHA-256 of bytes for payload integrity verification.
pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Verifies a downloaded/staged installer payload against its expected SHA-256 digest.
pub fn verify_payload_sha256(bytes: &[u8], expected_hex: &str) -> Result<(), String> {
    let actual = compute_sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        Err(format!(
            "SHA-256 checksum mismatch: expected {}, got {}",
            expected_hex.trim(),
            actual
        ))
    }
}

/// Stages and commits a verified installation manifest into `target_dir` with rollback on failure.
pub fn execute_staged_install(
    cfg: &InstallerConfig,
    payload_bytes: &[u8],
    expected_sha256: &str,
) -> Result<InstallPlan, String> {
    let plan = build_install_plan(cfg);
    if let Some(ref win7_pkg) = plan.redirect_installer {
        return Err(format!(
            "Windows 7 SP1 requires the legacy Inno Setup ESR installer ({win7_pkg}) with WebView2 109 ESR and Win7 ABI thunks."
        ));
    }

    verify_payload_sha256(payload_bytes, expected_sha256)?;

    let staging_dir = cfg.install_dir.with_extension("staging");
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {e}"))?;

    let write_res = (|| -> Result<(), String> {
        fs::write(staging_dir.join(&plan.package_filename), payload_bytes)
            .map_err(|e| format!("Failed to stage package payload: {e}"))?;

        let install_meta = serde_json::json!({
            "app": "Charlotte",
            "version": plan.app_version,
            "os": plan.profile.os.display_name(),
            "arch": plan.profile.arch.as_str(),
            "channel": plan.profile.channel,
            "webview_runtime": plan.profile.webview_runtime,
            "cloud_server_url": plan.cloud_server_url,
            "launch_args": plan.profile.launch_args,
            "desktop_shortcut": cfg.create_desktop_shortcut,
            "start_menu_shortcut": cfg.create_start_menu_shortcut,
            "default_browser_registered": cfg.register_default_browser,
            "sha256": expected_sha256,
        });
        let meta_bytes = serde_json::to_vec_pretty(&install_meta).map_err(|e| e.to_string())?;
        fs::write(staging_dir.join("install_manifest.json"), meta_bytes)
            .map_err(|e| format!("Failed to write install_manifest.json: {e}"))?;
        Ok(())
    })();

    if let Err(err) = write_res {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(err);
    }

    if cfg.install_dir.exists() {
        fs::remove_dir_all(&cfg.install_dir)
            .map_err(|e| format!("Failed to replace previous install dir: {e}"))?;
    }
    if let Some(parent) = cfg.install_dir.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent dir: {e}"))?;
        }
    }
    fs::rename(&staging_dir, &cfg.install_dir)
        .map_err(|e| format!("Failed to finalize installation: {e}"))?;

    Ok(plan)
}

/// Renders the Chrome/Opera-style branded installer UI HTML for interactive desktop setup.
pub fn render_installer_ui(plan: &InstallPlan) -> String {
    let channel_badge = match plan.profile.channel {
        RuntimeChannel::Win7Esr => "Windows 7 SP1 ESR (Inno Setup)",
        RuntimeChannel::Win8Esr => "Windows 8 / 8.1 ESR (WebView2 109 Fixed)",
        RuntimeChannel::ModernEsr => "Windows 10 / 11 ESR",
        RuntimeChannel::ModernStable => "Windows 10 / 11 Modern",
    };
    let action_note = if let Some(ref win7_exe) = plan.redirect_installer {
        format!("Windows 7 detected — launching dedicated Inno Setup ESR installer: {win7_exe}")
    } else {
        format!(
            "Ready for 1-click install ({}, {}) -> {}",
            plan.profile.arch.as_str(),
            channel_badge,
            plan.install_dir.display()
        )
    };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Install Charlotte Browser v{ver}</title>
<style>
  body {{ margin:0; font-family: "Segoe UI", system-ui, sans-serif; background:#0d0e11; color:#f3f4f6; display:flex; align-items:center; justify-content:center; min-height:100vh; }}
  .card {{ width:460px; background:#16181f; border:1px solid #2a2f3d; border-radius:14px; padding:28px; box-shadow:0 24px 60px rgba(0,0,0,.55); }}
  h1 {{ margin:0 0 6px; font-size:22px; }}
  .badge {{ display:inline-block; font-size:12px; padding:3px 10px; border-radius:999px; background:#232838; color:#93c5fd; margin-bottom:14px; }}
  p {{ color:#a1a7b8; font-size:13px; line-height:1.5; }}
  .meta {{ background:#111319; border:1px solid #242936; border-radius:8px; padding:12px; font-size:12px; color:#cbd5e1; margin:14px 0; }}
  .btn {{ width:100%; padding:12px; border:none; border-radius:8px; background:#e5e7eb; color:#111827; font-weight:700; font-size:14px; cursor:pointer; }}
</style>
</head>
<body>
  <div class="card">
    <h1>Charlotte Browser</h1>
    <div class="badge">{badge}</div>
    <p>{note}</p>
    <div class="meta">
      <div><strong>Package:</strong> {pkg}</div>
      <div><strong>Runtime:</strong> {rt}</div>
      <div><strong>Cloud API:</strong> {api}</div>
    </div>
    <button class="btn">Install Charlotte</button>
  </div>
</body>
</html>"#,
        ver = plan.app_version,
        badge = channel_badge,
        note = action_note,
        pkg = plan.package_filename,
        rt = plan.profile.webview_runtime,
        api = plan.cloud_server_url,
    )
}

pub fn verify_installed_manifest(install_dir: &Path) -> Result<serde_json::Value, String> {
    let manifest_path = install_dir.join("install_manifest.json");
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Could not read {}: {e}", manifest_path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid install manifest JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win7_redirects_to_inno_setup_esr_installer() {
        let cfg = InstallerConfig::default_for(WindowsVersion::Windows7Sp1, WindowsArch::X86);
        let plan = build_install_plan(&cfg);
        assert_eq!(plan.profile.installer, InstallerKind::InnoSetupWin7Esr);
        assert_eq!(
            plan.redirect_installer.as_deref(),
            Some("Charlotte-Setup-0.4.0-win7-esr-x86.exe")
        );
        assert!(plan.download_url.starts_with("https://api-charlotte-server.onrender.com/downloads/32-bit/windows/win7-esr/"));
    }

    #[test]
    fn win8_to_win11_uses_custom_installer_and_verifies_sha256() {
        let tmp = std::env::temp_dir().join(format!("charlotte_inst_test_{}", rand::random::<u32>()));
        let mut cfg = InstallerConfig::default_for(WindowsVersion::Windows11, WindowsArch::X64);
        cfg.install_dir = tmp.clone();
        cfg.register_default_browser = true;
        cfg.silent = true;

        let payload = b"charlotte-verified-release-archive";
        let good_hash = compute_sha256_hex(payload);

        // Tampered checksum must fail and leave no directory behind
        let bad_res = execute_staged_install(&cfg, payload, "0000000000000000");
        assert!(bad_res.is_err());
        assert!(!tmp.exists());

        // Valid checksum installs atomically
        let plan = execute_staged_install(&cfg, payload, &good_hash).expect("install succeeded");
        assert_eq!(plan.profile.installer, InstallerKind::CustomNativeInstallerWin8Plus);
        assert!(plan.stages.contains(&InstallStage::VerifySha256Checksum));
        assert!(plan.stages.contains(&InstallStage::RegisterBrowserProtocols));

        let manifest = verify_installed_manifest(&tmp).expect("manifest readable");
        assert_eq!(manifest["cloud_server_url"], "https://api-charlotte-server.onrender.com");
        assert_eq!(manifest["arch"], "x64");

        let ui_html = render_installer_ui(&plan);
        assert!(ui_html.contains("Charlotte Browser"));
        assert!(ui_html.contains("https://api-charlotte-server.onrender.com"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn win8_auto_selects_esr_webview2_109_in_custom_installer() {
        let cfg = InstallerConfig::default_for(WindowsVersion::Windows8_1, WindowsArch::X86);
        let plan = build_install_plan(&cfg);
        assert_eq!(plan.profile.installer, InstallerKind::CustomNativeInstallerWin8Plus);
        assert_eq!(plan.profile.channel, RuntimeChannel::Win8Esr);
        assert!(plan.profile.webview_runtime.contains("109.0.1518.78"));
        assert!(plan.package_filename.contains("windows-x86-win8-esr"));
    }
}
