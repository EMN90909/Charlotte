/**
 * Charlotte Shields: synchronous, bounded request decisions in the Electron
 * main process, with a small in-memory inspector and opt-in Strict scriptlets.
 * Browser chrome and chat are never intercepted or inspected.
 */
import { session, webContents, type WebContents } from 'electron';
import type { AdBlockSettings, AdBlockStatus, AdBlockRequestEntry, AdBlockResource, AdBlockRuleSource } from '../../shared/types';
import { DEFAULT_AD_BLOCK_SETTINGS } from '../../shared/types';
import { matchesDomain, normalizeAdBlockSettings } from './rules';
import { loadAdBlockSettings, saveAdBlockSettings } from './store';
import { openShieldDecision, openShieldCosmetics, openShieldScriptlets, openShieldStats, startOpenShield, stopOpenShield } from './openshieldAdapter';

const MAX_RECENT = 120;
let settings: AdBlockSettings = loadAdBlockSettings();
let siteExceptions = new Set(settings.allowedSites);
let allowedHosts = new Set(settings.allowedHosts);
let blockedHosts = new Set(settings.blockedHosts);
let blockedCount = 0;
let allowedCount = 0;
let inspectedCount = 0;
let accumulatedEvaluationMs = 0;
let scriptletsApplied = 0;
let requestId = 0;
const recent: AdBlockRequestEntry[] = [];
let statusListener: (() => void) | null = null;
let listTabs: (() => WebContents[]) | null = null;
let notificationTimer: ReturnType<typeof setTimeout> | null = null;
let hydrateTimer: ReturnType<typeof setTimeout> | null = null;

const cssKeys = new Map<number, string>();
const cssVersions = new Map<number, number>();

function notify(): void { statusListener?.(); }
function queueStatus(): void {
  // Keep cross-process traffic independent of request frequency.
  if (!notificationTimer) notificationTimer = setTimeout(() => {
    notificationTimer = null;
    notify();
  }, 300);
}
function safeHost(url: string): string {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'https:' || parsed.protocol === 'http:' ? parsed.hostname.toLowerCase() : '';
  } catch { return ''; }
}
function isExcepted(siteHost: string): boolean {
  return Boolean(siteHost && matchesDomain(siteHost, siteExceptions));
}
function resourceCategory(type: string): AdBlockResource {
  switch (type) {
    case 'script': return 'JavaScript';
    case 'image': return 'Images';
    case 'media': return 'Audio / video';
    case 'stylesheet': return 'Styles';
    case 'font': return 'Fonts';
    case 'mainFrame':
    case 'subFrame': return 'Documents';
    default: return 'Other';
  }
}

export function getAdBlockStatus(): AdBlockStatus {
  return {
    settings: { ...settings, allowedSites: [...settings.allowedSites], allowedHosts: [...settings.allowedHosts], blockedHosts: [...settings.blockedHosts] },
    blockedCount,
    allowedCount,
    averageEvaluationMs: inspectedCount ? Math.round(accumulatedEvaluationMs / inspectedCount * 1000) / 1000 : 0,
    recentRequests: recent.slice(0, 60),
    scriptletsApplied,
    engine: openShieldStats(),
  };
}

export function updateAdBlockSettings(raw: unknown): AdBlockStatus {
  settings = normalizeAdBlockSettings(raw, DEFAULT_AD_BLOCK_SETTINGS);
  siteExceptions = new Set(settings.allowedSites);
  allowedHosts = new Set(settings.allowedHosts);
  blockedHosts = new Set(settings.blockedHosts);
  saveAdBlockSettings(settings);
  refreshCosmetics();
  return getAdBlockStatus();
}

export function inspectAdRequest(requestUrl: string, pageUrl: string, type: string): {
  blocked: boolean; source: AdBlockRuleSource;
} {
  if (settings.mode === 'off') return { blocked: false, source: 'disabled' };
  if (type === 'mainFrame') return { blocked: false, source: 'no match' };
  const siteHost = safeHost(pageUrl);
  const requestHost = safeHost(requestUrl);
  if (!siteHost || !requestHost) return { blocked: false, source: 'no match' };
  if (isExcepted(siteHost)) return { blocked: false, source: 'site exception' };
  if (matchesDomain(requestHost, allowedHosts)) return { blocked: false, source: 'manual allow' };
  if (matchesDomain(requestHost, blockedHosts)) return { blocked: true, source: 'manual block' };
  if (requestHost === siteHost) return { blocked: false, source: 'no match' };
  const decision = openShieldDecision(requestUrl, pageUrl, type, settings.mode === 'strict');
  return { blocked: decision.blocked, source: decision.source === 'none' ? 'no match' : decision.source };
}

