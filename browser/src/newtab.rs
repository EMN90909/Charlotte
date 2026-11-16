//! Local Charlotte start page, generated entirely by Rust with no Node,
//! Electron, external scripts, privileged bridge or JavaScript execution.
//! Navigation and search are ordinary HTTPS links/forms handled by CEF.
use crate::storage::{Bookmark, Storage};
use std::fmt::Write;

fn escape_html(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(ch),
        }
    }
    result
}
pub fn render(bookmarks: &[Bookmark]) -> String {
    let mut page = String::from(r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; form-action https://www.google.com; base-uri 'none'">
<title>New tab — Charlotte</title>
<style>
:root{color-scheme:dark;--ink:#f5f5f5;--muted:#a3a3a3;--paper:#0e0e0e;--surface:#1b1b1b;--line:#303030;--accent:#dedede}
*{box-sizing:border-box}body{margin:0;background:var(--paper);color:var(--ink);font-family:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
main{max-width:850px;margin:0 auto;padding:clamp(32px,7vw,80px) 22px 90px}
.eyebrow{font-size:12px;text-transform:uppercase;letter-spacing:.22em;font-weight:650;color:var(--accent)}
h1{font-size:clamp(54px,9vw,100px);line-height:1.04;font-weight:480;letter-spacing:-.065em;margin:16px 0 20px}
p{line-height:1.6;color:var(--muted);margin:0 0 40px}
form{display:flex;align-items:center;gap:12px;background:var(--surface);border:1px solid var(--line);border-radius:4px;padding:14px 18px}
form:focus-within{border-color:var(--accent)}
input{font-size:18px;color:var(--ink);background:transparent;outline:0;border:0;flex:1;min-width:0}
input::placeholder{color:#aaa393}
button{cursor:pointer;background:var(--accent);border:0;color:#201c18;padding:10px 18px;border-radius:4px;font-weight:700}
section{margin-top:56px}h2{font-size:15px;font-weight:600;letter-spacing:.02em;margin:0 0 20px}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(175px,1fr));gap:12px}
a{display:flex;min-height:78px;align-items:center;color:var(--ink);text-decoration:none;padding:16px;background:var(--surface);border:1px solid var(--line);border-radius:4px;overflow-wrap:anywhere}
a:hover,a:focus-visible{border-color:var(--accent);background:#333128}
.small{font-size:13px;color:var(--muted);margin-top:18px}
</style></head><body><main><div class="eyebrow">Charlotte · Native</div><h1>A place to begin.</h1>
<p>Search, explore, and return to what matters.</p>
<form action="https://www.google.com/search" method="get">
  <input type="search" name="q" aria-label="Search the web" placeholder="Search the web" autofocus required>
  <button type="submit">Search</button>
</form>
<section><h2>Bookmarks</h2><div class="grid">"#);
    let mut count = 0;
    for entry in bookmarks.iter().filter(|b| crate::session::valid_page_url(&b.url)) {
        if count >= 24 { break; }
        let title = escape_html(&entry.title.chars().take(120).collect::<String>());
        let label = if title.trim().is_empty() { escape_html(&entry.url) } else { title };
        let _ = write!(page, r#"<a rel="noopener noreferrer" href="{}">{}</a>"#, escape_html(&entry.url), label);
        count += 1;
    }
    if count == 0 {
        page.push_str(r#"<p class="small">Saved pages will appear here. Press Ctrl+D (Cmd+D on Mac) to bookmark any website.</p>"#);
    }
    page.push_str(r#"</div></section><p class="small">Rendered locally by the Charlotte Rust host. Powered by Chromium Embedded Framework.</p></main></body></html>"#);
    page
}
/// A data: URL is an opaque origin. The page has no host API access and
/// contains only a constrained search form and validated HTTP(S) links.
pub fn start_url(db: &Storage) -> String {
    let bookmarks = db.bookmarks().unwrap_or_default();
    let html = render(&bookmarks);
    let payload: String = url::form_urlencoded::byte_serialize(html.as_bytes()).collect();
    format!("data:text/html;charset=utf-8,{payload}")
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn escapes_bookmark_titles_and_urls() {
        let html = render(&[
            Bookmark {url:"https://example.org/a?x=1&y=2".into(),title:"<script>alert(1)</script>".into()},
            Bookmark {url:"javascript:alert(1)".into(),title:"Bad".into()},
        ]);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("https://example.org/a?x=1&amp;y=2"));
        assert!(!html.contains("javascript:alert(1)"));
    }
    #[test]
    fn creates_self_contained_data_url() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        let url = start_url(&db);
        assert!(url.starts_with("data:text/html;charset=utf-8,"));
        assert!(url.len() > 1000);
    }
}
