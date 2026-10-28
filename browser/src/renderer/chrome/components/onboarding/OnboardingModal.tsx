import { useEffect, useRef, useState } from 'react';
import type {
  AgentConfig,
  BuiltinMcpServerStatus,
  ChatServerConfig,
  McpScope,
  McpServerConfig,
} from '../../../../shared/types';
import { CheckIcon } from '../../icons';
import { useUsernameClaim } from '../../useUsernameClaim';
import { AgentForm } from '../settings/AgentsSection';
import { McpServerForm } from '../settings/McpSection';
import { ProviderBadge } from '../settings/ProviderBadge';

interface OnboardingModalProps {
  onFinish: () => void;
}

const STEPS = [
  { id: 'username', label: 'Username' },
  { id: 'agents', label: 'AI agents' },
  { id: 'mcp', label: 'MCP servers' },
  { id: 'connect', label: 'Connect' },
] as const;

/**
 * First-run walkthrough, shown once (see main/onboardingStore.ts). Step 1
 * is mandatory - page chat is dead until the default server has a
 * username, and the old inline CTA in the chat panel was too easy to miss.
 * The rest are optional and reuse the exact forms Settings uses, so
 * anything added here shows up there unchanged.
 */
export function OnboardingModal({ onFinish }: OnboardingModalProps) {
  const [step, setStep] = useState(0);
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [mcpServers, setMcpServers] = useState<McpServerConfig[]>([]);
  const [usernameReady, setUsernameReady] = useState(false);

  useEffect(() => {
    const unsubs = [window.paperkite.onAgentsUpdated(setAgents), window.paperkite.onMcpServersUpdated(setMcpServers)];
    window.paperkite.requestAgents();
    window.paperkite.requestMcpServers();
    return () => unsubs.forEach((unsub) => unsub());
  }, []);

  const isLast = step === STEPS.length - 1;
  const next = () => (isLast ? onFinish() : setStep((s) => s + 1));
  const back = () => setStep((s) => Math.max(0, s - 1));

  const canContinue = step === 0 ? usernameReady : step === 1 ? agents.length > 0 : step === 2 ? mcpServers.length > 0 : true;

  return (
    <div className="modal-overlay onboarding-overlay">
      <div className="onboarding" role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
        <KiteStringProgress current={step} />

        <div className="onboarding__body" key={step}>
          <div className="onboarding__heading">
            <span className="onboarding__numeral" aria-hidden>
              {String(step + 1).padStart(2, '0')}
            </span>
            <div>
              <span className="onboarding__eyebrow">
                Step {step + 1} of {STEPS.length}
                {step > 0 && <span className="onboarding__optional">Optional</span>}
              </span>
              <h1 id="onboarding-title">{TITLES[step]}</h1>
            </div>
          </div>

          {step === 0 && <UsernameStep onReadyChange={setUsernameReady} onSkip={next} />}
          {step === 1 && <AgentsStep agents={agents} />}
          {step === 2 && <McpServersStep servers={mcpServers} />}
          {step === 3 && <ConnectStep />}
        </div>

        <footer className="onboarding__footer">
          {step > 0 ? (
            <button type="button" className="onboarding__back" onClick={back}>
              Back
            </button>
          ) : (
            <span />
          )}
          <div className="onboarding__footer-actions">
            {step > 0 && (
              <button type="button" className="onboarding__skip" onClick={next}>
                {isLast ? 'Skip' : 'Skip for now'}
              </button>
            )}
            {(step === 0 || isLast || canContinue) && (
              <button type="button" className="onboarding__primary" disabled={!canContinue} onClick={next}>
                {isLast ? 'Start browsing' : 'Continue'}
              </button>
            )}
          </div>
        </footer>
      </div>
    </div>
  );
}

const TITLES = ['Pick your username', 'Bring an AI agent', 'Plug in MCP servers', 'Let other apps drive Charlotte'];

/** The progress bar is a kite string: a dashed line with a knot per step,
 * and the kite itself sitting on whichever step you're on. */
function KiteStringProgress({ current }: { current: number }) {
  return (
    <ol className="kite-progress" aria-label="Onboarding progress">
      {STEPS.map((s, i) => {
        const state = i < current ? 'done' : i === current ? 'current' : 'todo';
        return (
          <li key={s.id} className={`kite-progress__step kite-progress__step--${state}`} aria-current={state === 'current' ? 'step' : undefined}>
            <span className="kite-progress__knot" aria-hidden>
              {state === 'done' ? <CheckIcon size={9} /> : state === 'current' ? <KiteGlyph /> : null}
            </span>
            <span className="kite-progress__label">{s.label}</span>
          </li>
        );
      })}
    </ol>
  );
}

function KiteGlyph() {
  return (
    <svg width="14" height="16" viewBox="0 0 14 16" aria-hidden>
      <path d="M7 0 L14 6 L7 16 L0 6 Z" fill="currentColor" />
      <path d="M7 0 V16 M0 6 H14" stroke="var(--paper-raised)" strokeWidth="0.9" opacity="0.7" />
    </svg>
  );
}

