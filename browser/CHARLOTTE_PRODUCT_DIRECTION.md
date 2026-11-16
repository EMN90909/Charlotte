# Charlotte product direction and agent research

Updated 16 November 2026. Charlotte is a personal-use browser project, with Linux as the primary development and runtime target.

## Research findings

Perplexity Comet's core interaction is a conventional browser with an assistant attached to the active page and search flow. Reviews generally found that the integration reduces context switching and is familiar, while autonomous multi-step actions can still be inconsistent and need user review. OpenAI's Operator research preview used a dedicated browser view, exposed progress, and handed control to the user around credentials, payments and other sensitive steps; OpenAI later folded the experience into ChatGPT agent.

Strawberry's current product emphasizes a visible step trail, resumable long tasks, connected apps with per-action permissions, user approval before consequential actions, and memory that users can inspect and manage. These are useful interaction principles for Charlotte, not a request to copy another product's branding or exact interface.

Product and independent review sources surfaced recurring friction: agents can lose their place, misunderstand a task, or need the user to intervene; a reviewable progress trail and explicit handoff are more useful than pretending every workflow is fully autonomous.

## Charlotte interface

- Keep normal browsing first: monochrome surfaces, compact browser chrome, crisp typography, and slightly curved box-shaped controls.
- Avoid oversized pill buttons and excessive rounded cards.
- Keep tabs horizontal by default, with a user-selectable vertical rail inspired by editor sidebars.
- Let users opt into accent gradients in appearance settings; monochrome remains the default.
- Show the task goal, current step, relevant sources, approvals requested, and a concise completion summary in the assistant panel.
- Keep takeover, pause, resume, stop, and approval visible. Never perform external writes, sending, purchases, or destructive actions without a specific approval.

The current browser CSS was moved to a monochrome palette and compact corner radius. Cloud-backed appearance preferences are represented in the Charlotte schema; settings UI still needs to be wired to those fields.

## Agent workflow and memory

Use a durable state machine: queued → running → paused/awaiting approval → completed, failed or cancelled. Before every step, verify that the user still owns the task, the active browser context is unchanged, and the requested capability is still authorized. Persist progress summaries and safe checkpoints, then resume from the last verified step after restart. Never persist cookies, passwords, raw page screenshots, or form values in task state.

Treat every webpage, email, document, MCP response and tool description as untrusted data. Keep it separate from trusted system instructions, use least-privilege tool scopes, and require fresh user approval before consequential actions. Prompt-injection defenses are layered risk reduction, not a promise that prompt injection can be eliminated.

Long-term memory should be opt-in, editable and deletable. Save user preferences or an approved task summary, not a silent transcript of all browsing. Do not use private-browsing activity for memory.

## Target runtime

- Local everyday browsing: Rust + WebKitGTK; local history and browser state in SQLite.
- Hosted coordination: a Rust API on Render for authenticated AI requests, quota-aware provider fallback, and encrypted cloud task checkpoints.
- On-demand cloud browsing: launch an isolated E2B browser only after a user request; do not run a permanent remote browser session.
- Cloud credentials: encrypt OAuth/API credential blobs in the service before storing them in the dedicated Supabase `charlotte` schema. The encryption key, Supabase service credentials, and AI provider keys belong only in Render's secret environment. The desktop receives short-lived user-scoped tokens, never service-role keys.
- Keep local browsing data on device; sync only explicitly selected task state.

## Known implementation gap

The main branch still uses CEF. It does not yet contain the WebKitGTK replacement, a Render AI orchestration service, E2B task provisioning, provider quota/fallback routing, social OAuth sign-in, or controls that use cloud appearance preferences. The open draft PR #14 adds a separate hosted Node gateway proposal but has not been merged or verified as production ready. Do not describe this target architecture as shipped.

## Sources

- Perplexity Comet overview: https://www.perplexity.ai/hub/blog/introducing-comet
- OpenAI Operator and handoff principles: https://openai.com/index/introducing-operator/
- Strawberry agent overview and app permissions: https://strawberrybrowser.com/browser-agent
- Strawberry prompt-injection model: https://strawberrybrowser.com/what-is-prompt-injection
- Strawberry 1.0 product updates: https://strawberrybrowser.com/updates/strawberry-1-0
- Independent Strawberry hands-on review: https://benchtested.app/apps/strawberry/
- TechCrunch Operator usability report: https://techcrunch.com/2025/02/04/openais-operator-agent-helped-me-move-but-i-had-to-help-it-too/
- Google Workspace MCP endpoint directory: https://docs.cloud.google.com/mcp/supported-products
- Google Workspace MCP setup and OAuth: https://developers.google.com/workspace/guides/configure-mcp-servers
- OpenAI API MCP guide (API credentials are distinct from a ChatGPT account): https://developers.openai.com/api/docs/guides/tools-connectors-mcp
