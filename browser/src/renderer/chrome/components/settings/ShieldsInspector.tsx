import { useState } from 'react';
import type { AdBlockSettings, AdBlockStatus, AdBlockResource, AdBlockRequestEntry } from '../../../../shared/types';

interface Props {
  status: AdBlockStatus;
  currentSiteUrl: string;
  onSave: (settings: AdBlockSettings) => void;
}
const TYPES: Array<'all' | 'Advertisements' | AdBlockResource> =
  ['all', 'Advertisements', 'JavaScript', 'Images', 'Audio / video', 'Styles', 'Fonts', 'Documents', 'Other'];

export function ShieldsInspector({ status, currentSiteUrl, onSave }: Props) {
  const [decisionFilter, setDecisionFilter] = useState<'all' | 'allow' | 'block'>('all');
  const [resourceFilter, setResourceFilter] = useState<string>('all');
  const [siteOnly, setSiteOnly] = useState(true);
  const settings = status.settings;
  let site = '';
  try {
    const url = new URL(currentSiteUrl);
    if (url.protocol === 'http:' || url.protocol === 'https:') site = url.hostname;
  } catch { /* internal new-tab page */ }

  const entries = (status.recentRequests ?? []).filter((entry) =>
    (!siteOnly || !site || entry.site === site) &&
    (decisionFilter === 'all' || entry.action === decisionFilter) &&
    (resourceFilter === 'all' || (resourceFilter === 'Advertisements'
      ? entry.rule === 'advertising' || entry.rule === 'manual block'
      : entry.resource === resourceFilter))
  ).slice(0, 35);

  const editHost = (entry: AdBlockRequestEntry, action: 'allow' | 'block' | 'reset') => {
    const host = entry.host;
    onSave({
      ...settings,
      allowedHosts: settings.allowedHosts.filter((v) => v !== host).concat(action === 'allow' ? [host] : []),
      blockedHosts: settings.blockedHosts.filter((v) => v !== host).concat(action === 'block' ? [host] : []),
    });
  };

  return (
    <div className="shields-inspector">
      <h2>Live request inspector</h2>
      <p className="settings-hint">
        Classifies website requests for JavaScript, images, music/video and more.
        Decisions happen before resources load. Only hostnames and decisions are kept in memory.
      </p>
      <div className="shields-stats">
        <div><strong>{(status.allowedCount ?? 0).toLocaleString()}</strong><span>Allowed</span></div>
        <div><strong>{status.blockedCount.toLocaleString()}</strong><span>Blocked</span></div>
        <div><strong>{(status.averageEvaluationMs ?? 0).toFixed(3)} ms</strong><span>Avg. rule evaluation</span></div>
      </div>
      <div className="shields-controls">
        <div className="segmented" role="radiogroup" aria-label="Request decision filter">
          {(['all', 'block', 'allow'] as const).map((value) => (
            <button key={value} type="button" role="radio" aria-checked={decisionFilter === value}
              className={'segmented__option' + (decisionFilter === value ? ' segmented__option--active' : '')}
              onClick={() => setDecisionFilter(value)}>
              {value === 'all' ? 'All' : value === 'block' ? 'Blocked' : 'Allowed'}
            </button>
          ))}
        </div>
        <label className="shields-resource">Content
          <select value={resourceFilter} onChange={(event) => setResourceFilter(event.target.value)}>
            {TYPES.map((value) => <option key={value} value={value}>
              {value === 'all' ? 'All content' : value}
            </option>)}
          </select>
        </label>
      </div>
      {site && <label className="shields-site">
        <input type="checkbox" checked={siteOnly} onChange={(event) => setSiteOnly(event.target.checked)} />
        Current site only: <strong>{site}</strong>
      </label>}
      <div className="shields-list">
        {entries.length ? entries.map((entry) => {
          const override = settings.allowedHosts.includes(entry.host) || settings.blockedHosts.includes(entry.host);
          const action = override ? 'reset' : entry.action === 'block' ? 'allow' : 'block';
          const disabled = !override && (action === 'allow' ? settings.allowedHosts.length >= 128 : settings.blockedHosts.length >= 128);
          return (
            <div className="shields-request" key={entry.id}>
              <span className={'shields-action shields-action--' + entry.action}>{entry.action}</span>
              <div className="shields-request__info">
                <strong title={entry.host}>{entry.host || 'Unknown host'}</strong>
                <span>{entry.resource} · {entry.rule} · {entry.elapsedMs.toFixed(3)} ms</span>
              </div>
              {entry.host.includes('.') && entry.host.length <= 253 && /^[a-z0-9.-]+$/.test(entry.host) && (
                <button type="button" className="shields-request__override" disabled={disabled}
                  onClick={() => editHost(entry, action)}>
                  {override ? 'Reset' : entry.action === 'block' ? 'Allow host' : 'Block host'}
                </button>
              )}
            </div>
          );
        }) : <p className="settings-hint shields-empty">No requests match these filters yet.</p>}
      </div>
      <p className="settings-hint">
        Latest 60 decisions, shown in batches. Timing is local rule evaluation, not download latency.
        Host overrides apply across sites; reset an override here to restore filter-list behavior.
      </p>
    </div>
  );
}
