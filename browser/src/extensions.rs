//! WebExtension support for Charlotte browser.
//! Parses Manifest V2 / V3, matches content script rules, and coordinates
//! script and style injection into rendered web pages.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunAt {
    DocumentStart,
    DocumentEnd,
    DocumentIdle,
}

impl Default for RunAt {
    fn default() -> Self {
        Self::DocumentEnd
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentScriptRule {
    pub matches: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub run_at: RunAt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundSpec {
    #[serde(default)]
    pub service_worker: Option<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub content_scripts: Vec<ContentScriptRule>,
    #[serde(default)]
    pub background: Option<BackgroundSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    pub id: String,
    pub manifest: ExtensionManifest,
    pub enabled: bool,
    pub files: HashMap<String, String>,
}

/// Matches a WebExtension pattern against a candidate URL.
/// Supports <all_urls>, schemes (*, http, https), domain wildcards (*.example.com), and path wildcards.
pub fn matches_pattern(pattern: &str, candidate_url: &str) -> bool {
    if pattern == "<all_urls>" {
        return candidate_url.starts_with("http://") || candidate_url.starts_with("https://");
    }
    let Ok(target) = Url::parse(candidate_url) else { return false };
    let scheme_end = match pattern.find("://") {
        Some(idx) => idx,
        None => return false,
    };
    let pattern_scheme = &pattern[..scheme_end];
    let remainder = &pattern[scheme_end + 3..];

    // Check scheme
    if pattern_scheme != "*" && pattern_scheme != target.scheme() {
        return false;
    }

    // Split host and path
    let (pattern_host, pattern_path) = match remainder.find('/') {
        Some(idx) => (&remainder[..idx], &remainder[idx..]),
        None => (remainder, "/*"),
    };

    let target_host = target.host_str().unwrap_or("");
    // Check host
    if pattern_host != "*" {
        if let Some(suffix) = pattern_host.strip_prefix("*.") {
            if target_host != suffix && !target_host.ends_with(&format!(".{suffix}")) {
                return false;
            }
        } else if pattern_host != target_host {
            return false;
        }
    }

    // Check path
    if pattern_path == "/*" || pattern_path == "*" {
        return true;
    }
    let target_path = target.path();
    if let Some(prefix) = pattern_path.strip_suffix('*') {
        target_path.starts_with(prefix)
    } else {
        target_path == pattern_path
    }
}

#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, Extension>,
    storage: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, id: &str, manifest_json: &str) -> Result<String, String> {
        let manifest: ExtensionManifest = serde_json::from_str(manifest_json)
            .map_err(|e| format!("Invalid extension manifest: {e}"))?;
        let id = id.to_owned();
        self.extensions.insert(id.clone(), Extension {
            id: id.clone(),
            manifest,
            enabled: true,
            files: HashMap::new(),
        });
        Ok(id)
    }

    pub fn set_file_content(&mut self, ext_id: &str, file_name: &str, content: &str) {
        if let Some(ext) = self.extensions.get_mut(ext_id) {
            ext.files.insert(file_name.to_owned(), content.to_owned());
        }
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(ext) = self.extensions.get_mut(id) {
            ext.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.extensions.get(id).map_or(false, |e| e.enabled)
    }

    pub fn list(&self) -> Vec<&Extension> {
        self.extensions.values().collect()
    }

    /// Finds matching content script code for a target page and injection timing.
    pub fn matching_scripts(&self, target_url: &str, timing: RunAt) -> Vec<String> {
        let mut scripts = Vec::new();
        for ext in self.extensions.values() {
            if !ext.enabled { continue; }
            for rule in &ext.manifest.content_scripts {
                if rule.run_at != timing { continue; }
                let matches_any = rule.matches.iter().any(|pattern| matches_pattern(pattern, target_url));
                if matches_any {
                    for js_file in &rule.js {
                        if let Some(content) = ext.files.get(js_file) {
                            scripts.push(content.clone());
                        }
                    }
                }
            }
        }
        scripts
    }

    /// Storage API: get value
    pub fn storage_get(&self, ext_id: &str, key: &str) -> Option<&serde_json::Value> {
        self.storage.get(ext_id).and_then(|m| m.get(key))
    }

    /// Storage API: set value
    pub fn storage_set(&mut self, ext_id: &str, key: &str, value: serde_json::Value) {
        self.storage.entry(ext_id.to_owned()).or_default().insert(key.to_owned(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_pattern_all_urls() {
        assert!(matches_pattern("<all_urls>", "https://example.com/page"));
        assert!(matches_pattern("<all_urls>", "http://localhost:8080"));
        assert!(!matches_pattern("<all_urls>", "file:///etc/passwd"));
    }

    #[test]
    fn match_pattern_wildcards() {
        assert!(matches_pattern("https://*.google.com/*", "https://mail.google.com/mail"));
        assert!(matches_pattern("https://*.google.com/*", "https://google.com/"));
        assert!(!matches_pattern("https://*.google.com/*", "http://mail.google.com/"));
        assert!(!matches_pattern("https://*.google.com/*", "https://evil-google.com/"));
    }

    #[test]
    fn register_and_retrieve_scripts() {
        let mut registry = ExtensionRegistry::new();
        let manifest = r#"{
            "manifest_version": 3,
            "name": "Ad Shield Helper",
            "version": "1.0.0",
            "content_scripts": [
                {
                    "matches": ["https://*.example.com/*"],
                    "js": ["content.js"],
                    "run_at": "document_end"
                }
            ]
        }"#;
        let ext_id = registry.register("helper-ext", manifest).unwrap();
        registry.set_file_content(&ext_id, "content.js", "console.log('helper injected');");

        let scripts = registry.matching_scripts("https://sub.example.com/test", RunAt::DocumentEnd);
        assert_eq!(scripts, vec!["console.log('helper injected');".to_string()]);

        let not_matched = registry.matching_scripts("https://other.com/test", RunAt::DocumentEnd);
        assert!(not_matched.is_empty());
    }
}
