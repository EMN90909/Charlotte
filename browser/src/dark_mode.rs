//! Forced dark mode engine for web page rendering.
//! Inverts light pages intelligently and enforces dark color schemes without
//! distorting images, videos, or existing dark websites.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DarkModePreference {
    Off,
    SmartInvert,
    HighContrast,
    ForceDarkScheme,
}

impl Default for DarkModePreference {
    fn default() -> Self {
        Self::Off
    }
}

impl DarkModePreference {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "off" | "disabled" => Some(Self::Off),
            "smart_invert" | "smart" => Some(Self::SmartInvert),
            "high_contrast" | "contrast" => Some(Self::HighContrast),
            "force_dark" | "dark" => Some(Self::ForceDarkScheme),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::SmartInvert => "smart_invert",
            Self::HighContrast => "high_contrast",
            Self::ForceDarkScheme => "force_dark",
        }
    }
}

/// Returns the CSS stylesheet content to be injected into web frames.
pub fn generate_dark_css(mode: DarkModePreference) -> &'static str {
    match mode {
        DarkModePreference::Off => "",
        DarkModePreference::SmartInvert => "\
            html {\
                filter: invert(90%) hue-rotate(180deg) !important;\
                background-color: #121212 !important;\
            }\
            img, video, canvas, svg, [style*='background-image'], [role='img'] {\
                filter: invert(100%) hue-rotate(180deg) !important;\
            }\
            iframe {\
                filter: invert(100%) hue-rotate(180deg) !important;\
            }",
        DarkModePreference::HighContrast => "\
            html {\
                filter: invert(100%) hue-rotate(180deg) contrast(110%) !important;\
                background-color: #000000 !important;\
                color: #ffffff !important;\
            }\
            img, video, canvas, svg, [style*='background-image'], [role='img'] {\
                filter: invert(100%) hue-rotate(180deg) !important;\
            }",
        DarkModePreference::ForceDarkScheme => "\
            :root {\
                color-scheme: dark !important;\
                --color-scheme: dark !important;\
                background-color: #121212 !important;\
                color: #e0e0e0 !important;\
            }\
            body {\
                background-color: #121212 !important;\
                color: #e0e0e0 !important;\
            }\
            a {\
                color: #8ab4f8 !important;\
            }",
    }
}

/// Script to check whether the active document is already dark-themed.
pub const SCRIPT_IS_PAGE_ALREADY_DARK: &str = r#"
(() => {
    try {
        const bg = window.getComputedStyle(document.body || document.documentElement).backgroundColor;
        const match = bg.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
        if (!match) return false;
        const r = parseInt(match[1], 10);
        const g = parseInt(match[2], 10);
        const b = parseInt(match[3], 10);
        const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        return luminance < 60;
    } catch (_) {
        return false;
    }
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_generate_dark_css() {
        assert_eq!(DarkModePreference::parse("smart"), Some(DarkModePreference::SmartInvert));
        assert_eq!(DarkModePreference::parse("off"), Some(DarkModePreference::Off));
        assert!(generate_dark_css(DarkModePreference::SmartInvert).contains("invert(90%)"));
        assert!(generate_dark_css(DarkModePreference::Off).is_empty());
    }

    #[test]
    fn protects_media_elements() {
        let css = generate_dark_css(DarkModePreference::SmartInvert);
        assert!(css.contains("img, video, canvas, svg"));
        assert!(css.contains("hue-rotate(180deg)"));
    }
}
