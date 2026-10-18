/**
 * Remembers whether the first-run onboarding flow (see
 * renderer/chrome/components/onboarding/OnboardingModal.tsx) has been
 * finished, in its own `userData/onboarding.json` - same "one small file
 * per device preference" convention as userStore.ts's theme.json. A
 * missing or corrupt file means "not done yet", so existing installs that
 * predate onboarding see it once too.
 */
import fs from 'node:fs';
import path from 'node:path';
import { app } from 'electron';

interface OnboardingFile {
  completed: boolean;
}

function onboardingFilePath(): string {
  return path.join(app.getPath('userData'), 'onboarding.json');
}

export function isOnboardingCompleted(): boolean {
  try {
    const raw = fs.readFileSync(onboardingFilePath(), 'utf-8');
    return (JSON.parse(raw) as Partial<OnboardingFile>).completed === true;
  } catch {
    return false;
  }
}

export function markOnboardingCompleted(): void {
  fs.mkdirSync(path.dirname(onboardingFilePath()), { recursive: true });
  fs.writeFileSync(onboardingFilePath(), JSON.stringify({ completed: true } satisfies OnboardingFile), 'utf-8');
}