function UsernameStep({ onReadyChange, onSkip }: { onReadyChange: (ready: boolean) => void; onSkip: () => void }) {
  const [server, setServer] = useState<ChatServerConfig | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [value, setValue] = useState('');
  const [changing, setChanging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const { status, errorText, claim, reset } = useUsernameClaim(server?.id ?? null, () => {
    setChanging(false);
    setValue('');
  });

  useEffect(() => {
    const unsub = window.paperkite.onChatServersUpdated(({ servers, defaultServerId }) => {
      setServer(servers.find((s) => s.id === defaultServerId) ?? null);
      setLoaded(true);
    });
    window.paperkite.requestChatServers();
    return unsub;
  }, []);

  useEffect(() => {
    if (errorText) requestAnimationFrame(() => inputRef.current?.focus());
  }, [errorText]);

  // No default server at all (the user removed the seeded one before
  // onboarding existed) - nothing to claim against, so don't trap them.
  const claimed = !!server?.username;
  useEffect(() => onReadyChange(claimed || (loaded && !server)), [claimed, loaded, server, onReadyChange]);

  if (loaded && !server) {
    return (
      <p className="onboarding__lede">
        No chat server is configured, so there's nothing to claim a name on yet. You can add one later in Settings
        &rsaquo; Chat Servers.
      </p>
    );
  }

  return (
    <>
      <p className="onboarding__lede">
        Every page has its own chat room. This is the name other people on <strong>{server?.name ?? 'the server'}</strong>{' '}
        will see you as.
      </p>

      {claimed && !changing ? (
        <div className="onboarding-claimed">
          <span className="onboarding-claimed__check">
            <CheckIcon size={14} />
          </span>
          <div className="onboarding-claimed__text">
            <span className="onboarding-claimed__name">{server?.username}</span>
            <span className="onboarding-claimed__hint">is yours on {server?.name}</span>
          </div>
          <button type="button" className="onboarding__secondary" onClick={() => setChanging(true)}>
            Change
          </button>
        </div>
      ) : (
        <form
          className="onboarding-username"
          onSubmit={(e) => {
            e.preventDefault();
            claim(value);
          }}
        >
          <div className={'onboarding-username__field' + (errorText ? ' onboarding-username__field--error' : '')}>
            <span className="onboarding-username__at" aria-hidden>
              @
            </span>
            <input
              ref={inputRef}
              autoFocus
              value={value}
              spellCheck={false}
              autoComplete="off"
              placeholder={changing ? (server?.username ?? 'your-name') : 'your-name'}
              aria-label={changing ? 'New username' : 'Username'}
              disabled={status === 'pending' || !loaded}
              onChange={(e) => {
                setValue(e.target.value);
                reset();
              }}
              onKeyDown={(e) => {
                if (e.key === 'Escape' && changing) {
                  e.preventDefault();
                  setChanging(false);
                  setValue('');
                  reset();
                }
              }}
            />
            <button type="submit" disabled={!value.trim() || status === 'pending' || !loaded}>
              {status === 'pending' ? <span className="spinner" aria-label="Claiming" /> : 'Claim'}
            </button>
          </div>
          {errorText && (
            <p className="onboarding-username__error" role="alert">
              {errorText}
            </p>
          )}
          <p className="onboarding__note">
            {changing ? (
              <>
                You'll switch to the new name. <strong>{server?.username}</strong> stays reserved on {server?.name}, so
                nobody else can take it.{' '}
                <button
                  type="button"
                  className="onboarding__later"
                  onClick={() => {
                    setChanging(false);
                    setValue('');
                    reset();
                  }}
                >
                  Keep {server?.username}
                </button>
              </>
            ) : (
              'Each name you claim stays reserved for you on this server, so pick one you like.'
            )}
          </p>
          {status === 'error' && !changing && (
            <button type="button" className="onboarding__later" onClick={onSkip}>
              Server unreachable? Set it later in Settings
            </button>
          )}
        </form>
      )}
    </>
  );
}

function AgentsStep({ agents }: { agents: AgentConfig[] }) {
  return (
    <>
      <p className="onboarding__lede">
        Chat privately with Claude, OpenAI, Gemini or a local Ollama model from the side panel. Keys stay on this
        device.
      </p>
      {agents.length > 0 && (
        <ul className="onboarding-added">
          {agents.map((agent) => (
            <li key={agent.id}>
              <CheckIcon size={11} />
              <span className="onboarding-added__name">{agent.name}</span>
              <ProviderBadge provider={agent.provider} model={agent.model} />
            </li>
          ))}
        </ul>
      )}
      <div className="onboarding__form">
        <AgentForm editingAgent={null} onDoneEditing={() => undefined} />
      </div>
    </>
  );
}

function McpServersStep({ servers }: { servers: McpServerConfig[] }) {
  return (
    <>
      <p className="onboarding__lede">
        Give your agents tools. Each connected server's tools are offered to every agent thread, and the model decides
        when to use them.
      </p>
      {servers.length > 0 && (
        <ul className="onboarding-added">
          {servers.map((server) => (
            <li key={server.id}>
              <CheckIcon size={11} />
              <span className="onboarding-added__name">{server.name}</span>
              <span className="onboarding-added__meta">{server.transport === 'stdio' ? 'local command' : 'remote URL'}</span>
            </li>
          ))}
        </ul>
      )}
      <div className="onboarding__form">
        <McpServerForm editingServer={null} onDoneEditing={() => undefined} />
      </div>
    </>
  );
}

const ALL_SCOPES: McpScope[] = [
  'bookmarks:read',
  'bookmarks:write',
  'history:read',
  'history:write',
  'domains:read',
  'domains:write',
  'proxy:read',
  'tabs:read',
  'tabs:write',
];

type ClientId = 'claude-code' | 'claude-desktop' | 'cursor';

const CLIENTS: Array<{ id: ClientId; label: string; where: string }> = [
  { id: 'claude-code', label: 'Claude Code', where: 'Run in your terminal' },
  { id: 'claude-desktop', label: 'Claude Desktop', where: 'Add to claude_desktop_config.json' },
  { id: 'cursor', label: 'Cursor & others', where: 'Add to .cursor/mcp.json (or your client\'s MCP config)' },
];

function configFor(client: ClientId, url: string, token: string): string {
  const bearer = `Bearer ${token}`;
  if (client === 'claude-code') {
    return `claude mcp add --transport http charlotte ${url} \\\n  --header "Authorization: ${bearer}"`;
  }
  if (client === 'claude-desktop') {
    return JSON.stringify(
      {
        mcpServers: {
          charlotte: {
            command: 'npx',
            args: ['-y', 'mcp-remote', url, '--header', 'Authorization:${AUTH_HEADER}'],
            env: { AUTH_HEADER: bearer },
          },
        },
      },
      null,
      2,
    );
  }
  return JSON.stringify({ mcpServers: { charlotte: { url, headers: { Authorization: bearer } } } }, null, 2);
}

function ConnectStep() {
  const [status, setStatus] = useState<BuiltinMcpServerStatus>({ enabled: false, port: null, url: null });
  const [client, setClient] = useState<ClientId>('claude-code');
  const [jwt, setJwt] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const unsubs = [
      window.paperkite.onBuiltinMcpStatus(setStatus),
      window.paperkite.onMcpTokenCreated((result) => {
        setJwt(result.jwt);
        setCreating(false);
      }),
    ];
    window.paperkite.requestBuiltinMcpStatus();
    return () => unsubs.forEach((unsub) => unsub());
  }, []);

  const url = status.url ?? 'http://127.0.0.1:7825/mcp';
  const snippet = configFor(client, url, jwt ?? '<your-token>');
  const meta = CLIENTS.find((c) => c.id === client);

  const copy = () => {
    void navigator.clipboard.writeText(snippet);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const createToken = () => {
    setCreating(true);
    window.paperkite.createMcpToken({ label: `${meta?.label ?? 'MCP client'} (onboarding)`, scopes: ALL_SCOPES, ttlMs: null });
  };

  return (
    <>
      <p className="onboarding__lede">
        Charlotte runs its own MCP server on this machine, so tools like Claude Code can open tabs, search your
        history and manage bookmarks. Copy the config for your client.
      </p>

      {!status.enabled ? (
        <div className="onboarding-connect__off">
          <span>The local MCP server is off.</span>
          <button type="button" className="onboarding__primary" onClick={() => window.paperkite.setBuiltinMcpEnabled(true)}>
            Turn it on
          </button>
        </div>
      ) : (
        <>
          <div className="segmented onboarding-connect__clients" role="radiogroup" aria-label="MCP client">
            {CLIENTS.map((c) => (
              <button
                key={c.id}
                type="button"
                role="radio"
                aria-checked={client === c.id}
                className={'segmented__option' + (client === c.id ? ' segmented__option--active' : '')}
                onClick={() => setClient(c.id)}
              >
                {c.label}
              </button>
            ))}
          </div>

          <div className="onboarding-snippet">
            <div className="onboarding-snippet__bar">
              <span>{meta?.where}</span>
              <button type="button" onClick={copy}>
                {copied ? 'Copied' : 'Copy'}
              </button>
            </div>
            <pre>{snippet}</pre>
          </div>

          {jwt ? (
            <p className="onboarding__note onboarding__note--sky">
              Token filled in above. Copy it now, it won't be shown again. Manage or revoke it in Settings &rsaquo; MCP.
            </p>
          ) : (
            <div className="onboarding-connect__token">
              <span>
                Clients need a token. Create one with full access, or pick narrower scopes later in Settings &rsaquo;
                MCP.
              </span>
              <button type="button" className="onboarding__secondary" disabled={creating} onClick={createToken}>
                {creating ? <span className="spinner" aria-label="Creating" /> : 'Create token'}
              </button>
            </div>
          )}
        </>
      )}
    </>
  );
}
