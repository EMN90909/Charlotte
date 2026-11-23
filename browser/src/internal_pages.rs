//! Pure-Rust server-rendered UI for Charlotte's unified Search, Agent, Private,
//! Active Work, and internal `charlotte://` pages (updates, history, settings,
//! bookmarks, downloads, extensions, about).
//!
//! Zero inline JavaScript — all interactions use CSRF-protected HTML forms.
//! Zero fake placeholders — pinned sites, open tabs, visited pages, and previous
//! agent work strictly reflect real persisted user state.
use crate::{
    byok::MaskedProviderKey,
    storage::{AgentWorkSession, Bookmark, PinnedSite, Visit},
    tabs::{Tab, TabLayout},
    updater::BrowserUpdate,
};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct PageContext {
    pub token: String,
    pub tabs: Vec<Tab>,
    pub active_tab_id: Option<i32>,
    pub pinned_sites: Vec<PinnedSite>,
    pub is_private: bool,
    pub cloud_mode_enabled: bool,
    pub selected_model: String,
    pub tab_filter: String,
    pub shields_enabled: bool,
    pub blocked_requests: u64,
    pub user_initials: String,
    pub sidebar_auto_hide: bool,
    pub current_internal_url: String,
    pub status_banner: Option<String>,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn hidden(name: &str, value: &str) -> String {
    format!(
        r#"<input type="hidden" name="{}" value="{}">"#,
        esc(name),
        esc(value)
    )
}

pub fn domain_from_url(raw: &str) -> String {
    if let Ok(parsed) = url::Url::parse(raw) {
        if let Some(host) = parsed.host_str() {
            return host.trim_start_matches("www.").to_owned();
        }
    }
    raw.to_owned()
}

pub fn favicon_url_for(raw: &str) -> String {
    if let Ok(parsed) = url::Url::parse(raw) {
        if matches!(parsed.scheme(), "http" | "https") {
            if let Some(host) = parsed.host_str() {
                return format!("{}://{}/favicon.ico", parsed.scheme(), host);
            }
        }
    }
    String::new()
}

fn initial_letter(title: &str, url: &str) -> String {
    let source = if !title.trim().is_empty() {
        title.trim()
    } else {
        url.trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
    };
    source
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "C".into())
}

fn format_relative_time(timestamp_ms: i64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let diff_secs = ((now_ms - timestamp_ms) / 1000).max(0);
    if diff_secs < 60 {
        "Just now".into()
    } else if diff_secs < 3600 {
        let m = diff_secs / 60;
        format!("{m} min ago")
    } else if diff_secs < 86_400 {
        let h = diff_secs / 3600;
        if h == 1 {
            "1 hour ago".into()
        } else {
            format!("{h} hours ago")
        }
    } else {
        let d = diff_secs / 86_400;
        if d == 1 {
            "Yesterday".into()
        } else {
            format!("{d} days ago")
        }
    }
}

fn format_short_date(timestamp_ms: i64) -> String {
    // Convert unix seconds to concise Month Day string (UTC)
    let secs = (timestamp_ms / 1000).max(0) as u64;
    let days = (secs / 86_400) as i64;
    // Civil date from days since 1970-01-01 (Howard Hinnant algorithm)
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let month_name = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    };
    format!("{month_name} {day}")
}

fn icon_incognito() -> &'static str {
    r#"<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 11h18"/><path d="M6 11l1.8-5.4A2 2 0 0 1 9.7 4h4.6a2 2 0 0 1 1.9 1.6L18 11"/><circle cx="8" cy="16.5" r="2.5"/><circle cx="16" cy="16.5" r="2.5"/><path d="M10.5 16.5h3"/></svg>"#
}

fn icon_search(size: u32) -> String {
    format!(
        r#"<svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>"#
    )
}

fn icon_mic() -> &'static str {
    r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="2" width="6" height="11" rx="3"/><path d="M5 10v1a7 7 0 0 0 14 0v-1"/><path d="M12 18v4"/><path d="M9 22h6"/></svg>"#
}

fn icon_clock() -> &'static str {
    r#"<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l3 2"/></svg>"#
}

fn icon_cloud() -> &'static str {
    r#"<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/></svg>"#
}

fn icon_folder() -> &'static str {
    r#"<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/></svg>"#
}

fn icon_file() -> &'static str {
    r#"<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z"/><path d="M14 2v6h6"/></svg>"#
}

fn icon_send_plane() -> &'static str {
    r#"<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M22 2 11 13"/><path d="m22 2-7 20-4-9-9-4Z"/></svg>"#
}

fn icon_arrow_up() -> &'static str {
    r#"<svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 19V5"/><path d="m5 12 7-7 7 7"/></svg>"#
}

fn base_styles(is_private: bool) -> String {
    if is_private {
        r#"
:root{
  color-scheme:dark;
  --bg:#111113;
  --sidebar:#18181b;
  --surface:#202024;
  --surface-hover:#29292f;
  --card:#1d1d21;
  --card-border:#2e2e35;
  --pill-bg:#222227;
  --pill-active:#2d2d34;
  --text:#f4f4f6;
  --text-muted:#9a9aa6;
  --text-subtle:#6f6f7b;
  --divider:#27272d;
  --accent-bar:#f4f4f6;
  --btn-solid-bg:#ffffff;
  --btn-solid-fg:#111113;
  --blue-btn:#3b6ef5;
  --shadow:0 8px 28px rgba(0,0,0,.45);
}
"#
        .to_owned()
    } else {
        r#"
:root{
  color-scheme:light;
  --bg:#ffffff;
  --sidebar:#eef0f3;
  --surface:#ffffff;
  --surface-hover:#f5f6f8;
  --card:#f3f4f6;
  --card-border:#d7dce2;
  --pill-bg:#ebedf0;
  --pill-active:#ffffff;
  --text:#111827;
  --text-muted:#6b7280;
  --text-subtle:#8a93a2;
  --divider:#e1e5eb;
  --accent-bar:#4b5563;
  --btn-solid-bg:#0d0e11;
  --btn-solid-fg:#ffffff;
  --blue-btn:#3b6ef5;
  --shadow:0 6px 24px rgba(15,23,42,.08);
}
"#
        .to_owned()
    }
}

fn shared_css(is_private: bool) -> String {
    let vars = base_styles(is_private);
    format!(
        r#"{vars}
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0;height:100%;background:var(--bg);color:var(--text);font-family:-apple-system,BlinkMacSystemFont,"Inter","Segoe UI",Roboto,sans-serif;-webkit-font-smoothing:antialiased}}
a{{color:inherit;text-decoration:none}}
button,input,textarea,select{{font:inherit;color:inherit}}
.shell{{display:flex;min-height:100vh;width:100%}}