export function blockAdRequest(requestUrl: string, pageUrl: string, type: string): boolean {
  return inspectAdRequest(requestUrl, pageUrl, type).blocked;
}

function recordRequest(requestUrl: string, pageUrl: string, type: string, decision: { blocked: boolean; source: AdBlockRuleSource }, elapsedMs: number): void {
  const entry: AdBlockRequestEntry = {
    id: ++requestId,
    // Do not store paths, search parameters, cookies or request/response bodies.
    site: safeHost(pageUrl),
    host: safeHost(requestUrl),
    resource: resourceCategory(type),
    action: decision.blocked ? 'block' : 'allow',
    rule: decision.source,
    elapsedMs: Math.round(elapsedMs * 1000) / 1000,
    at: Date.now(),
  };
  recent.unshift(entry);
  if (recent.length > MAX_RECENT) recent.pop();
  if (decision.blocked) blockedCount++;
  else allowedCount++;
  inspectedCount++;
  accumulatedEvaluationMs += elapsedMs;
  queueStatus();
}

export async function applyAdBlockCosmetics(wc: WebContents): Promise<void> {
  if (wc.isDestroyed()) return;
  const id = wc.id;
  const revision = (cssVersions.get(id) ?? 0) + 1;
  cssVersions.set(id, revision);
  const prevKey = cssKeys.get(id);
  cssKeys.delete(id);
  if (prevKey) {
    try { await wc.removeInsertedCSS(prevKey); } catch { /* previous navigation */ }
  }
  if (wc.isDestroyed() || cssVersions.get(id) !== revision) return;
  const url = wc.getURL();
  const host = safeHost(url);
  if (settings.mode === 'off' || !host || isExcepted(host)) return;
  try {
    const css = openShieldCosmetics(url, settings.mode === 'strict');
    if (!css) return;
    const key = await wc.insertCSS(css, { cssOrigin: 'user' });
    if (wc.isDestroyed()) return;
    if (cssVersions.get(id) !== revision) {
      try { await wc.removeInsertedCSS(key); } catch { /* old document */ }
    } else cssKeys.set(id, key);
  } catch { /* page navigated or was destroyed */ }
}

/**
 * Strict-only, allowlisted scriptlets run in the page world after DOM-ready.
 * Untrusted filter text is NEVER executed. Reload to remove page JS effects.
 * Unlike document-start injection, this affects late/dynamic ad scripts.
 */
export function applyAdBlockScriptlets(wc: WebContents): void {
  if (wc.isDestroyed() || settings.mode !== 'strict') return;
  const url = wc.getURL();
  const host = safeHost(url);
  if (!host || isExcepted(host)) return;
  const source = openShieldScriptlets(url);
  if (!source) return;
  void wc.executeJavaScript(source).then(() => {
    if (!wc.isDestroyed()) { scriptletsApplied++; queueStatus(); }
  }).catch(() => { /* site navigated, strict site may restrict execution */ });
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
  hydrateTimer = setTimeout(() => {
    hydrateTimer = null;
    startOpenShield(() => { refreshCosmetics(); notify(); });
  }, 1500);

  session.defaultSession.webRequest.onBeforeRequest({ urls: ['*://*/*'] }, (details, callback) => {
    const wc = details.webContents ?? (typeof details.webContentsId === 'number'
      ? webContents.fromId(details.webContentsId) : undefined);
    if (!wc || wc.isDestroyed() || !isBrowsingTab(wc)) {
      callback({});
      return;
    }
    const start = performance.now();
    const pageUrl = wc.getURL();
    const decision = inspectAdRequest(details.url, pageUrl, details.resourceType);
    // Return to Chromium synchronously; no IPC, network or disk on this path.
    callback(decision.blocked ? { cancel: true } : {});
    recordRequest(details.url, pageUrl, details.resourceType, decision, performance.now() - start);
  });
}

export function stopAdBlocker(): void {
  if (hydrateTimer) clearTimeout(hydrateTimer);
  hydrateTimer = null;
  stopOpenShield();
  if (notificationTimer) clearTimeout(notificationTimer);
  notificationTimer = null;
  statusListener = null;
  listTabs = null;
  cssKeys.clear();
  cssVersions.clear();
}
