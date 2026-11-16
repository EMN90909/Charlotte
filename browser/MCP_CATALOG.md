# Charlotte MCP — 52-profile connection directory

Source: *Charlotte MCP.pdf*, eight pages, provided 29 October 2026.
It describes 33 ranked integration categories; Charlotte has 52 individually
addressable profiles because several categories have multiple alternatives.

**Important:** A profile is a connection template, not an active account
connection. Vendor URLs and packages came from the PDF and are **not
guaranteed to remain current**. A catalog entry does not guarantee that the
provider offers free API usage, full tool access, compatible OAuth, or
unlimited queries. Published claims of '100% free' and popularity rankings
in the PDF were not independently audited.

## Google Workspace profiles

The catalog includes individual Gmail, Drive, Calendar, Docs, Sheets, Slides, Chat and Classroom endpoints published by Google. Google labels these MCP servers Developer Preview. Each requires a configured OAuth client; the browser does not claim an account is connected merely because its profile is listed.

YouTube is listed as a custom MCP entry without a guessed server URL. Supply and verify a server endpoint you control or trust. A ChatGPT subscription is not an OpenAI API key and is not listed as a direct account connector; Charlotte's AI provider/API configuration is a separate integration.

## Implemented native runtime

- Existing **Rust Streamable HTTP** JSON-RPC client supports HTTPS requests,
  MCP sessions and SSE/JSON replies, bearer tokens and response size limits.
- **User-initiated OAuth 2.1 authorization** for remote services with MCP
  protected-resource metadata, authorization-server discovery, PKCE **S256**,
  and either dynamic client registration (DCR) or a manually registered public
  client ID. The user opens the printed authorization URL and explicitly
  signs in; tokens are stored in the **OS credential store**, never SQLite.
  OAuth is provider-dependent; this is not a universal working sign-in claim.
- The allowlisted local `npx`/`uvx` commands run in **offline,
  no-install mode**. Missing packages cause a connection error; Charlotte
  does not silently download them or execute a shell pipeline. Install or
  vet package versions yourself before choosing to enable them.
- **Local STDIO MCP** support for allowlisted presets using a directly
  launched program (no shell string); JSON-RPC messages over newline-separated
  UTF-8 stdin/stdout, tool discovery and tool calls. The user must separately
  install the named package/runtime. Charlotte disables silent npx
  installation and never auto-installs npm/Python packages.
- A **native, read-only SQLite MCP-compatible adapter** opens one approved
  database in `SQLITE_OPEN_READ_ONLY` mode. It exposes table schema and
  bounded `SELECT`/`WITH` queries (100 rows/24 columns max). No write SQL,
  arbitrary file path from the model, or unapproved database access.
- Filesystem MCP requires an explicitly configured **canonical directory
  root**. Only tools scoped by that server's capabilities are exposed;
  all calls remain subject to approval. Do not point it at a home directory
  or broad drive unless you intend to expose that material.
- Up to **48 saved profiles** (rather than 8). Chat requests connect only
  a small **prompt-matched subset of up to 5**. Model tool context uses
  a bounded round-robin across relevant server groups and excludes schemas
  larger than 6 KB. Runtime still verifies every exact advertised tool.
- **Stable tool names** derived from each saved server ID, not connection
  order, to avoid misrouting approvals when active profiles differ between
  planning and execution.
- MCP tools are conservatively classified as **Read-only**, **Modifying**,
  or **Financial** for the approval card. Unknown operations default to
  Modifying; Stripe and PayPal default to Financial. The displayed risk
  must still match after the server reconnects before executing.
- **Every MCP tool call requires a signed, explicit approval** in Charlotte's
  sidebar; no read-only exceptions are silently executed. The approved
  server and advertised tool are re-verified just before invocation.
- Custom HTTPS MCP endpoints are accepted for vendors without one fixed
  endpoint, from the signed sidebar or settings CLI. No endpoint is guessed
  for Canva, Miro or a merchant's Shopify store.

## Settings commands

List available integrations and opt into a catalog profile:

```sh
charlotte-settings mcp catalog
charlotte-settings mcp enable github
charlotte-settings mcp servers
```

For a service that accepts a bearer token or a fine-grained personal token:

```sh
charlotte-settings mcp token set github
charlotte-settings mcp verify github
```

For a vendor that supports MCP OAuth 2.1 discovery/PKCE and DCR:

```sh
charlotte-settings mcp enable notion
charlotte-settings mcp oauth authorize notion
charlotte-settings mcp verify notion
```

The authorize command prints a consent URL; **the user must open it and
approve access**. A local loopback callback accepts the authorization code
for up to three minutes. If the vendor does not allow DCR, first register
a public OAuth client with that vendor, then:

```sh
charlotte-settings mcp oauth client-id set notion
charlotte-settings mcp oauth authorize notion
```

Authorization scopes, provider-specific registration, access-token renewal
and the availability of PKCE/DCR vary. The current implementation stores
refresh tokens when supplied, but **does not yet automatically refresh
expired access tokens**. Re-authorization may be necessary.

For a pre-installed local MCP process (explicit opt-in; no Node.js server
needed for Charlotte itself):

```sh
charlotte-settings mcp enable memory
charlotte-settings mcp verify memory
```

For local directory access (install the vetted filesystem package separately
before verification):

