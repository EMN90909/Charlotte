/**
 * Session-wide request interception. Every browsing tab uses defaultSession.
 * Never intercept the browser chrome, chat panel, or other app requests.
 */
import { session, type WebContents } from 'electron';
import type { AdBlockSettings, AdBlockStatus } from '../../shared/types';
import { DEFAULT_AD_BLOCK_SETTINGS } from '../../shared/types';
import { matchesDomain, normalizeAdBlockSettings, shouldBlockRequest } from './rules';
import { loadAdBlockSettings, saveAdBlockSettings } from './store';

let settings: AdBlockSettings = loadAdBlockSettings();
let blockedCount = 0;
let notificationTimer: ReturnType<typeof setTimeout> | null = null;
let statusListener: (() => void) | null = null;

// Precompute exceptions on edits, not for every network request.
let exceptions = new Set(settings.allowedSites);

export function getAdBlockStatus(): AdBlockStatus {
  return { settings: { mode: settings.mode, allowedSites: [...settings.allowedSites] }, blockedCount };
}

export function updateAdBlockSettings(raw: unknown): AdBlockStatus {
  settings = normalizeAdBlockSettings(raw, DEFAULT_AD_BLOCK_SETTINGS);
  exceptions = new Set(settings.allowedSites);
  saveAdBlockSettings(settings);
  statusListener?.();
  return getAdBlockStatus();
}

export function installAdBlocker(isBrowsingTab: (webContents: WebContents) => boolean, onStatus: () => void): void {
  statusListener = onStatus;

  // Electron permits only ONE listener for a given webRequest event. Keep
  // this as the one central onBeforeRequest hook; other filtering features
  // must compose with it rather than registering their own.
  session.defaultSession.webRequest.onBeforeRequest({ urls: ['*://*/*'] }, (details, callback) => {
    const wc = details.webContents;
    if (settings.mode === 'off' || details.resourceType === 'mainFrame' || !wc || wc.isDestroyed() || !isBrowsingTab(wc)) {
      callback({});
      return;
    }

    const pageUrl = wc.getURL();
    // Fast exception path before evaluating rule sets. Uses the top-level
    // page address, not a frame's untrusted referrer.
    if (exceptions.size) {
      try {
        const host = new URL(pageUrl).hostname;
        if (matchesDomain(host, exceptions)) {
          callback({});
          return;
        }
      } catch { /* A fresh tab may not have committed its first URL yet. */ }
    }

    if (!shouldBlockRequest(details.url, pageUrl, settings)) {
      callback({});
      return;
    }

    blockedCount += 1;
    callback({ cancel: true });
    // Coalesce bursts on ad-heavy pages instead of flooding React with one
    // cross-process message per blocked resource.
    if (!notificationTimer) {
      notificationTimer = setTimeout(() => {
        notificationTimer = null;
        statusListener?.();
      }, 250);
    }
  });
}
