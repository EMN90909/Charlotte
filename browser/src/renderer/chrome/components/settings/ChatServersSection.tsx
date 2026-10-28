import { useEffect, useState } from 'react';
import type { ChatServerConfig, AddChatServerPayload } from '../../../../shared/types';
import { EditIcon, TrashIcon, StarIcon, CheckIcon } from '../../icons';
import { useUsernameClaim } from '../../useUsernameClaim';

interface ChatServersSectionProps {
  /** Set when Settings was opened via the chat panel's "fix this server's
   * username" CTA - see SettingsModal.tsx's own doc comment on the same
   * props. `null` for a normal visit to this section. */
  focusServerId: string | null;
  focusToken: number;
}

export function ChatServersSection({ focusServerId, focusToken }: ChatServersSectionProps) {
  const [servers, setServers] = useState<ChatServerConfig[]>([]);
  const [defaultServerId, setDefaultServerId] = useState<string | null>(null);
  const [editingServer, setEditingServer] = useState<ChatServerConfig | null>(null);

  useEffect(() => {
    const unsub = window.paperkite.onChatServersUpdated((payload) => {
      setServers(payload.servers);
      setDefaultServerId(payload.defaultServerId);
    });
    window.paperkite.requestChatServers();
    return unsub;
  }, []);

  // Keep the open edit form in sync with the live list - e.g. once a
  // claim succeeds, `editingServer` (a snapshot from when Edit was
  // clicked) would otherwise still show the old, unset username.
  useEffect(() => {
    if (!editingServer) return;
    const fresh = servers.find((s) => s.id === editingServer.id);
    if (fresh && fresh !== editingServer) setEditingServer(fresh);
  }, [servers, editingServer]);

  // Depends on `servers` too, not just focusToken - if the CTA fires
  // before the server list has actually loaded yet, this re-checks once
  // it does instead of silently missing the deep-link.
  useEffect(() => {
    if (!focusServerId || focusToken === 0) return;
    const target = servers.find((s) => s.id === focusServerId);
    if (target) setEditingServer(target);
  }, [focusServerId, focusToken, servers]);

  return (
    <>
      <section className="settings-section">
        <h2>Chat servers</h2>
        <p className="settings-hint">
          Charlotte's page chat is a client of an external chat server (see chat-service) - your copy of the browser
          connects to whichever one's picked here. The default is what every tab joins unless it's switched from the
          picker in the chat panel's own header. Your username is set per server, not globally - each one may enforce
          its own uniqueness, so the same name isn't guaranteed to be free everywhere. You can switch to a new name any
          time, but every name you've claimed stays reserved for you on that server.
        </p>

        {servers.length === 0 ? (
          <p className="settings-hint">No chat servers configured yet.</p>
        ) : (
          <ul className="settings-list">
            {servers.map((server) => (
              <li key={server.id} className="settings-list__row">
                <div className="settings-list__main">
                  <span className="settings-list__title">{server.name}</span>
                  <span className="settings-list__url">{server.baseUrl}</span>
                  <span className={'settings-list__username' + (server.username ? '' : ' settings-list__username--unset')}>
                    {server.username ?? 'No username set'}
                  </span>
                </div>
                <button
                  type="button"
                  className={'settings-list__delete' + (server.id === defaultServerId ? ' settings-list__delete--active' : '')}
                  aria-label={server.id === defaultServerId ? `${server.name} is the default` : `Make ${server.name} the default`}
                  title={server.id === defaultServerId ? 'Default server' : 'Set as default'}
                  disabled={server.id === defaultServerId}
                  onClick={() => window.paperkite.setDefaultChatServer(server.id)}
                >
                  <StarIcon filled={server.id === defaultServerId} />
                </button>
                <button
                  type="button"
                  className="settings-list__delete"
                  aria-label={`Edit ${server.name}`}
                  onClick={() => setEditingServer(server)}
                >
                  <EditIcon />
                </button>
                <button
                  type="button"
                  className="settings-list__delete"
                  aria-label={`Remove ${server.name}`}
                  onClick={() => {
                    if (editingServer?.id === server.id) setEditingServer(null);
                    window.paperkite.deleteChatServer(server.id);
                  }}
                >
                  <TrashIcon />
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <ChatServerForm servers={servers} editingServer={editingServer} onDoneEditing={() => setEditingServer(null)} />
    </>
  );
}

interface FormFields {
  name: string;
  baseUrl: string;
}

function blankFields(): FormFields {
  return { name: '', baseUrl: '' };
}

function normalizeUrlForCompare(url: string): string {
  return url.trim().replace(/\/+$/, '').toLowerCase();
}

interface ChatServerFormProps {
  servers: ChatServerConfig[];
  editingServer: ChatServerConfig | null;
  onDoneEditing: () => void;
}

function ChatServerForm({ servers, editingServer, onDoneEditing }: ChatServerFormProps) {
  const [fields, setFields] = useState<FormFields>(blankFields());

  useEffect(() => {
    setFields(editingServer ? { name: editingServer.name, baseUrl: editingServer.baseUrl } : blankFields());
  }, [editingServer]);

  const isDuplicateUrl =
    fields.baseUrl.trim().length > 0 &&
    servers.some((s) => s.id !== editingServer?.id && normalizeUrlForCompare(s.baseUrl) === normalizeUrlForCompare(fields.baseUrl));

  const canSave = fields.name.trim().length > 0 && fields.baseUrl.trim().length > 0 && !isDuplicateUrl;

  const buildDraft = (): AddChatServerPayload => ({
    name: fields.name.trim(),
    baseUrl: fields.baseUrl.trim(),
  });

  const submit = () => {
    if (!canSave) return;
    if (editingServer) {
      window.paperkite.updateChatServer({ id: editingServer.id, ...buildDraft() });
    } else {
      window.paperkite.createChatServer(buildDraft());
    }
    setFields(blankFields());
    onDoneEditing();
  };

  return (
    <section className="settings-section">
      <h2>{editingServer ? `Edit "${editingServer.name}"` : 'Add chat server'}</h2>

      <form
        className="proxy-form"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <label className="proxy-form__field">
          <span>Name</span>
          <input value={fields.name} spellCheck={false} placeholder="Local" onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))} />
        </label>

        <label className="proxy-form__field">
          <span>URL</span>
          <input
            value={fields.baseUrl}
            spellCheck={false}
            placeholder="http://localhost:8080"
            onChange={(e) => setFields((f) => ({ ...f, baseUrl: e.target.value }))}
          />
        </label>
        {isDuplicateUrl && <p className="username-claim__error proxy-form__field--full">A server with this URL is already configured.</p>}

        {editingServer && (
          <div className="proxy-form__field--full">
            <ChatServerUsernameField server={editingServer} />
          </div>
        )}

        <button type="submit" className="proxy-form__save" disabled={!canSave}>
          {editingServer ? 'Save changes' : 'Add server'}
        </button>
        {editingServer && (
          <button type="button" className="settings-cancel-edit" onClick={onDoneEditing}>
            Cancel
          </button>
        )}
      </form>
    </section>
  );
}

/** The username claim itself is server-owned, not just a form field this
 * app persists (see chatServerStore.ts's setChatServerUsername) - Claim
 * actually asks the real server whether the name is free, the same POST
 * /connect claim a real page-chat connect would make. Changing it later is
 * just claiming another name; the old one stays reserved on the server. */
function ChatServerUsernameField({ server }: { server: ChatServerConfig }) {
  const [value, setValue] = useState('');
  const [changing, setChanging] = useState(false);
  const { status, errorText, claim, reset } = useUsernameClaim(server.id, () => {
    setChanging(false);
    setValue('');
  });

  const cancelChange = () => {
    setChanging(false);
    setValue('');
    reset();
  };

  if (server.username && !changing) {
    return (
      <div className="proxy-form__field proxy-form__field--full">
        <span>Your username on this server</span>
        <div className="username-claim__row">
          <input value={server.username} disabled readOnly aria-label="Username" />
          <span className="username-claim__check username-claim__check--set" aria-label="Claimed">
            <CheckIcon size={13} />
          </span>
          <button type="button" className="settings-cancel-edit username-claim__action" onClick={() => setChanging(true)}>
            Change
          </button>
        </div>
      </div>
    );
  }

  // A plain div, not a <label>/<form> - this sits inside ChatServerForm's
  // own <form>, so Enter is handled here rather than submitting that.
  return (
    <div className="proxy-form__field proxy-form__field--full">
      <span>{changing ? 'New username' : 'Your username on this server'}</span>
      <div className="username-claim__row">
        <input
          autoFocus={changing}
          value={value}
          spellCheck={false}
          aria-label={changing ? 'New username' : 'Username'}
          placeholder={changing ? (server.username ?? '') : 'Not set'}
          disabled={status === 'pending'}
          onChange={(e) => {
            setValue(e.target.value);
            reset();
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              claim(value);
            } else if (e.key === 'Escape' && changing) {
              e.preventDefault();
              cancelChange();
            }
          }}
        />
        <button
          type="button"
          className="proxy-form__save username-claim__action"
          disabled={!value.trim() || status === 'pending'}
          onClick={() => claim(value)}
        >
          {status === 'pending' ? <span className="spinner" aria-label="Checking availability" /> : 'Claim'}
        </button>
        {changing && (
          <button type="button" className="settings-cancel-edit username-claim__action" onClick={cancelChange}>
            Cancel
          </button>
        )}
      </div>
      {errorText && <p className="username-claim__error">{errorText}</p>}
      <p className="settings-hint">
        {changing
          ? `"${server.username}" stays reserved for you on this server after you switch, so nobody else can take it.`
          : 'Each name you claim stays reserved on this server - if it says taken, pick a different one.'}
      </p>
    </div>
  );
}
