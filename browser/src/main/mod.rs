//! Rust-native modules replacing the historical Electron main process.
//! Entry points and browser logic live in the main Cargo crate.
pub use crate::{app, auth, cef_bridge, credentials, downloads, legacy, permissions, profile, services, session, shield, storage, tabs, translate, ui_server};
