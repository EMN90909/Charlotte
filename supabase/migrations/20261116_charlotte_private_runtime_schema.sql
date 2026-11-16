-- Charlotte-owned cloud state is kept in its own schema.
-- Never write plaintext OAuth tokens, API keys, passwords, cookies or browser session data here.
CREATE SCHEMA IF NOT EXISTS charlotte;
REVOKE ALL ON SCHEMA charlotte FROM PUBLIC, anon, authenticated;
GRANT USAGE ON SCHEMA charlotte TO service_role;

CREATE TABLE IF NOT EXISTS charlotte.mcp_connections (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  provider_id text NOT NULL CHECK (provider_id ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
  display_name text NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 120),
  status text NOT NULL DEFAULT 'connected' CHECK (status IN ('pending','connected','reauth_required','disabled')),
  scopes text[] NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (user_id, provider_id)
);
CREATE INDEX IF NOT EXISTS mcp_connections_user_updated ON charlotte.mcp_connections(user_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS charlotte.mcp_credentials (
  connection_id uuid PRIMARY KEY REFERENCES charlotte.mcp_connections(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  ciphertext bytea NOT NULL CHECK (octet_length(ciphertext) BETWEEN 16 AND 65536),
  nonce bytea NOT NULL CHECK (octet_length(nonce) = 12),
  key_version smallint NOT NULL DEFAULT 1 CHECK (key_version > 0),
  expires_at timestamptz,
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS mcp_credentials_user ON charlotte.mcp_credentials(user_id);

CREATE TABLE IF NOT EXISTS charlotte.agent_tasks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  status text NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued','running','paused','awaiting_approval','completed','failed','cancelled')),
  goal text NOT NULL CHECK (char_length(goal) BETWEEN 1 AND 4000),
  checkpoint jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (octet_length(checkpoint::text) <= 65536),
  step integer NOT NULL DEFAULT 0 CHECK (step >= 0),
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS agent_tasks_user_updated ON charlotte.agent_tasks(user_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS charlotte.agent_memory (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  memory text NOT NULL CHECK (char_length(memory) BETWEEN 1 AND 4000),
  source text NOT NULL DEFAULT 'user' CHECK (source IN ('user','approved_summary')),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS agent_memory_user_updated ON charlotte.agent_memory(user_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS charlotte.browser_preferences (
  user_id uuid PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
  theme text NOT NULL DEFAULT 'monochrome' CHECK (theme IN ('monochrome','gradient')),
  gradient_start text NOT NULL DEFAULT '#171717' CHECK (gradient_start ~ '^#[0-9A-Fa-f]{6}$'),
  gradient_end text NOT NULL DEFAULT '#303030' CHECK (gradient_end ~ '^#[0-9A-Fa-f]{6}$'),
  tab_layout text NOT NULL DEFAULT 'horizontal' CHECK (tab_layout IN ('horizontal','vertical')),
  updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE charlotte.mcp_connections ENABLE ROW LEVEL SECURITY;
ALTER TABLE charlotte.mcp_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE charlotte.agent_tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE charlotte.agent_memory ENABLE ROW LEVEL SECURITY;
ALTER TABLE charlotte.browser_preferences ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS service_role_only ON charlotte.mcp_connections;
CREATE POLICY service_role_only ON charlotte.mcp_connections TO service_role USING (true) WITH CHECK (true);
DROP POLICY IF EXISTS service_role_only ON charlotte.mcp_credentials;
CREATE POLICY service_role_only ON charlotte.mcp_credentials TO service_role USING (true) WITH CHECK (true);
DROP POLICY IF EXISTS service_role_only ON charlotte.agent_tasks;
CREATE POLICY service_role_only ON charlotte.agent_tasks TO service_role USING (true) WITH CHECK (true);
DROP POLICY IF EXISTS service_role_only ON charlotte.agent_memory;
CREATE POLICY service_role_only ON charlotte.agent_memory TO service_role USING (true) WITH CHECK (true);
DROP POLICY IF EXISTS service_role_only ON charlotte.browser_preferences;
CREATE POLICY service_role_only ON charlotte.browser_preferences TO service_role USING (true) WITH CHECK (true);

GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA charlotte TO service_role;
