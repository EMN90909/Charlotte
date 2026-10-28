import { useEffect, useState } from 'react';
import type { AccountStatus } from '../../../../main/supabaseAuth';

export function AccountSection() {
  const [status, setStatus] = useState<AccountStatus | null>(null);
  const [mode, setMode] = useState<'signin' | 'signup'>('signin');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');

  useEffect(() => {
    let active = true;
    void window.paperkite.getAccountStatus().then((s) => { if (active) setStatus(s); })
      .catch((e: Error) => { if (active) setNotice(e.message); });
    return () => { active = false; };
  }, []);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true); setNotice('');
    try {
      const result = mode === 'signin'
        ? await window.paperkite.signInAccount(email, password)
        : await window.paperkite.signUpAccount(email, password);
      setStatus(result);
      setNotice(result.message ?? (result.authenticated ? 'Signed in successfully.' : ''));
      setPassword('');
    } catch (err) { setNotice(err instanceof Error ? err.message : 'Authentication failed.'); }
    finally { setBusy(false); }
  };

  return (
    <section className="settings-section">
      <h2>Charlotte account</h2>
      {!status && <p>Checking account configuration…</p>}
      {status && !status.configured && <p>Account login is not enabled in this build. Configure Supabase to enable it.</p>}
      {status?.authenticated ? (
        <div>
          <p>Signed in as <strong>{status.email ?? 'Charlotte user'}</strong>.</p>
          {!status.persistent && <p>OS key encryption is unavailable; your session ends when Charlotte closes.</p>}
          <button type="button" className="settings-toggle__label" disabled={busy} onClick={() => {
            setBusy(true);
            void window.paperkite.signOutAccount().then((next) => { setStatus(next); setNotice('Signed out.'); }).catch((e: Error) => setNotice(e.message)).finally(() => setBusy(false));
          }}>Sign out</button>
        </div>
      ) : status?.configured ? (
        <form onSubmit={(e) => { void submit(e); }}>
          <p>{mode === 'signin' ? 'Sign in with your email and password.' : 'Create your Charlotte account.'}</p>
          <label htmlFor="account-email">Email</label>
          <input id="account-email" type="email" autoComplete="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
          <label htmlFor="account-password">Password</label>
          <input id="account-password" type="password" minLength={mode === 'signup' ? 8 : undefined} autoComplete={mode === 'signup' ? 'new-password' : 'current-password'} required value={password} onChange={(e) => setPassword(e.target.value)} />
          <div style={{ display: 'flex', gap: 12, marginTop: 16 }}>
            <button type="submit" disabled={busy}>{busy ? 'Please wait…' : mode === 'signin' ? 'Sign in' : 'Create account'}</button>
            <button type="button" disabled={busy} onClick={() => { setMode(mode === 'signin' ? 'signup' : 'signin'); setNotice(''); }}>
              {mode === 'signin' ? 'Create an account' : 'Have an account? Sign in'}
            </button>
          </div>
        </form>
      ) : null}
      {notice && <p role="status">{notice}</p>}
    </section>
  );
}
