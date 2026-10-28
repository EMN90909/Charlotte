/**
 * OpenShield adapter for Charlotte's main Electron bundle.
 * Source imports are bundled by the ROOT Vite build, without a second
 * executable, package install or native extension.
 */
import fs from 'node:fs';
import path from 'node:path';
import { app } from 'electron';
import { OpenShield } from './src/index';
import { DEFAULT_LISTS } from './src/lists/defaults';
import { ADS, TRACKERS } from './rules';

const LIST_IDS = ['easylist', 'easyprivacy'] as const;
type ListId = (typeof LIST_IDS)[number];
const MAX_BYTES = 5 * 1024 * 1024;
const MAX_AGE = 7 * 86400_000;
const TIMEOUT = 12_000;
const adRules = [...ADS].map((host) => `||${host}^`).join('\n');
const trackerRules = [...TRACKERS].map((host) => `||${host}^`).join('\n');
let ads = makeShield(adRules);
let trackers = makeShield(trackerRules);
let listCount = 0;
let lastUpdated: string | null = null;
let lastError: string | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;

function makeShield(rules: string, extra?: string): OpenShield {
  const shield = new OpenShield();
  shield.addFilters(rules);
  if (extra) shield.addFilters(extra);
  return shield;
}

export function openShieldStats() {
  const a = ads.getStats();
  const b = trackers.getStats();
  return {
    networkRules: a.networkFilterCount + b.networkFilterCount,
    cosmeticRules: a.cosmeticFilterCount + b.cosmeticFilterCount,
    cachedLists: listCount,
    lastUpdated,
    lastError,
  };
}

/** The caller owns mode, tab-scoping, exceptions and main-frame bypass. */
export function openShieldBlocks(url: string, siteUrl: string, resource: string, strict: boolean): boolean {
  const type = ({ mainFrame: 'document', subFrame: 'subdocument',
    xhr: 'xmlhttprequest', fetch: 'xmlhttprequest', webSocket: 'websocket',
    cspReport: 'ping' } as Record<string, string>)[resource] ?? resource.toLowerCase();
  const first = ads.check(url, siteUrl, type);
  if (first.blocked) return true;
  if (first.exception) return false;
  return strict && trackers.check(url, siteUrl, type).blocked;
}

export function openShieldCosmetics(url: string, strict: boolean): string {
  const selectors = ads.getCosmeticsForUrl(url).hidingSelectors;
  if (strict) selectors.push(...trackers.getCosmeticsForUrl(url).hidingSelectors);
  const rules: string[] = [];
  const seen = new Set<string>();
  for (const selector of selectors) {
    if (rules.length >= 800) break;
    if (!selector || selector.length > 180 || /[{};@\r\n]|:has-text\(|:matches-css\(|:xpath\(|:remove\(/i.test(selector)) continue;
    if (seen.has(selector)) continue;
    seen.add(selector);
    // Only selector-hiding CSS. Deliberately ignore scriptlets, arbitrary
    // CSS declarations, redirects and all other executable list features.
    rules.push(`${selector}{display:none!important;}`);
  }
  return rules.join('\n').slice(0, 96 * 1024);
}

function cachePath(id: ListId): string {
  return path.join(app.getPath('userData'), 'adblock-lists', `${id}.txt`);
}
function cached(id: ListId): { text: string; timestamp: number } | null {
  try {
    const file = cachePath(id);
    const stat = fs.statSync(file);
    if (stat.size < 1 || stat.size > MAX_BYTES) return null;
    return { text: fs.readFileSync(file, 'utf8'), timestamp: stat.mtimeMs };
  } catch { return null; }
}

async function download(id: ListId): Promise<string> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), TIMEOUT);
  try {
    const response = await fetch(DEFAULT_LISTS[id].url, {
      signal: controller.signal, redirect: 'error',
      headers: { Accept: 'text/plain' },
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    if (Number(response.headers.get('content-length') || 0) > MAX_BYTES) throw new Error('list too large');
    if (!response.body) throw new Error('missing response body');
    const chunks: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of response.body) {
      size += chunk.byteLength;
      if (size > MAX_BYTES) throw new Error('list too large');
      chunks.push(chunk);
    }
    const text = Buffer.concat(chunks).toString('utf8');
    if (!text.includes('\n') || !text.includes('!')) throw new Error('invalid list');
    return text;
  } finally {
    clearTimeout(timeout);
  }
}

function loadPair(a?: string, b?: string): void {
  // Atomic replacement prevents duplicates after each update.
  const nextAds = makeShield(adRules, a);
  const nextTrackers = makeShield(trackerRules, b);
  ads = nextAds;
  trackers = nextTrackers;
  listCount = Number(Boolean(a)) + Number(Boolean(b));
}

/** Start after Electron is ready. Bundled fallback is active immediately. */
export function startOpenShield(onUpdated: () => void): void {
  const stored = { easylist: cached('easylist'), easyprivacy: cached('easyprivacy') };
  try {
    if (stored.easylist || stored.easyprivacy) {
      loadPair(stored.easylist?.text, stored.easyprivacy?.text);
      lastUpdated = new Date(Math.max(stored.easylist?.timestamp ?? 0, stored.easyprivacy?.timestamp ?? 0)).toISOString();
      onUpdated();
    }
  } catch (error) {
    lastError = error instanceof Error ? error.message : 'Invalid local filter cache';
  }
  refreshTimer = setTimeout(() => {
    void (async () => {
      const texts = { easylist: stored.easylist?.text, easyprivacy: stored.easyprivacy?.text };
      const errors: string[] = [];
      let changed = false;
      for (const id of LIST_IDS) {
        if (stored[id] && Date.now() - stored[id].timestamp < MAX_AGE) continue;
        try {
          const text = await download(id);
          // Update active matcher before saving the download.
          texts[id] = text;
          const file = cachePath(id);
          fs.mkdirSync(path.dirname(file), { recursive: true });
          fs.writeFileSync(file + '.tmp', text, 'utf8');
          fs.renameSync(file + '.tmp', file);
          changed = true;
        } catch (error) {
          errors.push(`${id}: ${error instanceof Error ? error.message : 'download failed'}`);
        }
      }
      if (changed) {
        loadPair(texts.easylist, texts.easyprivacy);
        lastUpdated = new Date().toISOString();
      }
      lastError = errors.length ? errors.join('; ').slice(0, 240) : null;
      onUpdated();
    })().catch((error: unknown) => {
      lastError = error instanceof Error ? error.message : 'Filter list update failed';
      onUpdated();
    });
  }, 8_000);
}
export function stopOpenShield(): void {
  if (refreshTimer) clearTimeout(refreshTimer);
  refreshTimer = null;
}
