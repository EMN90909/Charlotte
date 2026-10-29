# Charlotte Auto — native capability orchestration

**Source integration:** PR #6 into main (native verification pending).
**Commercial target:** a desktop browser where each user connects a chosen AI
provider and separately authorizes their own MCP services. Charlotte's Rust
host decides which *permitted* browser/service facility will execute the task.
The AI suggests routes; it does not gain direct CEF, system-shell, credential,
database-write or arbitrary third-party API capabilities.

## Architecture

```text
User in Charlotte sidebar
          |
          v
  Provider selection / OS keychain
          |
          v
AI CAPABILITY ROUTER (one model call)
  Input: user request + last four relevant chat turns +
         active tab ORIGIN only + configured MCP profile IDs
  Output: exactly one validated JSON route
          |
    Rust policy validation
          |
   +------+---------+-------------+--------------+
   |                |             |              |
  Reply           Local         Chromium          MCP
 (no tool)        skill         capability      capability
                  (read only)   + page consent  + user-configured service
                    |             |              |
                    |         read/act          tools/list
                    |           |               |
                    |        per-action        per-tool
                    |         approval         approval
                    |           |               |
                    +------- verify/inspect -----+
                               |
                          display outcome
```

A route cannot create a configured MCP service, connect an unrelated app,
choose an untrusted executable or bypass the current CEF tab's site origin.
The router's choices are checked in `src/orchestrator.rs` before being
executed by `src/ui_server.rs`.

## Supported routes

| Route | Example user task | Execution |
|---|---|---|
| `reply` | "Explain recursion" | The selected AI answers without DOM or MCP |
| `clarify` | "Create a Notion page" but Notion is disabled | Ask the user to configure the necessary service |
| `local_skill` | "List the links in this page" | One of four bounded Rust skills; page text stays local |
| `page_answer` | "Explain the document open in my tab" | Ask to share page text, inspect read-only CEF DOM, then ask selected AI |
| `browser_task` | "Open the settings menu on this tab" | Ask for page-reading consent; bounded observe → model → approve each action → verify |
| `mcp_task` | "Create an issue in my connected GitHub repo" | Only that enabled MCP; discover advertised tools, model proposes one and requests approval |
| `page_to_mcp` | "Use this open report to draft a Notion page" | Consent to read page, feed bounded untrusted page data to AI, show the exact proposed MCP action for separate approval |
| `browser_then_mcp` | "Open the site's report, then send its findings to Notion" | Approved Chromium interactions first; stop at a second page-sharing consent; then an MCP tool approval |

The model must return a well-formed JSON envelope with one of the above route
names. A route requiring a tab fails if there is no eligible active HTTPS or
loopback tab. A route requiring an MCP server fails unless its ID is explicitly
enabled. A model cannot supply arbitrary CDP method names, executable code,
operating-system paths or invented vendor credentials.

## User-configured AI providers

The sidebar provides a signed AI provider configuration form for:
OpenAI, Claude, Gemini and local Ollama. Users can enter an API key and an
optional compatible model name, save locally, remove a saved key, and
**explicitly test the saved model** with one small request (may incur charges).
The key is stored in the user's operating-system credential store, not SQLite.
Model identifiers are validated before storage and loaded across chat,
automation planning, MCP summaries and Auto.

The user chooses the AI provider. **Auto chooses what capability to use, not
which paid AI vendor account to bill.** It does not establish subscriptions,
quotas, invoice handling or provider-specific enterprise account management.
Ollama requires a separately installed/running local model server.

## Human permissions and data separation

1. **Routing** sends the user question, the active tab **origin** and the IDs
   of configured MCP integrations to the selected AI. It does **not** read the
   current DOM, screenshot, browser cookies, form values or OS credentials.
2. **Local skills** may then inspect the active CEF tab without uploading the
   resulting page text to the AI model or saving it in AI chat history.
3. **Page reading for AI**, including a Chromium action or a page-to-app
   handoff, requires an additional explicit page-sharing approval card in
   Auto mode. The exact source browser/tab/URL are pinned at approval.
4. **Chromium writes** are individually reviewed and approved, limited to one
   user-authorized top-level origin, live-target verified, and bounded to eight
   actions per task. Vision screenshots remain a separate explicit mode.
5. **MCP tool calls** require another per-tool approval. Tools are dynamically
   discovered from the selected configured server and classified for
   read-only, modifying or financial risk; the exact server/tool/risk are
   rechecked before the call.
6. **Cross-app handoff** sends no tool request until the user approves that
   exact MCP action. If the source Chromium tab or URL has changed since the
   handoff was proposed, the approval is invalidated.
7. **Browser-first handoffs** stop after the browser phase and require
   **another page-sharing consent**, followed by normal MCP approval.
   Reaching the browser's step limit never automatically starts MCP.
8. **Recovery** never replays a potentially completed write after tab switch,
   renderer crash, task pause or restart. Saved browser checkpoints require
   reinspection, and previous Auto handoff intent is discarded on pause.

Page data and MCP results are untrusted; only bounded excerpts are passed to
the chosen model, with explicit instruction not to follow commands embedded
in third-party content. AI instructions alone are **not** a guarantee against
prompt injection: Rust origin, target, URL, risk and approval checks are the
enforcement layer.

## Example workflow

User: "Find the release notes on the current project page, then prepare a
GitHub issue describing the breaking changes."

Charlotte Auto can select `browser_then_mcp` for the enabled GitHub MCP
profile. The user authorizes page reading; Charlotte's Chromium agent opens
the appropriate site controls with per-step approvals. When the browser
agent stops, the user reviews the page and consents to a second, current
page read. The selected AI receives bounded evidence as untrusted data and
proposes an actual advertised GitHub MCP tool invocation. Charlotte displays
the exact arguments and risk label. Only after the user approves can Rust
execute the call. If the GitHub connection is unavailable or the page
changes, the transfer stops.

## Commercial readiness limitations

This is **desktop-profile-scoped source implementation**, not yet a hosted
multi-tenant enterprise service. Production distribution still requires:
- Successful pinned CEF/Rust build and end-to-end tests on Windows/macOS/Linux.
- OAuth and model-API provider compatibility testing, including token renewal.
- Explicit account and tenant isolation/security audits if Charlotte later
  offers cloud-hosted accounts or organization-shared sessions.
- Abuse prevention, request/usage quotas and consumer-facing billing disclosure.
- Verified task trace/telemetry with opt-in privacy and user data deletion.
- Native user acceptance tests for crashes, page changes, private tabs, prompt
  injection, malicious MCP servers, payment actions and interrupted writes.
- Better support for multi-tool dependency graphs, streaming and complex
  cross-origin frame automation.

No claims are made that the implementation successfully compiled, executed
a real MCP tool or completed an OAuth session: current GitHub Actions jobs
have failed before running steps (see #7). Draft PR #6 remains unmerged.

## Code references

- `src/orchestrator.rs` — strict route schema and host permission checks
- `src/ui_server.rs` — Auto workflow, provider settings, consent cards,
  browser/MCP execution, source validation and approval controls
- `src/browser_agent.rs` / `src/browser_automation.rs` — Chromium loop
- `src/mcp.rs`, `src/mcp_stdio.rs`, `src/mcp_oauth.rs` — service transports
- `src/browser_skills.rs` — deterministic, non-model page reads
- `MCP_CATALOG.md` and `AGENTIC_BROWSER_RESEARCH.md`
