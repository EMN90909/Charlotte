//! Charlotte desktop runtime. CEF executes all browsing processes; no Electron.
pub mod app;
pub mod cef_bridge;
pub mod profile;
pub mod legacy;
pub mod services;
pub mod credentials;
pub mod shield;
pub mod shield_lists;
pub mod storage;
pub mod session;
pub mod tabs;
pub mod permissions;
pub mod downloads;
pub mod shortcuts;
pub mod newtab;
pub mod ui_server;
pub mod auth;
pub mod translate;
pub mod mcp;
pub mod mcp_catalog;
pub mod mcp_stdio;
pub mod mcp_oauth;
pub mod mcp_sqlite;
pub mod agent;
pub mod orchestrator;
pub mod browser_automation;
pub mod advanced_automation;
pub mod automation_semantics;
pub mod browser_agent;
pub mod browser_skills;
pub mod dark_mode;
pub mod extensions;
pub mod resource_manager;
pub mod engine;
pub mod cloud_client;
pub mod user_learning;
pub mod byok;
pub mod agent_skills;
pub mod windows_compat;
#[cfg(all(target_os = "windows", not(feature = "cef-engine")))]
pub mod windows_host;
pub mod installer;
pub mod updater;
pub mod internal_pages;
#[allow(special_module_name)]
pub mod main;
pub mod renderer;

#[cfg(all(target_os = "macos", feature = "cef-engine"))]
fn load_cef_library(helper: bool) -> cef::library_loader::LibraryLoader {
    let loader = cef::library_loader::LibraryLoader::new(
        &std::env::current_exe().expect("current executable"), helper
    );
    assert!(loader.load(), "Could not load Chromium Embedded Framework");
    loader
}

pub fn launch() -> Result<(), String> {
    #[cfg(feature = "cef-engine")]
    {
        #[cfg(target_os = "macos")]
        let _cef_library = load_cef_library(false);

        let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
        let args = cef::args::Args::new();
        let code = app::run(args.as_main_args(), std::ptr::null_mut());
        if code == 0 { Ok(()) } else { Err(format!("CEF exited with code {code}")) }
    }
    #[cfg(not(feature = "cef-engine"))]
    {
        app::run_platform()
    }
}

/// Windows CEF sandbox bootstrap entry point. The CEF packager provides
/// bootstrap.exe and loads this Rust cdylib with a valid sandbox_info.
#[cfg(all(target_os = "windows", feature = "sandbox", feature = "cef-engine"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn RunWinMain(
    instance: cef::sys::HINSTANCE,
    _command_line: *const u8,
    _command_show: i32,
    sandbox_info: *mut u8,
) -> i32 {
    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = cef::args::Args::from(cef::MainArgs { instance });
    app::run(args.as_main_args(), sandbox_info)
}
