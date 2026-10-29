//! Platform-neutral browser keyboard shortcuts. CEF sends Windows-style
//! virtual key codes even on macOS/Linux. Test the mapping separately from
//! the browser renderer and never intercept ordinary text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut { Bookmark, Back, Forward, Reload, HardReload }
const SHIFT: u32 = 1 << 1;
const CTRL: u32 = 1 << 2;
const ALT: u32 = 1 << 3;
const COMMAND: u32 = 1 << 7;
pub fn resolve(key: i32, modifiers: u32, raw_key_down: bool) -> Option<Shortcut> {
    if !raw_key_down { return None }
    let primary = modifiers & (CTRL | COMMAND) != 0;
    let alt = modifiers & ALT != 0;
    let shift = modifiers & SHIFT != 0;
    // Never capture a browser keyboard shortcut that includes both primary
    // and Alt: those are reserved for system/window manager actions.
    if primary && alt { return None }
    match (key, primary, alt, shift) {
        (0x44, true, false, false) => Some(Shortcut::Bookmark), // Ctrl/Cmd+D
        (0x25, false, true, false) => Some(Shortcut::Back), // Alt+Left
        (0x27, false, true, false) => Some(Shortcut::Forward), // Alt+Right
        (0x74, false, false, false) | (0x52, true, false, false) =>
            Some(Shortcut::Reload), // F5 or Ctrl/Cmd+R
        (0x74, false, false, true) | (0x52, true, false, true) =>
            Some(Shortcut::HardReload),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_shortcuts() {
        assert_eq!(resolve(0x44, CTRL, true), Some(Shortcut::Bookmark));
        assert_eq!(resolve(0x44, COMMAND, true), Some(Shortcut::Bookmark));
        assert_eq!(resolve(0x25, ALT, true), Some(Shortcut::Back));
        assert_eq!(resolve(0x27, ALT, true), Some(Shortcut::Forward));
        assert_eq!(resolve(0x74, 0, true), Some(Shortcut::Reload));
        assert_eq!(resolve(0x52, CTRL|SHIFT, true), Some(Shortcut::HardReload));
    }
    #[test]
    fn does_not_capture_typing_or_key_up() {
        assert_eq!(resolve(0x44, 0, true), None);
        assert_eq!(resolve(0x44, CTRL, false), None);
        assert_eq!(resolve(0x44, CTRL|ALT, true), None);
    }
}