```sh
charlotte-settings mcp filesystem root set /your/approved/folder
charlotte-settings mcp enable filesystem
charlotte-settings mcp verify filesystem
```

For approved read-only SQLite:

```sh
charlotte-settings mcp sqlite file set /your/approved/database.sqlite
charlotte-settings mcp enable sqlite
charlotte-settings mcp verify sqlite
```

For a workspace-specific vendor endpoint, obtain the real URL from the
service's documentation rather than substituting an unverified URL:

```sh
charlotte-settings mcp add canva https://your-real-vendor-mcp-endpoint
charlotte-settings mcp verify canva
```

Disable a profile without altering third-party accounts:

```sh
charlotte-settings mcp disable github
charlotte-settings mcp token remove github
```

Users must revoke external OAuth app access separately in the vendor's
security dashboard to fully revoke a grant.

## Integration directory

| Profile ID | Service | Transport | Sign-in |
|---|---|---|---|
| `github` | GitHub | Remote HTTPS | oauth-or-token |
| `google-workspace` | Google Workspace | Local STDIO | oauth-local |
| `notion` | Notion | Remote HTTPS | oauth |
| `slack` | Slack | Remote HTTPS | oauth |
| `atlassian` | Atlassian Jira & Confluence | Remote HTTPS | oauth |
| `linear` | Linear | Remote HTTPS | oauth |
| `linear-readonly` | Linear (read-only) | Remote HTTPS | oauth |
| `stripe` | Stripe | Remote HTTPS | oauth-or-token |
| `supabase` | Supabase (read-only) | Remote HTTPS | oauth-or-token |
| `figma` | Figma | Remote HTTPS | oauth |
| `playwright` | Playwright | Local STDIO | local |
| `hubspot` | HubSpot | Remote HTTPS | oauth |
| `sentry` | Sentry | Remote HTTPS | oauth |
| `asana` | Asana | Remote HTTPS | oauth |
| `context7` | Context7 | Remote HTTPS | optional-token |
| `cloudflare` | Cloudflare | Remote HTTPS | oauth-or-token |
| `vercel` | Vercel | Remote HTTPS | oauth |
| `gitlab` | GitLab | Remote HTTPS | oauth-or-token |
| `postgres` | PostgreSQL tools | Local STDIO | local |
| `sqlite` | SQLite tools | Native SQLite | local-file |
| `zapier` | Zapier | Remote HTTPS | oauth |
| `exa` | Exa | Remote HTTPS | optional-token |
| `firecrawl` | Firecrawl | Remote HTTPS | optional-token |
| `brave-search` | Brave Search | Local STDIO | local |
| `canva` | Canva | Manual endpoint | oauth-manual |
| `miro` | Miro | Manual endpoint | oauth-manual |
| `monday` | Monday.com | Remote HTTPS | oauth-or-token |
| `shopify-dev` | Shopify developer | Local STDIO | local |
| `shopify-store` | Shopify Store UCP | Manual endpoint | merchant-url |
| `x-docs` | X API documentation | Remote HTTPS | none |
| `x-api` | X API | Remote HTTPS | oauth |
| `paypal` | PayPal | Remote HTTPS | oauth |
| `granola` | Granola | Remote HTTPS | oauth |
| `amplitude` | Amplitude | Remote HTTPS | oauth |
| `clay` | Clay | Remote HTTPS | oauth |
| `box` | Box | Remote HTTPS | oauth |
| `grafana` | Grafana | Local STDIO | local |
| `huggingface` | Hugging Face | Remote HTTPS | oauth-or-token |
| `deepwiki` | DeepWiki | Remote HTTPS | none |
| `memory` | MCP Memory | Local STDIO | local |
| `filesystem` | MCP Filesystem | Restricted STDIO | local |
| `sequential-thinking` | MCP Sequential Thinking | Local STDIO | local |
| `git` | MCP Git | Local STDIO | local |

## Known limitations / release gates

1. GitHub CI provisioning failures currently prevent Rust/CEF build
   verification. These source commits are **not proof of working connections**.
2. OAuth-only vendors that reject DCR or a manually registered public client
   may need a vendor-specific certified application registration and
   associated consent experience. Existing protocol support cannot make
   an unavailable third-party application grant work.
3. Native CEF popup-based OAuth initiation has **not** been implemented:
   authorization is user-initiated through the CLI, using a URL opened in
   a browser, with a loopback callback.
4. Some catalog endpoints may have changed since the PDF was compiled.
   Use the verified vendor-supplied URL when connecting.
5. Third-party local tools can access your local environment and may need
   extra credentials; install and enable only software you trust.
6. MCP refresh-token rotation, schema pagination, streaming requests,
   server-sent notifications and cross-server transactional orchestration
   require more work.
7. Native browser automation through CEF remains separate from optional
   Playwright MCP, which would run an external browser runtime and may not
   inherit Charlotte's currently authenticated CEF tab.
8. Security approval for external payment, destructive, or financial tool
   actions must continue to require explicit confirmation; no guarantee of
   vendor-side idempotency is made.

Protocol references:
- https://modelcontextprotocol.io/specification/
- https://github.com/modelcontextprotocol/modelcontextprotocol
- https://chromedevtools.github.io/devtools-protocol/

The integration directory is not affiliated with Strawberry Browser.
