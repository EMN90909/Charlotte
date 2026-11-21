//! CEF renderer/utility process helper required by macOS app bundles.
#[cfg(feature = "cef-engine")]
use cef::{args::Args, *};

#[cfg(feature = "cef-engine")]
fn main() {
    let args = Args::new();

    #[cfg(all(target_os = "macos", feature = "sandbox"))]
    let _sandbox = {
        let mut sandbox = sandbox::Sandbox::new();
        sandbox.initialize(args.as_main_args());
        sandbox
    };

    #[cfg(target_os = "macos")]
    let _loader = {
        let loader = library_loader::LibraryLoader::new(
            &std::env::current_exe().expect("current executable"), true
        );
        assert!(loader.load(), "CEF helper failed to load framework");
        loader
    };

    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    execute_process(Some(args.as_main_args()), None::<&mut App>, std::ptr::null_mut());
}

#[cfg(not(feature = "cef-engine"))]
fn main() {
    println!("CEF helper not used in platform web engine builds.");
}