/* Left Sidebar Rail (matches reference images 1-4) */
.rail{{
  width:214px;min-width:214px;background:var(--sidebar);
  border-right:1px solid var(--divider);
  display:flex;flex-direction:column;padding:18px 14px 14px;
  gap:14px;position:sticky;top:0;height:100vh;overflow:visible;
}}
.rail-label{{
  font-size:11px;font-weight:600;letter-spacing:.08em;color:var(--text-muted);
  display:flex;align-items:center;justify-content:space-between;padding:0 4px;
}}
.pinned-grid{{
  display:grid;grid-template-columns:repeat(2,1fr);gap:10px;
}}
.pin-tile{{
  position:relative;display:flex;align-items:center;justify-content:center;
  height:56px;border-radius:16px;background:var(--surface);
  border:1px solid var(--divider);box-shadow:0 1px 4px rgba(0,0,0,.04);
  transition:transform .12s ease,border-color .12s ease;
}}
.pin-tile:hover{{transform:translateY(-1px);border-color:var(--text-muted)}}
.pin-icon{{
  width:26px;height:26px;border-radius:8px;display:flex;align-items:center;
  justify-content:center;font-weight:700;font-size:13px;background:var(--card);color:var(--text);
  overflow:hidden;
}}
.pin-icon img{{width:20px;height:20px;object-fit:contain}}
.pin-unpin{{
  position:absolute;top:3px;right:5px;border:0;background:transparent;
  color:var(--text-subtle);font-size:11px;cursor:pointer;padding:2px 4px;opacity:.6;
}}
.pin-unpin:hover{{opacity:1;color:var(--text)}}
.rail-search{{
  display:flex;align-items:center;gap:8px;background:var(--surface);
  border:1px solid var(--divider);border-radius:12px;padding:8px 11px;
}}
.rail-search input{{
  border:0;outline:0;background:transparent;width:100%;font-size:13px;color:var(--text);
}}
.rail-search input::placeholder{{color:var(--text-subtle)}}
.rail-divider{{height:1px;background:var(--divider);margin:2px 0}}
.tabs-list{{display:flex;flex-direction:column;gap:4px;flex:1;min-height:40px;overflow:auto}}
.tab-row{{
  position:relative;display:flex;align-items:center;gap:6px;
  border-radius:13px;padding:7px 9px;transition:background .12s ease;
}}
.tab-row:hover{{background:var(--surface-hover)}}
.tab-row.active{{
  background:var(--surface);box-shadow:0 2px 8px rgba(0,0,0,.07);font-weight:500;
}}
.tab-row.active::before{{
  content:"";position:absolute;left:-7px;top:9px;bottom:9px;width:3.5px;
  border-radius:99px;background:var(--accent-bar);
}}
.tab-select-form{{display:flex;align-items:center;flex:1;min-width:0}}
.tab-btn{{
  display:flex;align-items:center;gap:9px;width:100%;border:0;background:transparent;
  padding:0;text-align:left;cursor:pointer;min-width:0;
}}
.tab-fav{{
  width:22px;height:22px;min-width:22px;border-radius:7px;background:var(--card);
  display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:700;overflow:hidden;
}}
.tab-fav img{{width:16px;height:16px;object-fit:contain}}
.tab-title{{
  font-size:13.5px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;
}}
.tab-mini-btn{{
  border:0;background:transparent;color:var(--text-subtle);cursor:pointer;
  padding:2px 4px;border-radius:6px;font-size:13px;line-height:1;opacity:.65;
}}
.tab-mini-btn:hover{{opacity:1;background:var(--card);color:var(--text)}}
.new-tab-btn{{
  display:flex;align-items:center;gap:8px;border:0;background:transparent;
  color:var(--text-muted);font-size:13.5px;padding:8px 10px;border-radius:11px;
  cursor:pointer;width:100%;text-align:left;
}}
.new-tab-btn:hover{{background:var(--surface-hover);color:var(--text)}}
.rail-footer{{margin-top:auto;position:relative}}
.profile-pill{{
  display:flex;align-items:center;justify-content:space-between;
  background:var(--surface);border:1px solid var(--divider);
  border-radius:18px;padding:7px 11px;
}}
.avatar{{
  width:30px;height:30px;border-radius:999px;background:#d6d4f7;color:#4f46e5;
  display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:600;
}}
.menu-details{{position:relative;width:100%}}
.profile-trigger{{display:flex;align-items:center;gap:10px;width:100%;list-style:none;cursor:pointer}}
.profile-trigger::-webkit-details-marker{{display:none}}
.profile-copy{{flex:1;font-size:12px;font-weight:600;white-space:nowrap}}
.profile-chevron{{color:var(--text-muted);font-size:14px}}
.profile-trigger:focus-visible,.rail-pin-toggle:focus-visible{{outline:2px solid #3b82f6;outline-offset:3px}}
.menu-details summary{{
  list-style:none;cursor:pointer;padding:4px 7px;border-radius:8px;
  color:var(--text-muted);font-weight:700;letter-spacing:.1em;
}}
.menu-details summary::-webkit-details-marker{{display:none}}
.menu-details summary:hover{{background:var(--card);color:var(--text)}}
.menu-popover{{
  position:absolute;bottom:44px;left:0;width:240px;
  background:var(--surface);border:1px solid var(--card-border);
  border-radius:14px;padding:6px;box-shadow:var(--shadow);z-index:40;
  display:flex;flex-direction:column;gap:2px;
}}
.menu-popover a{{
  padding:8px 10px;border-radius:9px;font-size:13px;color:var(--text);display:flex;align-items:center;justify-content:space-between;
}}
.menu-popover a:hover{{background:var(--card)}}

/* Main Canvas */
.canvas{{
  flex:1;min-width:0;display:flex;flex-direction:column;
  padding:20px 36px 32px;position:relative;
}}
.topbar{{
  display:flex;align-items:center;justify-content:center;position:relative;
  min-height:44px;margin-bottom:24px;
}}
.mode-pill{{
  display:inline-flex;align-items:center;background:var(--pill-bg);
  border-radius:999px;padding:4px;gap:2px;
}}
.mode-pill a{{
  padding:8px 26px;border-radius:999px;font-size:14px;font-weight:500;
  color:var(--text-muted);transition:background .12s ease,color .12s ease;
}}
.mode-pill a.active{{
  background:var(--pill-active);color:var(--text);
  box-shadow:0 2px 8px rgba(0,0,0,.08);
}}
.privacy-btn{{
  position:absolute;right:0;top:50%;transform:translateY(-50%);
  border:0;background:transparent;color:var(--text);cursor:pointer;
  padding:8px;border-radius:999px;display:inline-flex;align-items:center;justify-content:center;
}}
.privacy-btn:hover{{background:var(--card)}}
.status-toast{{
  max-width:680px;margin:0 auto 16px;padding:10px 16px;border-radius:12px;
  background:var(--card);border:1px solid var(--card-border);font-size:13px;color:var(--text);
  text-align:center;
}}

/* Search Hero (Image 1 & Image 3) */
.search-stage{{
  flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;
  max-width:740px;width:100%;margin:0 auto;padding-bottom:40px;
}}
.hero-title{{
  font-size:34px;font-weight:450;letter-spacing:-.02em;margin:0 0 26px;text-align:center;
}}
.search-pill{{
  width:100%;max-width:620px;display:flex;align-items:center;gap:12px;
  background:var(--surface);border:1px solid var(--card-border);
  border-radius:999px;padding:8px 10px 8px 18px;box-shadow:var(--shadow);
}}
.search-pill input{{
  flex:1;border:0;outline:0;background:transparent;font-size:16px;color:var(--text);min-width:0;
}}
.search-pill input::placeholder{{color:var(--text-subtle)}}
.pill-icon-btn{{
  border:0;background:transparent;color:var(--text);width:34px;height:34px;
  border-radius:999px;display:inline-flex;align-items:center;justify-content:center;
  cursor:pointer;font-size:22px;line-height:1;
}}
.pill-icon-btn:hover{{background:var(--card)}}
.pill-submit{{
  width:42px;height:42px;min-width:42px;border-radius:999px;border:0;
  background:var(--btn-solid-bg);color:var(--btn-solid-fg);
  display:inline-flex;align-items:center;justify-content:center;cursor:pointer;
}}
.pill-submit:hover{{opacity:.9}}

/* Last Pages Visited (Image 1) */
.section-block{{width:100%;max-width:620px;margin-top:44px}}
.section-heading{{
  display:flex;align-items:center;gap:8px;font-size:14px;font-weight:500;
  color:var(--text-muted);margin-bottom:14px;
}}
.visited-grid{{
  display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px;
}}
.visited-card{{
  display:flex;flex-direction:column;justify-content:space-between;min-height:132px;
  background:var(--card);border:1px solid var(--divider);border-radius:18px;
  padding:14px 15px;box-shadow:0 3px 12px rgba(15,23,42,.04);
  transition:transform .12s ease,border-color .12s ease;
}}
.visited-card:hover{{transform:translateY(-2px);border-color:var(--text-muted)}}
.visited-top{{display:flex;align-items:center;justify-content:space-between;margin-bottom:10px}}
.visited-title{{font-size:14px;font-weight:500;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;margin-bottom:2px}}
.visited-host{{font-size:12px;color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
.visited-time{{font-size:11.5px;color:var(--text-subtle);margin-top:12px}}
.empty-note{{
  font-size:13px;color:var(--text-subtle);padding:14px 16px;border-radius:14px;
  border:1px dashed var(--divider);background:transparent;
}}

/* Agent Mode Composer + Cloud Tray (Image 2) */
.agent-stage{{
  width:100%;max-width:680px;margin:18px auto 0;display:flex;flex-direction:column;align-items:center;
}}
.agent-composer-wrap{{width:100%}}
.agent-card{{
  width:100%;background:var(--card);border:1.5px solid #4b5563;
  border-radius:22px;padding:18px 18px 14px;position:relative;z-index:2;
}}
.agent-card textarea{{
  width:100%;min-height:76px;border:0;outline:0;background:transparent;resize:vertical;
  font-size:16px;line-height:1.5;color:var(--text);
}}
.agent-card textarea::placeholder{{color:var(--text-subtle)}}
.agent-card-bar{{
  display:flex;align-items:center;justify-content:space-between;margin-top:8px;gap:10px;
}}
.bar-left,.bar-right{{display:flex;align-items:center;gap:10px}}
.circle-ghost{{
  width:34px;height:34px;border-radius:999px;border:1px solid var(--divider);
  background:transparent;color:var(--text);display:inline-flex;align-items:center;
  justify-content:center;cursor:pointer;font-size:18px;
}}
.circle-ghost:hover{{background:var(--surface)}}
.model-select{{
  border:0;background:transparent;font-size:14px;font-weight:500;color:var(--text);
  cursor:pointer;padding:6px 8px;border-radius:10px;outline:0;
}}
.model-select:hover{{background:var(--surface)}}
.cloud-tray{{
  width:92%;margin:-10px auto 0;padding:18px 18px 10px;
  background:var(--pill-bg);border-radius:0 0 18px 18px;
  display:flex;align-items:center;justify-content:space-between;
  border:1px solid var(--divider);border-top:0;
}}
.tray-icons{{display:flex;align-items:center;gap:16px;color:var(--text)}}
.cloud-toggle-btn{{
  display:inline-flex;align-items:center;gap:8px;border:1px solid transparent;
  background:transparent;color:var(--text);padding:6px 12px;border-radius:999px;
  font-size:13.5px;font-weight:500;cursor:pointer;transition:all .14s ease;
}}
.cloud-toggle-btn:hover{{background:var(--surface);border-color:var(--card-border)}}
.cloud-toggle-btn.enabled{{
  background:#111827;color:#ffffff;border-color:#111827;
}}

/* Previous Work List (Image 2) */
.work-list{{
  width:100%;background:var(--card);border:1px solid var(--card-border);
  border-radius:16px;overflow:hidden;
}}
.work-item{{
  display:flex;align-items:flex-start;justify-content:space-between;gap:16px;
  padding:14px 18px;border-bottom:1px solid var(--divider);transition:background .12s ease;
}}
.work-item:last-child{{border-bottom:0}}
.work-item:hover{{background:var(--surface-hover)}}
.work-item-title{{font-size:14.5px;font-weight:600;margin-bottom:3px;display:flex;align-items:center;gap:8px}}
.work-item-snippet{{font-size:13px;color:var(--text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:500px}}
.work-item-date{{font-size:13px;color:var(--text-muted);white-space:nowrap;padding-top:2px}}
.badge{{
  display:inline-flex;align-items:center;gap:4px;font-size:11px;font-weight:600;
  padding:2px 8px;border-radius:999px;background:var(--pill-bg);color:var(--text);
}}
.badge.cloud{{background:#dbeafe;color:#1d4ed8}}
.badge.green{{background:#dcfce7;color:#15803d}}

/* Active Work Thread View (Image 4) */
.thread-shell{{
  flex:1;display:flex;flex-direction:column;max-width:740px;width:100%;margin:0 auto;
  height:calc(100vh - 52px);
}}
.thread-header{{
  display:flex;align-items:center;justify-content:space-between;gap:12px;
  padding-bottom:14px;border-bottom:1px solid var(--divider);margin-bottom:18px;
}}
.thread-feed{{
  flex:1;overflow-y:auto;padding:4px 4px 24px;display:flex;flex-direction:column;gap:20px;
}}
.msg-user{{
  align-self:flex-end;max-width:85%;background:var(--card);border:1px solid var(--divider);
  border-radius:18px;padding:12px 16px;font-size:14.5px;line-height:1.55;
}}
.msg-assistant{{
  font-size:15px;line-height:1.68;color:var(--text);
}}
.msg-assistant h3{{font-size:16px;font-weight:700;margin:18px 0 8px}}
.msg-assistant ul{{margin:6px 0 12px;padding-left:22px}}
.msg-assistant li{{margin-bottom:6px}}
.kbd-chip{{
  display:inline-block;padding:1px 7px;border-radius:6px;background:var(--card);
  border:1px solid var(--card-border);font-family:ui-monospace,SFMono-Regular,Menlo,monospace;
  font-size:12.5px;
}}
.thread-scroll-btn{{
  align-self:center;margin-bottom:10px;width:30px;height:30px;border-radius:999px;
  border:1px solid var(--card-border);background:var(--surface);color:var(--text-muted);
  display:inline-flex;align-items:center;justify-content:center;font-size:14px;
}}
.thread-composer{{
  background:var(--card);border:1px solid var(--card-border);
  border-radius:20px;padding:14px 16px 12px;
}}
.thread-composer textarea{{
  width:100%;min-height:56px;border:0;outline:0;background:transparent;resize:vertical;
  font-size:14.5px;line-height:1.5;color:var(--text);
}}
.blue-send{{
  width:36px;height:36px;min-width:36px;border-radius:999px;border:0;
  background:var(--blue-btn);color:#ffffff;display:inline-flex;
  align-items:center;justify-content:center;cursor:pointer;
}}
.blue-send:hover{{filter:brightness(1.06)}}

/* Internal Management Pages (updates, history, settings, bookmarks, downloads) */
.internal-wrap{{max-width:820px;width:100%;margin:0 auto;display:flex;flex-direction:column;gap:20px}}
.internal-card{{
  background:var(--surface);border:1px solid var(--divider);border-radius:18px;
  padding:20px 22px;box-shadow:0 2px 12px rgba(15,23,42,.04);
}}
.internal-row{{
  display:flex;align-items:center;justify-content:space-between;gap:14px;
  padding:12px 0;border-bottom:1px solid var(--divider);
}}
.internal-row:last-child{{border-bottom:0}}
.btn-primary{{
  display:inline-flex;align-items:center;justify-content:center;gap:8px;
  background:var(--btn-solid-bg);color:var(--btn-solid-fg);border:0;
  border-radius:999px;padding:11px 26px;font-size:14.5px;font-weight:600;cursor:pointer;
}}
.btn-primary:hover{{opacity:.9}}
.btn-secondary{{
  display:inline-flex;align-items:center;justify-content:center;gap:6px;
  background:var(--card);color:var(--text);border:1px solid var(--card-border);
  border-radius:10px;padding:8px 14px;font-size:13px;font-weight:500;cursor:pointer;
}}
.btn-secondary:hover{{background:var(--surface-hover)}}
.highlight-grid{{
  display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:14px;margin-top:14px;
}}
.highlight-box{{
  background:var(--card);border:1px solid var(--divider);border-radius:14px;padding:14px 16px;
}}
.input-std{{
  padding:9px 12px;border-radius:10px;border:1px solid var(--card-border);
  background:var(--surface);color:var(--text);font-size:13.5px;outline:0;
}}
/* Settings navigation and layout */
.settings-layout{{display:grid;grid-template-columns:218px minmax(0,1fr);gap:22px;width:100%;max-width:1130px;margin:0 auto}}
.settings-menu{{display:flex;flex-direction:column;gap:6px;position:sticky;top:20px;align-self:start;padding:16px 12px;border-radius:18px;background:var(--sidebar);border:1px solid var(--divider)}}
.settings-menu a{{padding:11px 12px;border-radius:10px;font-size:13.5px;color:var(--text-muted)}}
.settings-menu a:hover,.settings-menu a:focus-visible{{background:var(--surface);color:var(--text);outline:0}}
.settings-menu strong{{padding:5px 12px 11px;font-size:14px}}
.settings-content{{display:flex;flex-direction:column;gap:18px;min-width:0}}
.settings-content .internal-card{{scroll-margin-top:20px}}
.settings-content h2{{font-size:17px;margin:0 0 12px}}
.settings-help{{font-size:12.5px;color:var(--text-muted);line-height:1.55}}
/* A CSS-only pinned/hovering rail: no script or new local server needed. */
.rail-pin-input{{position:absolute;width:1px;height:1px;opacity:0}}
.rail-pin-toggle{{position:fixed;left:17px;top:7px;z-index:80;padding:7px 10px;border-radius:9px;cursor:pointer;background:var(--sidebar);border:1px solid var(--divider);font-size:16px}}
.rail{{padding-top:46px;transition:width .19s ease,min-width .19s ease}}
.sidebar-auto-hide .rail{{width:56px;min-width:56px;padding-left:7px;padding-right:7px}}
.sidebar-auto-hide .rail > :not(.rail-footer){{opacity:0;pointer-events:none;white-space:nowrap}}
.sidebar-auto-hide .rail .profile-copy,.sidebar-auto-hide .rail .profile-chevron{{display:none}}
.sidebar-auto-hide .rail .profile-pill{{padding:5px;justify-content:center}}
.sidebar-auto-hide .rail:is(:hover,:focus-within),.sidebar-auto-hide .rail-pin-input:checked ~ .shell .rail{{width:214px;min-width:214px;padding-left:14px;padding-right:14px}}
.sidebar-auto-hide .rail:is(:hover,:focus-within) > :not(.rail-footer),.sidebar-auto-hide .rail-pin-input:checked ~ .shell .rail > :not(.rail-footer){{opacity:1;pointer-events:auto;white-space:normal}}
.sidebar-auto-hide .rail:is(:hover,:focus-within) .profile-copy,.sidebar-auto-hide .rail:is(:hover,:focus-within) .profile-chevron,.sidebar-auto-hide .rail-pin-input:checked ~ .shell .rail .profile-copy,.sidebar-auto-hide .rail-pin-input:checked ~ .shell .rail .profile-chevron{{display:inline}}
.sidebar-auto-hide .rail:is(:hover,:focus-within) .profile-pill,.sidebar-auto-hide .rail-pin-input:checked ~ .shell .rail .profile-pill{{padding:7px 11px;justify-content:space-between}}
@media(max-width:900px){{.settings-layout{{grid-template-columns:1fr}}.settings-menu{{position:static;flex-direction:row;flex-wrap:wrap}}}}
@media(max-width:840px){{
  .rail{{width:180px;min-width:180px}}
  .visited-grid{{grid-template-columns:1fr}}
  .canvas{{padding:16px}}
}}
"#
    )
}

fn render_sidebar_rail(ctx: &PageContext) -> String {
    let tok = hidden("token", &ctx.token);
    let mut out = String::from(r#"<aside class="rail" aria-label="Sidebar">"#);

    // 1. PINNED section — ONLY render favicon tiles if user has actually pinned websites
    out.push_str(r#"<div class="rail-label"><span>PINNED</span></div>"#);
    if !ctx.pinned_sites.is_empty() {
        out.push_str(r#"<div class="pinned-grid">"#);
        for pin in &ctx.pinned_sites {
            let letter = initial_letter(&pin.title, &pin.url);
            let fav = if !pin.favicon_url.is_empty() {
                format!(
                    r#"<img src="{}" alt="{}">"#,
                    esc(&pin.favicon_url),
                    esc(&letter)
                )
            } else {
                esc(&letter)
            };
            let _ = write!(
                out,
                r#"<div class="pin-tile" title="{}">
  <form method="post" action="/action" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center">
    {tok}<input type="hidden" name="value" value="{}">
    <button name="command" value="navigate" style="border:0;background:transparent;cursor:pointer;width:100%;height:100%;display:flex;align-items:center;justify-content:center" aria-label="Open {}">
      <span class="pin-icon">{fav}</span>
    </button>
  </form>
  <form method="post" action="/pin">
    {tok}<input type="hidden" name="command" value="unpin"><input type="hidden" name="value" value="{}">
    <button class="pin-unpin" title="Unpin">×</button>
  </form>
</div>"#,
                esc(&pin.title),
                esc(&pin.url),
                esc(&pin.title),
                esc(&pin.url),
            );
        }
        out.push_str("</div>");
    }

    // 2. Sidebar Search input (filters open tabs or navigates to URL)
    let _ = write!(
        out,
        r#"<form method="get" action="{}" class="rail-search">
  {}<input type="search" name="q" value="{}" placeholder="Search" aria-label="Search tabs">
</form>"#,
        if ctx.is_private {
            "/internal/private"
        } else {
            "/internal/search"
        },
        icon_search(15),
        esc(&ctx.tab_filter),
    );

    out.push_str(r#"<div class="rail-divider"></div>"#);

    // 3. TABS section — ONLY real open tabs appear here!
    let filter_lc = ctx.tab_filter.trim().to_ascii_lowercase();
    let visible_tabs: Vec<&Tab> = ctx
        .tabs
        .iter()
        .filter(|t| {
            filter_lc.is_empty()
                || t.title.to_ascii_lowercase().contains(&filter_lc)
                || t.url.to_ascii_lowercase().contains(&filter_lc)
        })
        .collect();

    let _ = write!(
        out,
        r#"<div class="rail-label"><span>TABS</span><span>{}</span></div><div class="tabs-list">"#,
        ctx.tabs.len()
    );

    for tab in visible_tabs {
        let is_active = ctx.active_tab_id == Some(tab.id);
        let active_cls = if is_active { " active" } else { "" };
        let letter = initial_letter(&tab.title, &tab.url);
        let fav_url = favicon_url_for(&tab.url);
        let fav_html = if !fav_url.is_empty() {
            format!(r#"<img src="{}" alt="{}">"#, esc(&fav_url), esc(&letter))
        } else {
            esc(&letter)
        };
        let label = if tab.title.trim().is_empty() {
            "New tab"
        } else {
            tab.title.as_str()
        };
        let pin_btn = if crate::session::valid_page_url(&tab.url) {
            format!(
                r#"<form method="post" action="/pin">{tok}<input type="hidden" name="command" value="pin"><input type="hidden" name="value" value="{}"><input type="hidden" name="skill_name" value="{}"><button class="tab-mini-btn" title="Pin website to sidebar">📌</button></form>"#,
                esc(&tab.url),
                esc(label)
            )
        } else {
            String::new()
        };
        let _ = write!(
            out,
            r#"<div class="tab-row{active_cls}">
  <form method="post" action="/action" class="tab-select-form">
    {tok}<input type="hidden" name="id" value="{}">
    <button class="tab-btn" name="command" value="select" title="{}">
      <span class="tab-fav">{fav_html}</span>
      <span class="tab-title">{}</span>
    </button>
  </form>
  {pin_btn}
  <form method="post" action="/action">
    {tok}<input type="hidden" name="id" value="{}">
    <button class="tab-mini-btn" name="command" value="close" aria-label="Close tab" title="Close tab">×</button>
  </form>
</div>"#,
            tab.id,
            esc(&tab.url),
            esc(label),
            tab.id,
        );
    }

    // + New tab button
    let _ = write!(
        out,
        r#"<form method="post" action="/action" style="margin-top:4px">
  {tok}<button class="new-tab-btn" name="command" value="new">+ New tab</button>
</form></div>"#
    );

    // 4. Bottom user avatar + "..." menu linking to internal charlotte:// pages
    let _ = write!(
        out,
        r#"<div class="rail-footer">
  <div class="profile-pill">
    <details class="menu-details">
      <summary class="profile-trigger" aria-label="Open Charlotte profile menu" title="Charlotte profile">
        <span class="avatar">{}</span><span class="profile-copy">Charlotte Profile</span><span class="profile-chevron">⌄</span>
      </summary>
      <div class="menu-popover">
        <a href="/internal/settings#personalization"><span>Personalization</span><span>⚙</span></a>
        <a href="/internal/settings#passwords-security"><span>Passwords &amp; Security</span><span>🛡</span></a>
        <a href="/internal/settings#ai-agent"><span>AI &amp; Agent</span><span>✦</span></a>
        <a href="/internal/extensions"><span>Extensions Marketplace</span><span>＋</span></a>
        <a href="/internal/history"><span>History</span><span>↺</span></a>
        <a href="/internal/bookmarks"><span>Bookmarks</span><span>☆</span></a>
        <a href="/internal/downloads"><span>Downloads</span><span>↓</span></a>
        <a href="/internal/updates"><span>Updates</span><span>v{}</span></a>
        <a href="/internal/about"><span>About Charlotte</span><span>ⓘ</span></a>
      </div>
    </details>
  </div>
</div></aside>"#,
        esc(&ctx.user_initials),
        env!("CARGO_PKG_VERSION"),
    );

    out
}

fn wrap_shell(title: &str, ctx: &PageContext, main_content: &str) -> String {
    let css = shared_css(ctx.is_private);
    let rail = render_sidebar_rail(ctx);
    let rail_mode = if ctx.sidebar_auto_hide { "sidebar-auto-hide" } else { "sidebar-pinned" };
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src https: http: data:; form-action 'self' https://www.google.com; base-uri 'none'; object-src 'none'">
<title>{}</title><style>{css}</style></head><body class="{rail_mode}"><input type="checkbox" id="rail-pin" class="rail-pin-input" aria-label="Pin sidebar open"><label for="rail-pin" class="rail-pin-toggle" title="Toggle sidebar pin">☰</label><div class="shell">{rail}<main class="canvas">{main_content}</main></div></body></html>"#,
        esc(title)
    )
}

fn render_topbar(ctx: &PageContext, active_mode: &str, show_mode_toggle: bool) -> String {
    let tok = hidden("token", &ctx.token);
    let mut out = String::from(r#"<header class="topbar">"#);
    if show_mode_toggle {
        let search_cls = if active_mode == "search" { " active" } else { "" };
        let agent_cls = if active_mode == "agent" { " active" } else { "" };
        let _ = write!(
            out,
            r#"<nav class="mode-pill" aria-label="Workspace mode">
  <a href="/internal/search" class="{search_cls}">Search</a>
  <a href="/internal/agent" class="{agent_cls}">Agent</a>
</nav>"#
        );
    }
    if ctx.is_private {
        let _ = write!(
            out,
            r#"<a href="/internal/search" class="privacy-btn" title="Exit Private Search">{}</a>"#,
            icon_incognito()
        );
    } else {
        let _ = write!(
            out,
            r#"<form method="post" action="/privacy/open" style="position:absolute;right:0;top:50%;transform:translateY(-50%)">
  {tok}<button class="privacy-btn" style="position:static;transform:none" title="Open Private / Incognito window (Black Theme)" aria-label="Enable Private browsing">{}</button>
</form>"#,
            icon_incognito()
        );
    }
    out.push_str("</header>");
    if let Some(banner) = &ctx.status_banner {
        let _ = write!(out, r#"<div class="status-toast">{}</div>"#, esc(banner));
    }
    out
}

/// Image 1: Search Mode (`charlotte://search` / `charlotte://newtab`)
pub fn render_search_mode(ctx: &PageContext, recent_visits: &[Visit]) -> String {
    let tok = hidden("token", &ctx.token);
    let mut body = render_topbar(ctx, "search", true);
    let _ = write!(
        body,
        r#"<div class="search-stage">
  <form method="post" action="/action" class="search-pill">
    {tok}
    <button type="submit" name="command" value="new" class="pill-icon-btn" title="New tab" aria-label="New tab">+</button>
    <input type="text" name="value" placeholder="Search web" aria-label="Search web or enter URL" autofocus autocomplete="off">
    <span class="pill-icon-btn" title="Voice input" aria-hidden="true">{}</span>
    <button type="submit" name="command" value="navigate" class="pill-submit" aria-label="Search">{}</button>
  </form>
  <section class="section-block" aria-label="Last pages visited">
    <div class="section-heading">{}<span>Last pages visited</span></div>"#,
        icon_mic(),
        icon_search(18),
        icon_clock(),
    );

    if recent_visits.is_empty() {
        body.push_str(
            r#"<div class="empty-note">Pages you visit will appear here automatically.</div>"#,
        );
    } else {
        body.push_str(r#"<div class="visited-grid">"#);
        for visit in recent_visits.iter().take(6) {
            let host = domain_from_url(&visit.url);
            let title = if visit.title.trim().is_empty() {
                host.clone()
            } else {
                visit.title.clone()
            };
            let letter = initial_letter(&title, &visit.url);
            let fav_url = favicon_url_for(&visit.url);
            let fav_html = if !fav_url.is_empty() {
                format!(r#"<img src="{}" alt="{}">"#, esc(&fav_url), esc(&letter))
            } else {
                esc(&letter)
            };
            let ago = format_relative_time(visit.visited_at);
            let _ = write!(
                body,
                r#"<form method="post" action="/action" style="margin:0;display:flex">
  {tok}<input type="hidden" name="value" value="{}">
  <button type="submit" name="command" value="navigate" class="visited-card" style="width:100%;text-align:left;cursor:pointer">
    <div>
      <div class="visited-top">
        <span class="pin-icon">{fav_html}</span>
        <span style="color:var(--text-muted);font-size:15px">↗</span>
      </div>
      <div class="visited-title">{}</div>
      <div class="visited-host">{}</div>
    </div>
    <div class="visited-time">{}</div>
  </button>
</form>"#,
                esc(&visit.url),
                esc(&title),
                esc(&host),
                esc(&ago),
            );
        }
        body.push_str("</div>");
    }

    body.push_str("</section></div>");
    wrap_shell("Charlotte — Search", ctx, &body)
}

/// Image 2: Agent Mode (`charlotte://agent`)
pub fn render_agent_mode(ctx: &PageContext, sessions: &[AgentWorkSession]) -> String {
    let tok = hidden("token", &ctx.token);
    let mut body = render_topbar(ctx, "agent", true);
    let cloud_cls = if ctx.cloud_mode_enabled {
        " enabled"
    } else {
        ""
    };
    let cloud_label = if ctx.cloud_mode_enabled {
        "Work it on Cloud · ON (E2B)"
    } else {
        "Work it on Cloud"
    };
    let models = [
        ("charlotte-ai", "Charlotte-AI"),
        ("gemini", "Gemini"),
        ("claude", "Claude"),
        ("openai", "OpenAI"),
        ("deepseek", "DeepSeek"),
        ("ollama", "Local Ollama"),
    ];
    let mut model_opts = String::new();
    for (val, label) in models {
        let sel = if ctx.selected_model == val {
            " selected"
        } else {
            ""
        };
        let _ = write!(
            model_opts,
            r#"<option value="{val}"{sel}>{label}</option>"#
        );
    }

    let _ = write!(
        body,
        r#"<div class="agent-stage">
  <h1 class="hero-title">What should we work on?</h1>
  <div class="agent-composer-wrap">
    <form method="post" action="/agent/work" class="agent-card">
      {tok}
      <input type="hidden" name="mode" value="{}">
      <textarea name="prompt_text" placeholder="Talk to Charlotte" aria-label="Talk to Charlotte" maxlength="8000" required></textarea>
      <div class="agent-card-bar">
        <div class="bar-left">
          <a href="/internal/agent" class="circle-ghost" title="New task">+</a>
          <a href="/internal/settings" class="circle-ghost" title="Connectors &amp; context">@</a>
        </div>
        <div class="bar-right">
          <select name="provider" class="model-select" aria-label="Select AI model">{model_opts}</select>
          <button type="submit" class="pill-submit" title="Start Charlotte Agent" aria-label="Send prompt">{}</button>
        </div>
      </div>
    </form>
    <div class="cloud-tray">
      <div class="tray-icons">
        <a href="/internal/bookmarks" title="Workspace folders">{}</a>
        <a href="/internal/downloads" title="Files &amp; artifacts">{}</a>
      </div>
      <form method="post" action="/agent/cloud-toggle" style="margin:0">
        {tok}
        <button type="submit" class="cloud-toggle-btn{cloud_cls}" aria-pressed="{}" title="Toggle E2B Cloud sandbox execution">
          {}<span>{cloud_label}</span>
        </button>
      </form>
    </div>
  </div>
  <section class="section-block" style="max-width:100%;margin-top:34px" aria-label="Previous work">
    <div class="section-heading">{}<span>Previous work</span></div>"#,
        if ctx.cloud_mode_enabled {
            "cloud_e2b"
        } else {
            "local"
        },
        icon_send_plane(),
        icon_folder(),
        icon_file(),
        if ctx.cloud_mode_enabled { "true" } else { "false" },
        icon_cloud(),
        icon_clock(),
    );

    if sessions.is_empty() {
        body.push_str(
            r#"<div class="empty-note">Your agent tasks and cloud runs will appear here once you start working with Charlotte.</div>"#,
        );
    } else {
        body.push_str(r#"<div class="work-list">"#);
        for s in sessions.iter().take(25) {
            let date_str = format_short_date(s.updated_at);
            let mode_badge = if s.mode == "cloud_e2b" {
                r#"<span class="badge cloud">☁ E2B Cloud</span>"#
            } else {
                ""
            };
            let snippet = s
                .messages
                .last()
                .map(|m| m.content.as_str())
                .unwrap_or(&s.prompt)
                .lines()
                .next()
                .unwrap_or(&s.prompt)
                .chars()
                .take(110)
                .collect::<String>();
            let _ = write!(
                body,
                r#"<a href="/internal/agent/work/{}" class="work-item">
  <div style="min-width:0">
    <div class="work-item-title"><span>{}</span>{mode_badge}</div>
    <div class="work-item-snippet">{}</div>
  </div>
  <div class="work-item-date">{}</div>
</a>"#,
                esc(&s.id),
                esc(&s.title),
                esc(&snippet),
                esc(&date_str),
            );
        }
        body.push_str("</div>");
    }

    body.push_str("</section></div>");
    wrap_shell("Charlotte — Agent", ctx, &body)
}

/// Image 3: Black-Themed Private / Incognito Search (`charlotte://private`)
pub fn render_private_mode(ctx: &PageContext) -> String {
    let mut private_ctx = ctx.clone();
    private_ctx.is_private = true;
    let tok = hidden("token", &private_ctx.token);
    let mut body = render_topbar(&private_ctx, "private", false);
    let _ = write!(
        body,
        r#"<div class="search-stage">
  <h1 class="hero-title" style="color:#ffffff">Private search</h1>
  <form method="post" action="/action" class="search-pill">
    {tok}
    <button type="submit" name="command" value="new" class="pill-icon-btn" title="New private tab" aria-label="New tab">+</button>
    <input type="text" name="value" placeholder="Search web" aria-label="Private search web" autofocus autocomplete="off">
    <span class="pill-icon-btn" title="Voice input" aria-hidden="true">{}</span>
    <button type="submit" name="command" value="navigate" class="pill-submit" aria-label="Search">{}</button>
  </form>
</div>"#,
        icon_mic(),
        icon_search(18),
    );
    wrap_shell("Charlotte — Private search", &private_ctx, &body)
}

/// Renders structured markdown-like assistant text safely (headings, bullets, inline `code` chips).
fn format_agent_markdown(raw: &str) -> String {
    let mut out = String::new();
    let mut in_list = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            continue;
        }
        if let Some(bullet) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            if !in_list {
                out.push_str("<ul>");
                in_list = true;
            }
            let _ = write!(out, "<li>{}</li>", format_inline_code(bullet));
        } else if trimmed.starts_with("### ") || trimmed.starts_with("## ") || trimmed.starts_with("# ") {
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            let heading = trimmed.trim_start_matches('#').trim();
            let _ = write!(out, "<h3>{}</h3>", format_inline_code(heading));
        } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
            && trimmed.contains(". ")
            && trimmed.len() < 80
        {
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            let _ = write!(out, "<h3>{}</h3>", format_inline_code(trimmed));
        } else {
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            let _ = write!(out, "<p style=\"margin:6px 0\">{}</p>", format_inline_code(trimmed));
        }
    }
    if in_list {
        out.push_str("</ul>");
    }
    out
}

fn format_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut parts = line.split('`');
    let mut is_code = false;
    while let Some(seg) = parts.next() {
        if is_code {
            let _ = write!(out, r#"<code class="kbd-chip">{}</code>"#, esc(seg));
        } else {
            // Support **bold** segments safely
            let mut bold_parts = seg.split("**");
            let mut is_bold = false;
            while let Some(bseg) = bold_parts.next() {
                if is_bold {
                    let _ = write!(out, "<strong>{}</strong>", esc(bseg));
                } else {
                    out.push_str(&esc(bseg));
                }
                is_bold = !is_bold;
            }
        }
        is_code = !is_code;
    }
    out
}

/// Image 4: Agent Active Work Thread (`charlotte://agent/work/<id>`)
pub fn render_agent_work_view(ctx: &PageContext, session: &AgentWorkSession) -> String {
    let tok = hidden("token", &ctx.token);
    let env_badge = if session.mode == "cloud_e2b" {
        let sb = if session.cloud_sandbox_id.is_empty() {
            "provisioning".into()
        } else {
            session.cloud_sandbox_id.clone()
        };
        format!(
            r#"<span class="badge cloud">☁ E2B Cloud · {}</span>"#,
            esc(&sb)
        )
    } else {
        r#"<span class="badge">Local Agent</span>"#.to_owned()
    };

    let mut body = String::new();
    let _ = write!(
        body,
        r#"<div class="thread-shell">
  <div class="thread-header">
    <div style="display:flex;align-items:center;gap:10px;min-width:0">
      <a href="/internal/agent" class="btn-secondary" style="padding:6px 11px" title="Back to Agent">← Back</a>
      <strong style="font-size:15.5px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{}</strong>
    </div>
    <div style="display:flex;align-items:center;gap:8px">{env_badge}<span class="badge green">{}</span></div>
  </div>
  <div class="thread-feed" id="feed">"#,
        esc(&session.title),
        esc(&session.status),
    );

    for msg in &session.messages {
        if msg.role == "user" {
            let _ = write!(
                body,
                r#"<div class="msg-user">{}</div>"#,
                format_inline_code(&msg.content)
            );
        } else {
            let _ = write!(
                body,
                r#"<div class="msg-assistant">{}</div>"#,
                format_agent_markdown(&msg.content)
            );
        }
    }

    let _ = write!(
        body,
        r##"<div id="bottom"></div></div>
  <a href="#bottom" class="thread-scroll-btn" title="Scroll to latest" aria-label="Scroll to bottom">↓</a>
  <form method="post" action="/agent/work/{}/message" class="thread-composer">
    {tok}
    <textarea name="prompt_text" placeholder="Reply to Charlotte or guide the next step..." maxlength="8000" required></textarea>
    <div class="agent-card-bar">
      <div class="bar-left">
        <a href="/internal/agent" class="pill-icon-btn" title="Start new work">+</a>
      </div>
      <div class="bar-right">
        <select name="provider" class="model-select" aria-label="Model">
          <option value="charlotte-ai">Charlotte-AI</option>
          <option value="gemini">Gemini</option>
          <option value="claude">Claude</option>
          <option value="openai">OpenAI</option>
          <option value="deepseek">DeepSeek</option>
        </select>
        <span class="pill-icon-btn" aria-hidden="true">{}</span>
        <button type="submit" class="blue-send" title="Send follow-up" aria-label="Send">{}</button>
      </div>
    </div>
  </form>
</div>"##,
        esc(&session.id),
        icon_mic(),
        icon_arrow_up(),
    );

    wrap_shell(&format!("Charlotte — {}", session.title), ctx, &body)
}

/// `charlotte://updates` (also aliased from `charllote://updates`)
pub fn render_updates_page(
    ctx: &PageContext,
    updates: &[BrowserUpdate],
    loaded_from_server: bool,
    source_note: &str,
    pending_version: Option<&str>,
) -> String {
    let tok = hidden("token", &ctx.token);
    let latest_ver = pending_version
        .or_else(|| updates.first().map(|u| u.version.as_str()))
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    let sync_badge = if loaded_from_server {
        r#"<span class="badge green">Realtime Server</span>"#
    } else {
        r#"<span class="badge">Server Unreachable</span>"#
    };

    let mut body = render_topbar(ctx, "search", true);
    let _ = write!(
        body,
        r#"<div class="internal-wrap">
  <div class="internal-card" style="display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:16px">
    <div>
      <div style="display:flex;align-items:center;gap:10px;margin-bottom:6px">
        <span class="badge cloud">charlotte://updates</span>
        {sync_badge}
      </div>
      <h1 style="margin:0 0 6px;font-size:26px;letter-spacing:-.02em">Charlotte Updates (v{})</h1>
      <p style="margin:0;color:var(--text-muted);font-size:14px">{}</p>
    </div>
    <div style="display:flex;align-items:center;gap:10px">
      <form method="post" action="/updates/check" style="margin:0">{tok}<button class="btn-secondary">Check Now</button></form>
      <form method="post" action="/updates/continue" style="margin:0">
        {tok}<input type="hidden" name="value" value="{}">
        <button type="submit" class="btn-primary">Continue →</button>
      </form>
    </div>
  </div>"#,
        esc(latest_ver),
        esc(source_note),
        esc(latest_ver),
    );

    if updates.is_empty() {
        let empty_msg = if loaded_from_server {
            "No updates have been published to the server yet. New releases saved in Supabase will appear here automatically in realtime."
        } else {
            source_note
        };
        let _ = write!(
            body,
            r#"<div class="internal-card"><div class="empty-note">{}</div></div>"#,
            esc(empty_msg)
        );
    }

    for rel in updates {
        let mut highlights_html = String::new();
        if !rel.highlights.is_empty() {
            highlights_html.push_str(r#"<div class="highlight-grid">"#);
            for h in &rel.highlights {
                let _ = write!(
                    highlights_html,
                    r#"<div class="highlight-box">
  <span class="badge" style="margin-bottom:8px">{}</span>
  <div style="font-weight:600;font-size:14.5px;margin-bottom:4px">{}</div>
  <div style="font-size:13px;color:var(--text-muted);line-height:1.5">{}</div>
</div>"#,
                    esc(&h.category),
                    esc(&h.title),
                    esc(&h.description),
                );
            }
            highlights_html.push_str("</div>");
        }
        let notes_html = format_agent_markdown(&rel.release_notes);
        let _ = write!(
            body,
            r#"<div class="internal-card">
  <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:8px">
    <h2 style="margin:0;font-size:18px">{}</h2>
    <span class="badge">v{} · {}</span>
  </div>
  <p style="margin:0 0 12px;color:var(--text-muted);font-size:14px">{}</p>
  {highlights_html}
  <div class="msg-assistant" style="margin-top:14px;font-size:14px">{notes_html}</div>
</div>"#,
            esc(&rel.title),
            esc(&rel.version),
            esc(&rel.channel),
            esc(&rel.summary),
        );
    }

    let _ = write!(
        body,
        r#"<div style="display:flex;justify-content:flex-end">
  <form method="post" action="/updates/continue">
    {tok}<input type="hidden" name="value" value="{}">
    <button type="submit" class="btn-primary">Continue to Charlotte →</button>
  </form>
</div></div>"#,
        esc(latest_ver)
    );

    wrap_shell("Charlotte Updates — charlotte://updates", ctx, &body)
}

/// `charlotte://history`
pub fn render_history_page(ctx: &PageContext, visits: &[Visit], query: &str) -> String {
    let tok = hidden("token", &ctx.token);
    let mut body = render_topbar(ctx, "search", true);
    let _ = write!(
        body,
        r#"<div class="internal-wrap">
  <div class="internal-card" style="display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:12px">
    <div>
      <span class="badge" style="margin-bottom:6px">charlotte://history</span>
      <h1 style="margin:0;font-size:24px">Browsing History</h1>
    </div>
    <div style="display:flex;align-items:center;gap:10px">
      <form method="get" action="/internal/history" style="display:flex;gap:6px">
        <input class="input-std" type="search" name="q" value="{}" placeholder="Search history...">
        <button class="btn-secondary">Filter</button>
      </form>
      <form method="post" action="/history/clear">{tok}<button class="btn-secondary">Clear History</button></form>
    </div>
  </div>
  <div class="internal-card">"#,
        esc(query),
    );

    if visits.is_empty() {
        body.push_str(r#"<div class="empty-note">No visited pages recorded yet.</div>"#);
    } else {
        for v in visits {
            let host = domain_from_url(&v.url);
            let title = if v.title.trim().is_empty() {
                host.as_str()
            } else {
                v.title.as_str()
            };
            let _ = write!(
                body,
                r#"<div class="internal-row">
  <div style="min-width:0;flex:1">
    <div style="font-weight:600;font-size:14px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{}</div>
    <div style="font-size:12.5px;color:var(--text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{}</div>
  </div>
  <span style="font-size:12px;color:var(--text-subtle);white-space:nowrap">{}</span>
  <form method="post" action="/action" style="margin:0">{tok}<input type="hidden" name="value" value="{}"><button name="command" value="navigate" class="btn-secondary">Open</button></form>
  <form method="post" action="/pin" style="margin:0">{tok}<input type="hidden" name="command" value="pin"><input type="hidden" name="value" value="{}"><input type="hidden" name="skill_name" value="{}"><button class="btn-secondary">Pin</button></form>
  <form method="post" action="/history/delete" style="margin:0">{tok}<input type="hidden" name="id" value="{}"><button class="btn-secondary">×</button></form>
</div>"#,
                esc(title),
                esc(&v.url),
                esc(&format_relative_time(v.visited_at)),
                esc(&v.url),
                esc(&v.url),
                esc(title),
                v.id,
            );
        }
    }
    body.push_str("</div></div>");
    wrap_shell("History — charlotte://history", ctx, &body)
}

/// `charlotte://settings`
pub fn render_settings_page(
    ctx: &PageContext,
    byok_keys: &[MaskedProviderKey],
    layout: TabLayout,
    dark_mode_pref: &str,
    update_status: &str,
    server_url: &str,
) -> String {
    let tok = hidden("token", &ctx.token);
    let mut body = render_topbar(ctx, "search", true);
    let sidebar_next = if ctx.sidebar_auto_hide { "off" } else { "on" };
    let sidebar_label = if ctx.sidebar_auto_hide { "Enabled" } else { "Disabled" };
    let shields_next = if ctx.shields_enabled {
        "shields-off"
    } else {
        "shields-on"
    };
    let shields_label = if ctx.shields_enabled {
        "Enabled — Click to Disable"
    } else {
        "Disabled — Click to Enable"
    };
    let layout_label = match layout {
        TabLayout::Horizontal => "Horizontal",
        TabLayout::Vertical => "Vertical",
    };

    let _ = write!(
        body,
        r#"<div class="settings-layout">
  <nav class="settings-menu" aria-label="Settings categories">
    <strong>⚙ Settings</strong>
    <a href='#personalization'>Personalization</a>
    <a href='#passwords-security'>Passwords &amp; Security</a>
    <a href='#ai-agent'>AI &amp; Agent</a>
    <a href='#extensions-marketplace'>Extensions Marketplace</a>
  </nav>
  <div class="settings-content">
  <div class="internal-card">
    <span class="badge" style="margin-bottom:6px">charlotte://settings</span>
    <h1 style="margin:0 0 6px;font-size:24px">Charlotte Settings</h1>
    <p style="margin:0;color:var(--text-muted);font-size:13.5px">Manage privacy, updates, appearance, cloud endpoint, and encrypted BYOK keys.</p>
  </div>

  <div class="internal-card" id='personalization'>
    <h2 style="margin:0 0 10px;font-size:16px">Personalization</h2>
    <div class="internal-row">
      <div><strong>Charlotte Shields (Ad &amp; Tracker Blocker)</strong><div style="font-size:12.5px;color:var(--text-muted)">{} blocked requests this session</div></div>
      <form method="post" action="/action">{tok}<button name="command" value="{shields_next}" class="btn-secondary">{shields_label}</button></form>
    </div>
    <div class="internal-row">
      <div><strong>Tab Layout</strong><div style="font-size:12.5px;color:var(--text-muted)">Current layout: {layout_label}</div></div>
      <form method="post" action="/settings/update" style="display:flex;gap:8px">{tok}
        <select name="mode" class="input-std"><option value="vertical">Vertical Sidebar</option><option value="horizontal">Horizontal Strip</option></select>
        <button name="command" value="layout" class="btn-secondary">Save</button>
      </form>
    </div>
    <div class="internal-row">
      <div><strong>Sidebar Auto-Hide</strong><div class="settings-help">Collapse the sidebar when you are not using it. Hover to expand or use the ☰ pin.</div></div>
      <form method="post" action="/settings/update">{tok}<input type="hidden" name="mode" value="{sidebar_next}"><button name="command" value="sidebar_auto_hide" class="btn-secondary">{sidebar_label} · Toggle</button></form>
    </div>
    <div class="internal-row">
      <div><strong>Forced Dark Mode for Web Pages</strong><div style="font-size:12.5px;color:var(--text-muted)">Current: {}</div></div>
      <form method="post" action="/settings/update" style="display:flex;gap:8px">{tok}
        <select name="mode" class="input-std"><option value="off">Off</option><option value="system">System</option><option value="always">Always</option></select>
        <button name="command" value="dark_mode" class="btn-secondary">Save</button>
      </form>
    </div>
    <div class="internal-row">
      <div><strong>Automatic Browser Updates</strong><div style="font-size:12.5px;color:var(--text-muted)">{} · Server: {}</div></div>
      <div style="display:flex;gap:8px">
        <a href="/internal/updates" class="btn-secondary">View Updates</a>
        <form method="post" action="/updates/check">{tok}<button class="btn-secondary">Check Now</button></form>
      </div>
    </div>
  </div>

  <div class="internal-card" id='ai-agent'>
    <h2 style="margin:0 0 8px;font-size:16px">AI &amp; Agent</h2>
    <p class="settings-help">Manage your AI providers and open Charlotte's agent workspace.</p>
    <p><a href="/internal/agent" class="btn-secondary">Open AI Agent</a></p>
    <h3 style="font-size:14px;margin-top:18px">BYOK API Keys (AES-256-GCM Encrypted)</h3>
    <p style="margin:0 0 12px;font-size:13px;color:var(--text-muted)">Keys are encrypted with AES-256-GCM and masked in the UI. Raw keys are never displayed after saving.</p>"#,
        ctx.blocked_requests,
        esc(dark_mode_pref),
        esc(update_status),
        esc(server_url),
    );

    if !byok_keys.is_empty() {
        for k in byok_keys {
            let _ = write!(
                body,
                r#"<div class="internal-row">
  <div><strong>{}</strong> · <code>{}</code> <span style="font-size:11.5px;color:var(--text-subtle)">({})</span></div>
  <form method="post" action="/settings/byok">{tok}<input type="hidden" name="provider" value="{}"><button name="command" value="delete" class="btn-secondary">Remove</button></form>
</div>"#,
                esc(&k.provider),
                esc(&k.masked_key),
                esc(&k.fingerprint),
                esc(&k.provider),
            );
        }
    }

    let _ = write!(
        body,
        r#"<form method="post" action="/settings/byok" style="display:flex;flex-wrap:wrap;gap:8px;margin-top:12px">
      {tok}
      <select name="provider" class="input-std">
        <option value="gemini">Google Gemini</option>
        <option value="openai">OpenAI</option>
        <option value="anthropic">Anthropic Claude</option>
        <option value="deepseek">DeepSeek</option>
        <option value="openrouter">OpenRouter</option>
        <option value="nvidia">Nvidia NIM</option>
      </select>
      <input type="password" name="api_key" class="input-std" style="flex:1;min-width:200px" placeholder="Enter API key" autocomplete="new-password">
      <button name="command" value="save" class="btn-primary" style="padding:8px 18px;font-size:13px">Save Key</button>
    </form>
  </div>

  <div class="internal-card" id='passwords-security'>
    <h2>Passwords &amp; Security</h2>
    <p class="settings-help">Charlotte keeps AI provider credentials in encrypted local storage. Saved website passwords are not yet managed by Charlotte; do not enter website passwords into an API key field.</p>
    <div class="internal-row"><div><strong>Tracker Protection</strong><div class="settings-help">Charlotte Shields is available from Personalization above.</div></div><span class="badge">Local</span></div>
    <div class="internal-row"><div><strong>Private Browsing</strong><div class="settings-help">Open a private browsing session.</div></div><form method="post" action="/privacy/open">{tok}<button class="btn-secondary">Open Private</button></form></div>
  </div>
  <div class="internal-card" id='extensions-marketplace'>
    <h2>Extensions Marketplace</h2>
    <p class="settings-help">Explore built-in Charlotte capabilities and extension support. Installing third-party extensions is not yet enabled in this build.</p>
    <a class="btn-primary" href="/internal/extensions">Browse Extensions</a>
  </div>
  </div>
</div>"#
    );

    wrap_shell("Settings — charlotte://settings", ctx, &body)
}

/// `charlotte://bookmarks`
pub fn render_bookmarks_page(ctx: &PageContext, bookmarks: &[Bookmark]) -> String {
    let tok = hidden("token", &ctx.token);
    let mut body = render_topbar(ctx, "search", true);
    body.push_str(
        r#"<div class="internal-wrap">
  <div class="internal-card">
    <span class="badge" style="margin-bottom:6px">charlotte://bookmarks</span>
    <h1 style="margin:0;font-size:24px">Bookmarks</h1>
  </div>
  <div class="internal-card">"#,
    );
    if bookmarks.is_empty() {
        body.push_str(r#"<div class="empty-note">No saved bookmarks yet. Press Ctrl+D on any page to save it.</div>"#);
    } else {
        for b in bookmarks {
            let _ = write!(
                body,
                r#"<div class="internal-row">
  <div style="min-width:0;flex:1">
    <div style="font-weight:600;font-size:14px">{}</div>
    <div style="font-size:12.5px;color:var(--text-muted)">{}</div>
  </div>
  <form method="post" action="/action" style="margin:0">{tok}<input type="hidden" name="value" value="{}"><button name="command" value="navigate" class="btn-secondary">Open</button></form>
  <form method="post" action="/pin" style="margin:0">{tok}<input type="hidden" name="command" value="pin"><input type="hidden" name="value" value="{}"><input type="hidden" name="skill_name" value="{}"><button class="btn-secondary">Pin</button></form>
</div>"#,
                esc(&b.title),
                esc(&b.url),
                esc(&b.url),
                esc(&b.url),
                esc(&b.title),
            );
        }
    }
    body.push_str("</div></div>");
    wrap_shell("Bookmarks — charlotte://bookmarks", ctx, &body)
}

/// `charlotte://downloads`
pub fn render_downloads_page(
    ctx: &PageContext,
    downloads: &[(i64, String, String, i64, i64, String, i64)],
) -> String {
    let mut body = render_topbar(ctx, "search", true);
    body.push_str(
        r#"<div class="internal-wrap">
  <div class="internal-card">
    <span class="badge" style="margin-bottom:6px">charlotte://downloads</span>
    <h1 style="margin:0;font-size:24px">Downloads</h1>
  </div>
  <div class="internal-card">"#,
    );
    if downloads.is_empty() {
        body.push_str(r#"<div class="empty-note">No downloads recorded yet.</div>"#);
    } else {
        for (_, url, save_path, recv, total, status, ts) in downloads {
            let _ = write!(
                body,
                r#"<div class="internal-row">
  <div style="min-width:0;flex:1">
    <div style="font-weight:600;font-size:14px">{}</div>
    <div style="font-size:12.5px;color:var(--text-muted)">{} · {} / {} bytes</div>
  </div>
  <span class="badge">{}</span>
  <span style="font-size:12px;color:var(--text-subtle)">{}</span>
</div>"#,
                esc(save_path),
                esc(url),
                recv,
                total,
                esc(status),
                esc(&format_relative_time(*ts)),
            );
        }
    }
    body.push_str("</div></div>");
    wrap_shell("Downloads — charlotte://downloads", ctx, &body)
}

/// `charlotte://extensions` — installed first-party tools and a truthful
/// marketplace landing page. Third-party installs require a future signed
/// extension package/download flow and must not be presented as functional.
pub fn render_extensions_page(ctx: &PageContext) -> String {
    let mut body = render_topbar(ctx, "search", true);
    let shields = if ctx.shields_enabled { "Enabled" } else { "Disabled" };
    let _ = write!(body, r#"<div class="internal-wrap">
  <div class="internal-card">
    <span class="badge" style="margin-bottom:6px">charlotte://extensions</span>
    <h1 style="margin:0 0 6px;font-size:24px">Extensions Marketplace</h1>
    <p class="settings-help">Manage Charlotte's built-in browser tools and explore extension support.</p>
  </div>
  <div class="internal-card">
    <h2 style="font-size:17px">Built into Charlotte</h2>
    <div class="highlight-grid">
      <div class="highlight-box"><strong>🛡 Charlotte Shields</strong><p class="settings-help">Privacy and tracker blocking</p><span class="badge">{shields}</span><p><a class="btn-secondary" href="/internal/settings#passwords-security">Manage protection</a></p></div>
      <div class="highlight-box"><strong>✦ Charlotte AI Agent</strong><p class="settings-help">Agent workspace and provider connections</p><span class="badge">Built in</span><p><a class="btn-secondary" href="/internal/agent">Open Agent</a></p></div>
      <div class="highlight-box"><strong>⚡ MCP Tools</strong><p class="settings-help">Connected tools are configured in the agent workspace</p><span class="badge">Built in</span><p><a class="btn-secondary" href="/internal/settings#ai-agent">Configure AI</a></p></div>
    </div>
  </div>
  <div class="internal-card">
    <h2 style="font-size:17px">Third-party extensions</h2>
    <p class="settings-help">Extension manifests and content-script matching exist in Charlotte's Rust engine, but there is currently no verified marketplace download, install, update, or permission-review pipeline. Third-party installation is not available yet.</p>
    <div class="empty-note">No third-party extensions installed.</div>
  </div>
</div>"#);
    wrap_shell("Extensions Marketplace — charlotte://extensions", ctx, &body)
}

/// `charlotte://about`
pub fn render_about_page(ctx: &PageContext, server_url: &str) -> String {
    let mut body = render_topbar(ctx, "search", true);
    let _ = write!(
        body,
        r#"<div class="internal-wrap">
  <div class="internal-card">
    <span class="badge cloud" style="margin-bottom:8px">charlotte://about</span>
    <h1 style="margin:0 0 8px;font-size:26px">Charlotte Browser v{}</h1>
    <p style="margin:0 0 16px;color:var(--text-muted);font-size:14px">Native Rust Autonomous AI Browser with Local &amp; E2B Cloud Agent Execution.</p>
    <div class="internal-row"><span>Platform</span><strong>{} ({})</strong></div>
    <div class="internal-row"><span>Web Engine</span><strong>{}</strong></div>
    <div class="internal-row"><span>Cloud Server</span><strong>{}</strong></div>
    <div class="internal-row"><span>Auto-Updates</span><strong>Enabled (checked on launch)</strong></div>
  </div>
</div>"#,
        env!("CARGO_PKG_VERSION"),
        crate::updater::current_platform(),
        crate::updater::current_arch(),
        if cfg!(target_os = "windows") {
            "WebView2 COM (Windows 7–11 ESR / Evergreen)"
        } else {
            "WebKitGTK (Linux)"
        },
        esc(server_url),
    );
    wrap_shell("About — charlotte://about", ctx, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ctx(is_private: bool) -> PageContext {
        PageContext {
            token: "tok123".into(),
            tabs: vec![],
            active_tab_id: None,
            pinned_sites: vec![],
            is_private,
            cloud_mode_enabled: false,
            selected_model: "charlotte-ai".into(),
            tab_filter: String::new(),
            shields_enabled: true,
            blocked_requests: 0,
            user_initials: "CH".into(),
            sidebar_auto_hide: true,
            current_internal_url: "charlotte://search".into(),
            status_banner: None,
        }
    }

    #[test]
    fn settings_navigation_and_sidebar_preferences_are_rendered() {
        let ctx = sample_ctx(false);
        let page = render_settings_page(&ctx, &[], TabLayout::Horizontal, "off", "Ready", "https://example.org");
        assert!(page.contains("Personalization"));
        assert!(page.contains("Passwords &amp; Security"));
        assert!(page.contains("Extensions Marketplace"));
        assert!(page.contains("sidebar_auto_hide"));
        assert!(!page.contains("Internal Charlotte Pages"));
        assert!(page.contains("profile-trigger"));
        assert!(page.contains("rail-pin-toggle"));
    }

    #[test]
    fn search_mode_has_no_fake_placeholders_when_empty() {
        let ctx = sample_ctx(false);
        let html = render_search_mode(&ctx, &[]);
        assert!(html.contains("Search web"));
        assert!(html.contains("Last pages visited"));
        assert!(!html.contains("Figma workspace"));
        assert!(!html.contains("Project notes"));
        assert!(!html.contains("Team inbox"));
        assert!(!html.contains("Work it on Cloud"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn agent_mode_shows_work_on_cloud_toggle_and_no_fake_previous_work() {
        let mut ctx = sample_ctx(false);
        ctx.cloud_mode_enabled = true;
        let html = render_agent_mode(&ctx, &[]);
        assert!(html.contains("What should we work on?"));
        assert!(html.contains("Work it on Cloud"));
        assert!(html.contains("Previous work"));
        assert!(!html.contains("Fix Auth Layout"));
        assert!(!html.contains("Implement Compliance KYB"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn private_mode_renders_black_theme_and_private_search_heading() {
        let ctx = sample_ctx(true);
        let html = render_private_mode(&ctx);
        assert!(html.contains("Private search"));
        assert!(html.contains("--bg:#111113"));
        assert!(!html.contains("Last pages visited"));
    }

    #[test]
    fn updates_page_renders_server_updates_and_continue_button() {
        let ctx = sample_ctx(false);
        let updates = vec![BrowserUpdate {
            id: "u1".into(),
            version: "0.5.0".into(),
            channel: "stable".into(),
            platform: "all".into(),
            arch: "all".into(),
            title: "Charlotte 0.5.0".into(),
            summary: "New release".into(),
            release_notes: "- Faster startup".into(),
            highlights: vec![],
            download_url: String::new(),
            sha256: String::new(),
            file_size_bytes: 0,
            is_mandatory: false,
            published_at: String::new(),
        }];
        let html = render_updates_page(&ctx, &updates, true, "Loaded live", Some("0.5.0"));
        assert!(html.contains("charlotte://updates"));
        assert!(html.contains("Charlotte 0.5.0"));
        assert!(html.contains("Continue"));
    }
}
