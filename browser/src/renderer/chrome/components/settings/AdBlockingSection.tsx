import { useState } from 'react';
import type { AdBlockSettings, AdBlockStatus } from '../../../../shared/types';

interface AdBlockingSectionProps {
  status: AdBlockStatus;
  onSave: (settings: AdBlockSettings) => void;
}

const MODES: Array<{ value: AdBlockSettings['mode']; label: string }> = [
  { value: 'off', label: 'Off' },
  { value: 'balanced', label: 'Balanced' },
  { value: 'strict', label: 'Strict' },
];

export function AdBlockingSection({ status, onSave }: AdBlockingSectionProps) {
  const [domain, setDomain] = useState('');
  const { settings, blockedCount } = status;
  const candidate = domain.trim().toLowerCase().replace(/\.$/, '');
  const valid = /^(?=.{1,253}$)[a-z0-9]+(?:[a-z0-9.-]*[a-z0-9])?\.[a-z0-9.-]*[a-z0-9]$/.test(candidate)
    && !candidate.split('.').some((label) => !label || label.length > 63 || label.startsWith('-') || label.endsWith('-'));
  const canAdd = valid && !settings.allowedSites.includes(candidate) && settings.allowedSites.length < 128;

  return (
    <section className="settings-section">
      <h2>Ad &amp; tracker blocking</h2>
      <p className="settings-hint">
        OpenShield blocks advertising and tracking requests locally, with cosmetic hiding for ad placeholders. No extra extension or service.
      </p>
      <div className="segmented" role="radiogroup" aria-label="Ad blocking level">
        {MODES.map(({ value, label }) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={settings.mode === value}
            className={'segmented__option' + (settings.mode === value ? ' segmented__option--active' : '')}
            onClick={() => onSave({ ...settings, mode: value })}
          >
            {label}
          </button>
        ))}
      </div>
      <p className="settings-hint">
        {settings.mode === 'off'
          ? 'Protection is off. Pages load normally.'
          : settings.mode === 'balanced'
            ? 'Balanced blocks known advertising servers with minimal site breakage.'
            : 'Strict also blocks common analytics and tracking services. Some site features may need an exception.'}
      </p>
      <div className="adblock-stat" role="status" aria-live="off">
        <strong>{blockedCount.toLocaleString()}</strong>
        <span>requests blocked this session</span>
      </div>

      <h2>Filter engine</h2>
      <p className="settings-hint">
        {status.engine ? `${status.engine.networkRules.toLocaleString()} network rules, ${status.engine.cosmeticRules.toLocaleString()} cosmetic rules. ${status.engine.cachedLists} community lists cached.` : 'Loading local protection.'}
        {status.engine?.lastUpdated ? ` Updated ${new Date(status.engine.lastUpdated).toLocaleDateString()}.` : ''}
      </p>
      {status.engine?.lastError && <p className="settings-hint" role="status">Community filters could not update. Existing offline protection remains enabled.</p>}
      <h2>Site exceptions</h2>
      <p className="settings-hint">
        Allow ads and trackers on sites that need them to work. The exception covers the domain and its subdomains.
      </p>
      <form
        className="proxy-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (!canAdd) return;
          onSave({ ...settings, allowedSites: [...settings.allowedSites, candidate] });
          setDomain('');
        }}
      >
        <label className="proxy-form__field">
          <span>Domain</span>
          <input
            type="text"
            value={domain}
            placeholder="example.com"
            aria-label="Allow ads on domain"
            spellCheck={false}
            onChange={(event) => setDomain(event.target.value)}
          />
        </label>
        <button type="submit" className="proxy-form__save" disabled={!canAdd}>Add exception</button>
      </form>
      {settings.allowedSites.length > 0 && (
        <div className="adblock-exceptions">
          {settings.allowedSites.map((site) => (
            <div className="adblock-exception" key={site}>
              <span>{site}</span>
              <button
                type="button"
                aria-label={`Remove exception for ${site}`}
                onClick={() => onSave({ ...settings, allowedSites: settings.allowedSites.filter((value) => value !== site) })}
              >Remove</button>
            </div>
          ))}
        </div>
      )}
      <p className="settings-hint">
        Changes apply immediately to new requests. Reload an already open page to replace blocked resources.
        The built-in rules work offline. EasyList and EasyPrivacy refresh from their official feeds when connected.
      </p>
    </section>
  );
}
