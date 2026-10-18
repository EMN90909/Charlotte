import { useEffect, useRef, useState } from 'react';

export type ClaimStatus = 'idle' | 'pending' | 'taken' | 'error';

/**
 * One explicit username claim against a chat server (see main/ipc.ts's
 * claimChatServerUsername) - shared by onboarding's username step and
 * Settings > Chat Servers. Claims only on an explicit call, never while
 * typing: every successful claim reserves that name on the server forever,
 * so a half-typed name must never be grabbed by accident.
 */
export function useUsernameClaim(serverId: string | null, onClaimed?: () => void) {
  const [status, setStatus] = useState<ClaimStatus>('idle');
  const [message, setMessage] = useState<string | null>(null);
  const pendingRef = useRef<string | null>(null);
  const onClaimedRef = useRef(onClaimed);
  onClaimedRef.current = onClaimed;

  useEffect(() => {
    pendingRef.current = null;
    setStatus('idle');
    setMessage(null);
    return window.paperkite.onChatServerUsernameClaimResult((result) => {
      // Ignore stale answers for a name the user has since moved past.
      if (result.serverId !== serverId || result.username !== pendingRef.current) return;
      pendingRef.current = null;
      if (result.ok) {
        setStatus('idle');
        onClaimedRef.current?.();
      } else {
        setStatus(result.reason === 'taken' ? 'taken' : 'error');
        setMessage(result.message ?? null);
      }
    });
  }, [serverId]);

  const claim = (username: string) => {
    const trimmed = username.trim();
    if (!serverId || !trimmed || status === 'pending') return;
    pendingRef.current = trimmed;
    setStatus('pending');
    setMessage(null);
    window.paperkite.claimChatServerUsername({ serverId, username: trimmed });
  };

  /** Clears a shown error once the user starts editing again. */
  const reset = () => {
    if (status === 'pending') return;
    setStatus('idle');
    setMessage(null);
  };

  const errorText =
    status === 'taken' || status === 'error'
      ? (message ?? (status === 'taken' ? 'That one is taken. Try another.' : "Couldn't reach the server. Try again."))
      : null;

  return { status, errorText, claim, reset };
}
