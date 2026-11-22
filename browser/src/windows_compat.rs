//! Windows 7–11 Compatibility Matrix, ESR Runtime Selector, and Win7 ABI Import Auditor.
//!
//! Important architectural reality:
//! Setting `<supportedOS Id="{35138b9a-...}"/>` in `app.manifest` and `MinVersion=6.1sp1` in Inno Setup
//! ONLY sets OS compatibility metadata. To genuinely boot and render pages on Windows 7 SP1 (32-bit & 64-bit):
//! 1. **Web engine**: Modern Evergreen WebView2 (>= 110) and CEF (>= 110) dropped Windows 7/8/8.1.
//!    Windows 7 SP1 and Windows 8/8.1 MUST run in **ESR mode** using **WebView2 Fixed Version 109.0.1518.78 ESR**
//!    or **CEF 109 ESR** (never Evergreen WebView2 >= 110).
//! 2. **ABI / DLL imports**: Modern Rust (>= 1.78) links Win8+ APIs (`WaitOnAddress`, `ProcessPrng`,
//!    `GetSystemTimePreciseAsFileTime`, `api-ms-win-core-synch-l1-2-0.dll`) unless built with a Win7-compatible
//!    Rust toolchain (1.75.0 / `i686-pc-windows-msvc` or `x86_64-pc-windows-msvc`) or linked with `YY-Thunks`
//!    / `VC-LTL5` API thunks.
//! 3. **Installer split**:
//!    - Windows 7 SP1 (32-bit & 64-bit ESR) uses the dedicated **Inno Setup** installer (`charlotte.iss`)
//!      bundling the Win7 ESR runtime and verifying SP1 / KB3063858 prerequisites.
//!    - Windows 8, 8.1, 10, and 11 (32-bit & 64-bit, ESR & Standard modes) use Charlotte's custom
//!      Opera/Chrome-style native installer (`charlotte-installer`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowsVersion {
    Windows7Sp1,
    Windows8,
    Windows8_1,
    Windows10,
    Windows11,
}

impl WindowsVersion {
    /// Parses a Windows NT `(major, minor, build)` version tuple.
    pub fn from_nt_version(major: u32, minor: u32, build: u32) -> Option<Self> {
        match (major, minor) {
            (6, 1) => Some(Self::Windows7Sp1),
            (6, 2) => Some(Self::Windows8),
            (6, 3) => Some(Self::Windows8_1),
            (10, 0) if build >= 22000 => Some(Self::Windows11),
            (10, 0) => Some(Self::Windows10),
            _ if major > 10 => Some(Self::Windows11),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Windows7Sp1 => "Windows 7 SP1",
            Self::Windows8 => "Windows 8",
            Self::Windows8_1 => "Windows 8.1",
            Self::Windows10 => "Windows 10",
            Self::Windows11 => "Windows 11",
        }
    }

