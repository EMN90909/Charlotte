/** Small device-local preference file, independent of chat safety filters. */
import fs from 'node:fs';
import path from 'node:path';
import { app } from 'electron';
import type { AdBlockSettings } from '../../shared/types';
import { DEFAULT_AD_BLOCK_SETTINGS } from '../../shared/types';
import { normalizeAdBlockSettings } from './rules';

function filePath(): string {
  return path.join(app.getPath('userData'), 'adblock.json');
}

export function loadAdBlockSettings(): AdBlockSettings {
  try {
    const parsed: unknown = JSON.parse(fs.readFileSync(filePath(), 'utf8'));
    return normalizeAdBlockSettings(parsed, DEFAULT_AD_BLOCK_SETTINGS);
  } catch {
    return { ...DEFAULT_AD_BLOCK_SETTINGS, allowedSites: [] };
  }
}

export function saveAdBlockSettings(settings: AdBlockSettings): void {
  const filename = filePath();
  fs.mkdirSync(path.dirname(filename), { recursive: true });
  fs.writeFileSync(filename, JSON.stringify(settings, null, 2), 'utf8');
}
