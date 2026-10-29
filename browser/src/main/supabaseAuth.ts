/**
 * Charlotte desktop Supabase Auth adapter.
 *
 * All credentials and refresh tokens stay in the privileged main process.
 * Session data is only written when Electron safeStorage provides OS-backed
 * encryption; otherwise login works for this process only.
 *
 * Configured at launch with CHARLOTTE_SUPABASE_URL and
 * CHARLOTTE_SUPABASE_PUBLISHABLE_KEY. NEVER use a Supabase secret/service key.
 */
import { app, safeStorage } from 'electron';
import fs from 'node:fs';
import path from 'node:path';

export type AccountStatus = {
  configured: boolean;
  authenticated: boolean;
  email: string | null;
  persistent: boolean;
  message?: string;
};

type AuthSession = {
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
  email: string | null;
};
type AuthResponse = {
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  user?: { email?: string };
  error_description?: string;
  msg?: string;
  message?: string;
};
const rawUrl = process.env.CHARLOTTE_SUPABASE_URL?.trim() ?? '';
const key = process.env.CHARLOTTE_SUPABASE_PUBLISHABLE_KEY?.trim() ?? '';
const validUrl = (() => {
  try {
    const u = new URL(rawUrl);
    return u.protocol === 'https:' || (u.protocol === 'http:' && (u.hostname === 'localhost' || u.hostname === '127.0.0.1'));
  } catch { return false; }
})();
const configured = Boolean(validUrl && key.startsWith('sb_publishable_'));
let session: AuthSession | null = null;
let loaded = false;

function diskPath(): string {
  return path.join(app.getPath('userData'), 'supabase-session.enc');
}

function clear(): void {
  session = null;
  try { fs.unlinkSync(diskPath()); } catch { /* file may not exist */ }
}

function load(): void {
  if (loaded) return;
  loaded = true;
  if (!configured || !safeStorage.isEncryptionAvailable()) return;
  try {
    const encoded = fs.readFileSync(diskPath());
    const decoded = JSON.parse(safeStorage.decryptString(encoded)) as AuthSession;
    if (typeof decoded.accessToken === 'string' && typeof decoded.refreshToken === 'string' &&
        typeof decoded.expiresAt === 'number') session = decoded;
  } catch {
    clear();
  }
}

function save(next: AuthSession): void {
  session = next;
  if (!safeStorage.isEncryptionAvailable()) return;
  fs.mkdirSync(path.dirname(diskPath()), { recursive: true });
  fs.writeFileSync(diskPath(), safeStorage.encryptString(JSON.stringify(next)), { mode: 0o600 });
}

async function request(endpoint: string, init: RequestInit = {}): Promise<AuthResponse> {
  if (!configured) throw Error('Supabase Auth is not configured for Charlotte.');
  const res = await fetch(new URL(endpoint, rawUrl), {
    ...init,
    headers: {
      apikey: key,
      'content-type': 'application/json',
      ...init.headers,
    },
    signal: AbortSignal.timeout(10000),
  });
  const payload = await res.json().catch(() => ({})) as AuthResponse;
  if (!res.ok) throw Error(payload.error_description || payload.msg || payload.message || 'Authentication request failed.');
  return payload;
}

function fromResponse(result: AuthResponse): AuthSession {
  if (!result.access_token || !result.refresh_token) throw Error('Email confirmation may be required before signing in.');
  return {
    accessToken: result.access_token,
    refreshToken: result.refresh_token,
    expiresAt: Date.now() + Math.max(1, result.expires_in || 3600) * 1000,
    email: result.user?.email ?? null,
  };
}

async function validSession(): Promise<AuthSession | null> {
  load();
  if (!session) return null;
  if (session.expiresAt <= Date.now() + 60000) {
    try {
      const result = await request('/auth/v1/token?grant_type=refresh_token', {
        method: 'POST',
        body: JSON.stringify({ refresh_token: session.refreshToken }),
      });
      save(fromResponse(result));
    } catch {
      clear();
      return null;
    }
  }
  return session;
}

export async function getAccountStatus(): Promise<AccountStatus> {
  if (!configured) return { configured: false, authenticated: false, email: null, persistent: false, message: 'Set CHARLOTTE_SUPABASE_URL and CHARLOTTE_SUPABASE_PUBLISHABLE_KEY.' };
  const current = await validSession();
  if (!current) return { configured: true, authenticated: false, email: null, persistent: safeStorage.isEncryptionAvailable() };
  try {
    const user = await request('/auth/v1/user', {
      method: 'GET',
      headers: { Authorization: `Bearer ${current.accessToken}` },
    });
    current.email = user.user?.email ?? (user as { email?: string }).email ?? current.email;
    return { configured: true, authenticated: true, email: current.email, persistent: safeStorage.isEncryptionAvailable() };
  } catch {
    clear();
    return { configured: true, authenticated: false, email: null, persistent: safeStorage.isEncryptionAvailable(), message: 'Session expired. Sign in again.' };
  }
}

export async function signIn(email: string, password: string): Promise<AccountStatus> {
  if (!email || !password) throw Error('Enter your email and password.');
  const result = await request('/auth/v1/token?grant_type=password', { method: 'POST', body: JSON.stringify({ email: email.trim(), password }) });
  save(fromResponse(result));
  return getAccountStatus();
}

export async function signUp(email: string, password: string): Promise<AccountStatus> {
  if (!email || password.length < 8) throw Error('Enter an email and a password of at least 8 characters.');
  const result = await request('/auth/v1/signup', { method: 'POST', body: JSON.stringify({ email: email.trim(), password }) });
  if (result.access_token && result.refresh_token) {
    save(fromResponse(result));
    return getAccountStatus();
  }
  return { configured: true, authenticated: false, email: null, persistent: safeStorage.isEncryptionAvailable(), message: 'Check your email for a confirmation link, then sign in.' };
}

export async function signOut(): Promise<AccountStatus> {
  load();
  const previous = session;
  clear();
  if (previous && configured) {
    // Clear locally even if Supabase is temporarily unavailable.
    try {
      await request('/auth/v1/logout', { method: 'POST', headers: { Authorization: `Bearer ${previous.accessToken}` } });
    } catch { /* local credentials are already discarded */ }
  }
  return { configured, authenticated: false, email: null, persistent: safeStorage.isEncryptionAvailable() };
}