    pub fn requires_esr_engine(&self) -> bool {
        matches!(self, Self::Windows7Sp1 | Self::Windows8 | Self::Windows8_1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowsArch {
    X86,
    X64,
    Arm64,
}

impl WindowsArch {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X64 => "x64",
            Self::Arm64 => "arm64",
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            Self::X86 => "32-bit",
            Self::X64 | Self::Arm64 => "64-bit",
        }
    }

    pub fn rust_target(&self) -> &'static str {
        match self {
            Self::X86 => "i686-pc-windows-msvc",
            Self::X64 => "x86_64-pc-windows-msvc",
            Self::Arm64 => "aarch64-pc-windows-msvc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeChannel {
    /// Windows 7 SP1 ESR (32-bit & 64-bit): WebView2 Fixed Version 109.0.1518.78 ESR / CEF 109 ESR + YY-Thunks ABI
    Win7Esr,
    /// Windows 8 / 8.1 ESR (32-bit & 64-bit): WebView2 Fixed Version 109.0.1518.78 ESR / CEF 109 ESR
    Win8Esr,
    /// Windows 10 / 11 Modern (32-bit & 64-bit): Evergreen WebView2 / modern CEF
    ModernStable,
    /// Windows 10 / 11 Enterprise ESR mode (pinned long-term Chromium/WebView2 release)
    ModernEsr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallerKind {
    /// Legacy Inno Setup package specifically for Windows 7 SP1 (32-bit & 64-bit ESR)
    InnoSetupWin7Esr,
    /// Custom Opera/Chrome-style native installer for Windows 8, 8.1, 10, and 11 (32-bit & 64-bit)
    CustomNativeInstallerWin8Plus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsRuntimeProfile {
    pub os: WindowsVersion,
    pub arch: WindowsArch,
    pub channel: RuntimeChannel,
    pub installer: InstallerKind,
    pub webview_runtime: String,
    pub requires_win7_abi_thunks: bool,
    pub tls_provider: String,
    pub launch_args: Vec<String>,
}

/// Resolves the exact runtime engine, ESR mode, ABI requirements, and installer type
/// for any Windows 7–11 target across 32-bit (x86) and 64-bit (x64/arm64).
pub fn resolve_runtime_profile(
    os: WindowsVersion,
    arch: WindowsArch,
    prefer_esr: bool,
) -> WindowsRuntimeProfile {
    match os {
        WindowsVersion::Windows7Sp1 => WindowsRuntimeProfile {
            os,
            arch,
            channel: RuntimeChannel::Win7Esr,
            installer: InstallerKind::InnoSetupWin7Esr,
            webview_runtime: format!(
                "Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.{}",
                arch.as_str()
            ),
            requires_win7_abi_thunks: true,
            tls_provider: "rustls-tls12-bundled".into(),
            launch_args: vec![
                "--esr-mode=win7".into(),
                "--disable-gpu-sandbox-win7-fallback".into(),
                "--low-ram-mode".into(),
            ],
        },
        WindowsVersion::Windows8 | WindowsVersion::Windows8_1 => WindowsRuntimeProfile {
            os,
            arch,
            channel: RuntimeChannel::Win8Esr,
            installer: InstallerKind::CustomNativeInstallerWin8Plus,
            webview_runtime: format!(
                "Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.{}",
                arch.as_str()
            ),
            requires_win7_abi_thunks: false,
            tls_provider: "rustls-tls13".into(),
            launch_args: vec!["--esr-mode=win8".into()],
        },
        WindowsVersion::Windows10 | WindowsVersion::Windows11 => {
            let channel = if prefer_esr {
                RuntimeChannel::ModernEsr
            } else {
                RuntimeChannel::ModernStable
            };
            let runtime = if prefer_esr {
                format!("Microsoft.WebView2.FixedVersionRuntime.ESR.{}", arch.as_str())
            } else {
                format!("Microsoft.WebView2.Evergreen.{}", arch.as_str())
            };
            let launch_args = if prefer_esr {
                vec!["--esr-mode=modern".into()]
            } else {
                vec![]
            };
            WindowsRuntimeProfile {
                os,
                arch,
                channel,
                installer: InstallerKind::CustomNativeInstallerWin8Plus,
                webview_runtime: runtime,
                requires_win7_abi_thunks: false,
                tls_provider: "rustls-tls13".into(),
                launch_args,
            }
        }
    }
}

/// Windows 8 / 10 / 11 DLLs and API symbols that MUST NOT appear as hard imports
/// in compiled Windows 7 SP1 ESR binaries (`dist/32-bit/windows/win7-esr` and `dist/64-bit/windows/win7-esr`).
pub const WIN7_FORBIDDEN_IMPORTS: &[&str] = &[
    "api-ms-win-core-synch-l1-2-0.dll",
    "api-ms-win-core-winrt-l1-1-0.dll",
    "api-ms-win-core-winrt-string-l1-1-0.dll",
    "api-ms-win-shcore-scaling-l1-1-1.dll",
    "bcryptprimitives.dll",
    "WaitOnAddress",
    "WakeByAddressSingle",
    "WakeByAddressAll",
    "GetSystemTimePreciseAsFileTime",
    "ProcessPrng",
    "SetThreadDescription",
    "GetDpiForWindow",
    "SetProcessDpiAwarenessContext",
    "RoInitialize",
    "RoGetActivationFactory",
    "CreateFile2",
    "DiscardVirtualMemory",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Win7ImportAuditReport {
    pub compatible: bool,
    pub violations: Vec<String>,
}

/// Audits a list of PE imported DLL names / symbol names against `WIN7_FORBIDDEN_IMPORTS`.
/// Returns `compatible = false` if any Win8+/Win10+ hard import is present.
pub fn audit_win7_imports(imported_symbols: &[&str]) -> Win7ImportAuditReport {
    let mut violations = Vec::new();
    for sym in imported_symbols {
        let lower_sym = sym.to_lowercase();
        for forbidden in WIN7_FORBIDDEN_IMPORTS {
            if lower_sym == forbidden.to_lowercase() {
                violations.push((*sym).to_string());
            }
        }
    }
    Win7ImportAuditReport {
        compatible: violations.is_empty(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_windows_7_to_11_32bit_and_64bit_profiles() {
        // Windows 7 SP1 (32-bit & 64-bit) -> Inno Setup + Fixed WebView2 109 ESR + Win7 ABI thunks
        for arch in [WindowsArch::X86, WindowsArch::X64] {
            let p = resolve_runtime_profile(WindowsVersion::Windows7Sp1, arch, false);
            assert_eq!(p.channel, RuntimeChannel::Win7Esr);
            assert_eq!(p.installer, InstallerKind::InnoSetupWin7Esr);
            assert!(p.requires_win7_abi_thunks);
            assert!(p.webview_runtime.contains("109.0.1518.78"));
        }

        // Windows 8 / 8.1 -> Custom Opera/Chrome-style installer + Fixed WebView2 109 ESR
        for os in [WindowsVersion::Windows8, WindowsVersion::Windows8_1] {
            let p = resolve_runtime_profile(os, WindowsArch::X64, false);
            assert_eq!(p.channel, RuntimeChannel::Win8Esr);
            assert_eq!(p.installer, InstallerKind::CustomNativeInstallerWin8Plus);
            assert!(!p.requires_win7_abi_thunks);
            assert!(p.webview_runtime.contains("109.0.1518.78"));
        }

        // Windows 10 / 11 -> Custom installer + Modern Stable or Modern ESR
        let win10_stable = resolve_runtime_profile(WindowsVersion::Windows10, WindowsArch::X86, false);
        assert_eq!(win10_stable.installer, InstallerKind::CustomNativeInstallerWin8Plus);
        assert_eq!(win10_stable.channel, RuntimeChannel::ModernStable);
        assert!(win10_stable.webview_runtime.contains("Evergreen"));

        let win11_esr = resolve_runtime_profile(WindowsVersion::Windows11, WindowsArch::X64, true);
        assert_eq!(win11_esr.installer, InstallerKind::CustomNativeInstallerWin8Plus);
        assert_eq!(win11_esr.channel, RuntimeChannel::ModernEsr);
        assert!(win11_esr.launch_args.contains(&"--esr-mode=modern".to_string()));
    }

    #[test]
    fn nt_version_parsing_covers_win7_through_win11() {
        assert_eq!(WindowsVersion::from_nt_version(6, 1, 7601), Some(WindowsVersion::Windows7Sp1));
        assert_eq!(WindowsVersion::from_nt_version(6, 2, 9200), Some(WindowsVersion::Windows8));
        assert_eq!(WindowsVersion::from_nt_version(6, 3, 9600), Some(WindowsVersion::Windows8_1));
        assert_eq!(WindowsVersion::from_nt_version(10, 0, 19045), Some(WindowsVersion::Windows10));
        assert_eq!(WindowsVersion::from_nt_version(10, 0, 22631), Some(WindowsVersion::Windows11));
        assert_eq!(WindowsVersion::from_nt_version(5, 1, 2600), None);
    }

    #[test]
    fn win7_pe_import_audit_catches_modern_apis() {
        let clean = audit_win7_imports(&["kernel32.dll", "user32.dll", "CreateFileW", "CryptGenRandom"]);
        assert!(clean.compatible);
        assert!(clean.violations.is_empty());

        let broken = audit_win7_imports(&[
            "kernel32.dll",
            "api-ms-win-core-synch-l1-2-0.dll",
            "WaitOnAddress",
            "ProcessPrng",
        ]);
        assert!(!broken.compatible);
        assert_eq!(broken.violations.len(), 3);
    }
}
