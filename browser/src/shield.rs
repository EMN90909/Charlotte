//! ABP-compatible network and cosmetic rule engine.
//!
//! The embedder must invoke this on each CEF request and apply cosmetic
//! resources in the appropriate renderer context. This module alone does not
//! intercept requests or inject scripts.
use adblock::{
    cosmetic_filter_cache::UrlSpecificResources, lists::ParseOptions, request::Request, Engine,
    FilterSet,
};

/// A small offline starter list. Additional reviewed ABP-format lists can be
/// passed to `new`; they are compiled once rather than per request.
pub const STARTER_RULES: &str = "\
||doubleclick.net^\n\
||googlesyndication.com^\n\
||googleadservices.com^\n\
||adnxs.com^\n\
||criteo.com^\n\
||taboola.com^\n\
||outbrain.com^\n\
||pubmatic.com^\n\
||adsrvr.org^\n\
||google-analytics.com^\n";

pub struct Shield {
    engine: Engine,
    enabled: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Block,
    Rewrite(String),
}

impl Shield {
    pub fn new(lists: &[&str], enabled: bool) -> Self {
        let mut filters = FilterSet::new(false);
        for list in lists {
            filters.add_filter_list((*list).to_owned(), ParseOptions::default());
        }
        Self { engine: Engine::new_with_filter_set(filters), enabled }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Source URL must identify the originating page, not the resource URL.
    /// Malformed/non-web requests are allowed rather than crashing browsing.
    pub fn check(&self, url: &str, source_url: &str, resource_type: &str, method: &str) -> Decision {
        if !self.enabled { return Decision::Allow; }
        let Ok(request) = Request::new(url, source_url, resource_type, method) else {
            return Decision::Allow;
        };
        let result = self.engine.check_network_request(&request);
        if result.should_block() { return Decision::Block; }
        result.rewritten_url.map_or(Decision::Allow, Decision::Rewrite)
    }

    /// Scriptlet output is empty unless a trusted resource library is provided
    /// to the engine; do not execute JavaScript from untrusted filter lists.
    pub fn cosmetics(&self, url: &str) -> UrlSpecificResources {
        if self.enabled { self.engine.url_cosmetic_resources(url) }
        else { UrlSpecificResources::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_rules_respect_exceptions_and_domain_boundaries() {
        let shield = Shield::new(&["||ads.example.com^\n@@||ads.example.com/okay^"], true);
        let check = |url| shield.check(url, "https://news.example/", "script", "GET");
        assert_eq!(check("https://ads.example.com/ad.js"), Decision::Block);
        assert_eq!(check("https://ads.example.com/okay/a.js"), Decision::Allow);
        assert_eq!(check("https://ads.example.com/okay.js"), Decision::Block);
        assert_eq!(check("https://ads.example.com.evil.test/a.js"), Decision::Allow);
    }

    #[test]
    fn switch_disables_network_and_cosmetic_rules() {
        let mut shield = Shield::new(&["||ads.example^\nexample.com##.ad-banner"], true);
        assert!(shield.cosmetics("https://example.com/").hide_selectors.contains(".ad-banner"));
        shield.set_enabled(false);
        assert_eq!(shield.check("https://ads.example/a.js", "https://example.com/", "script", "GET"), Decision::Allow);
        assert!(shield.cosmetics("https://example.com/").hide_selectors.is_empty());
    }

    #[test]
    fn unrelated_music_images_and_javascript_are_allowed() {
        let shield = Shield::new(&[STARTER_RULES], true);
        for (url, kind) in [("https://cdn.example/music.mp3", "media"), ("https://cdn.example/image.png", "image"), ("https://cdn.example/app.js", "script")] {
            assert_eq!(shield.check(url, "https://example.com/", kind, "GET"), Decision::Allow);
        }
    }
}
