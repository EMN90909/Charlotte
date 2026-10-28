/**
 * One Electron webRequest hook for OpenShield across all browsing tabs.
 * Browser UI, chat and top-level navigations are never filtered.
 */
import { session, webContents, type WebContents } from 'electron';
import type { AdBlockSettings, AdBlockStatus } from '../../shared/types';
import { DEFAULT_AD_BLOCK_SETTINGS } from '../../shared/types';
import { matchesDomain, normalizeAdBlockSettings } from './rules';
import { loadAdBlockSettings, saveAdBlockSettings } from './store';
import { openShieldBlocks, openShieldCosmetics, openShieldStats, startOpenShield, stopOpenShield } from './openshieldAdapter';

let settings: AdBlockSettings = loadAdBlockSettings();
let exceptions = new Set(settings.allowedSites);
let blockedCount = 0;
let statusListener: (() => void) | null = null;
let listTabs: (() => WebContents[]) | null = null;
let notificationTimer: ReturnType<typeof setTimeout> | null = null;

// Track inserted CSS per tab, so changing mode / exceptions removes it.
// CSS is injected as Chromium USER stylesheet; no JS/scriptlet is executed.
const cssKeys = new Map<number, string>();
const cssVersions = new Map<number, number>();

function notify(): void { statusListener?.(); }

export function getAdBlockStatus(): AdBlockStatus {
  return {
    settings: { mode: settings.mode, allowedSites: [...settings.allowedSites] },
    blockedCount,
    engine: openShieldStats(),
  };
}

function isException(pageUrl: string): boolean {
  if (!exceptions.size) return false;
  try {
    const url = new URL(pageUrl);
    return (url.protocol === 'https:' || url.protocol === 'http:') && matchesDomain(url.hostname, exceptions);
  } catch { return false; }
}

export function updateAdBlockSettings(raw: unknown): AdBlockStatus {
  settings = normalizeAdBlockSettings(raw, DEFAULT_AD_BLOCK_SETTINGS);
  exceptions = new Set(settings.allowedSites);
  saveAdBlockSettings(settings);
  refreshCosmetics();
  return getAdBlockStatus();
}

export function blockAdRequest(requestUrl: string, pageUrl: string, resourceType: string): boolean {
  if (settings.mode === 'off' || resourceType === 'mainFrame' || isException(pageUrl)) return false;
  try {
    const resource = new URL(requestUrl);
    const top = new URL(pageUrl);
    if (!['http:', 'https:'].includes(resource.protocol)) return false;
    // An ad platform's own website must be navigable and usable.
    if (resource.hostname === top.hostname) return false;
  } catch { return false; }
  return openShieldBlocks(requestUrl, pageUrl, resourceType, settings.mode === 'strict');
}

export async function applyAdBlockCosmetics(wc: WebContents): Promise<void> {
  if (wc.isDestroyed()) return;
  const id = wc.id;
  const revision = (cssVersions.get(id) ?? 0) + 1;
  cssVersions.set(id, revision);
  const prevKey = cssKeys.get(id);
  cssKeys.delete(id);
  if (prevKey) {
    try { await wc.removeInsertedCSS(prevKey); } catch { /* previous document navigated away */ }
  }
  if (wc.isDestroyed() || cssVersions.get(id) !== revision) return;
  const url = wc.getURL();
  if (settings.mode === 'off' || isException(url)) return;
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return;
    const css = openShieldCosmetics(url, settings.mode === 'strict');
    if (!css) return;
    const key = await wc.insertCSS(css, { cssOrigin: 'user' });
    if (wc.isDestroyed()) return;
    if (cssVersions.get(id) !== revision) {
      try { await wc.removeInsertedCSS(key); } catch { /* stale document */ }
    } else {
      cssKeys.set(id, key);
    }
  } catch {
    // A page might navigate or close while an async CSS insert is pending.
  }
}

export function detachAdBlockCosmetics(wc: WebContents): void {
  cssKeys.delete(wc.id);
  cssVersions.delete(wc.id);
}

function refreshCosmetics(): void {
  for (const wc of listTabs?.() ?? []) void applyAdBlockCosmetics(wc);
}

export function installAdBlocker(
  isBrowsingTab: (wc: WebContents) => boolean,
  onStatus: () => void,
  getTabs: () => WebContents[],
): void {
  statusListener = onStatus;
  listTabs = getTabs;
  // Bundled host rules are active immediately; cached community filters
  // hydrate next and refresh in the background without external services.
  startOpenShield(() => { refreshCosmetics(); notify(); });

  // Electron supports a single onBeforeRequest hook per session.
  session.defaultSession.webRequest.onBeforeRequest({ urls: ['*://*/*'] }, (details, callback) => {
    const wc = details.webContents ?? (typeof details.webContentsId === 'number'
      ? webContents.fromId(details.webContentsId) : undefined);
    if (!wc || wc.isDestroyed() || !isBrowsingTab(wc) || settings.mode === 'off' || details.resourceType === 'mainFrame') {
      callback({});
      return;
    }
    if (!blockAdRequest(details.url, wc.getURL(), details.resourceType)) {
      callback({});
      return;
    }
    blockedCount++;
    callback({ cancel: true });
    if (!notificationTimer) notificationTimer = setTimeout(() => {
      notificationTimer = null;
      notify();
    }, 250);
  });
}

export function stopAdBlocker(): void {
  stopOpenShield();
  if (notificationTimer) clearTimeout(notificationTimer);
  notificationTimer = null;
  statusListener = null;
  listTabs = null;
  cssKeys.clear();
  cssVersions.clear();
}
