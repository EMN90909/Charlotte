# Charlotte AI — BYOK + hosted cloud gateway

This implementation adds a **new independent `services/` gateway** next to
the existing Rust `server/` and Rust/CEF `browser/` modules; no existing
server routes need to be replaced. The gateway uses built-in Node.js 22 APIs,
so it has no runtime npm dependencies.

## Two separate AI connection modes

**BYOK (bring your own key):** The user selects a provider in Charlotte's
native sidebar (OpenAI, Anthropic, Gemini, xAI, Mistral, DeepSeek, Groq,
Cohere, OpenRouter, Together, Fireworks, Cerebras or Perplexity). A user's
secret is stored using the desktop OS credential store via Rust `keyring`.
The user selects a provider-specific model ID, optionally using the curated
model hints; requests originate directly from native Rust. Keys **never**
enter the CEF webpage DOM, SQLite profile or cloud gateway.

**Charlotte Cloud:** The user signs into their Charlotte account with
the existing Supabase identity flow, saves their service's HTTPS URL and
chooses **Charlotte Cloud** as the AI source. The native browser sends the
user's freshly refreshed Supabase access token to the service in the
Authorization header; the gateway verifies it at the configured Supabase
`/auth/v1/user` endpoint. The gateway then uses its own server-side
credentials from host environment secrets. No provider key is sent to the
desktop. Cloud model choices are the provider/model IDs returned by its
authenticated `GET /v1/models` endpoint. `auto` randomly selects an enabled
model (from those appropriate to the text/vision modality).

**Model routing is not user authentication**: Cloud requests are subject to
account auth, per-user limits and the configured provider pool. It is the
account operator's responsibility to configure quotas and billing before
offering metered usage. The in-memory rate limiter in this initial
implementation is **single-instance only**; production scaling requires
a centralized quota/rate-limit store and abuse monitoring.

## Provider/model directory

`services/src/catalog.mjs` includes **13 API providers and more than
30 suggested chat models**, verified against published vendor documentation
when this PR was authored (29 October 2026). This is **not an exhaustive
list of all models**. Model lifecycles, access tiers, chat vs Responses API,
vision capability, rate limits and pricing change. Some examples require a
specific provider plan. Only a model with a configured provider API key and
optional `CLOUD_ENABLED_MODELS` membership is exposed in cloud mode.

Providers and source references:

- OpenAI: https://developers.openai.com/api/docs/models
- Anthropic: https://platform.claude.com/docs/en/models/overview
- Google Gemini: https://ai.google.dev/gemini-api/docs/models
- xAI: https://docs.x.ai/developers/models
- Mistral: https://docs.mistral.ai/models
- DeepSeek: https://api-docs.deepseek.com/en/
- Groq: https://console.groq.com/docs/models
- Cohere: https://docs.cohere.com/docs/models
- OpenRouter: https://openrouter.ai/docs
- Together: https://docs.together.ai/docs/serverless-models
- Fireworks: https://docs.fireworks.ai
- Cerebras: https://inference-docs.cerebras.ai
- Perplexity: https://docs.perplexity.ai

Users may type a newer vendor-supported model ID in BYOK. Live cloud model
availability can be checked from the authenticated browser CLI:

```sh
charlotte-settings ai cloud models
```

An API key is required for hosted BYOK providers. Paid usage and entitlements
depend on the provider; **do not claim all models are free or universally
available**.

## Deploy the gateway

On a trusted host with Node.js 22 or a compatible container runtime:

```sh
cd services
cp .env.example .env
# Fill SUPABASE_URL, SUPABASE_PUBLISHABLE_KEY and selected provider API keys.
# For production, store credentials in the platform secret manager instead.
node --env-file=.env src/server.mjs
```

`GET /health` is public. `GET /v1/models` and
`POST /v1/ai/respond` require `Authorization: Bearer <Charlotte user JWT>`.
The server validates that JWT through Supabase; it rejects unsigned sessions
and anonymous accounts. It does **not** accept a client-provided AI vendor key.

The service understands native OpenAI Responses, Anthropic Messages,
Gemini generateContent, Cohere Chat v2, and OpenAI-compatible Chat Completions.
Keys may be configured as `OPENAI_API_KEY`, `OPENAI_API_KEY_1`...
`OPENAI_API_KEY_8` or comma-delimited `OPENAI_API_KEYS`. Key selection
uses Node's cryptographically secure random index/shuffle; at most two keys
are attempted on provider capacity failures (429/502/503/504). Keys are
never logged or returned to the browser.

Before public deployment: put the service behind TLS, configure the matching
Supabase project, a persistent quota/billing service, strict ingress limits
and independent provider-usage alerts. Test current provider model IDs and
access tiers with actual accounts. Do not embed private keys into images or
source control. The sample values in `.env.example` are placeholders.

## Browser connection

In Charlotte's native sidebar: sign into the Charlotte account; choose a
BYOK provider and enter its own key, or save your HTTPS cloud endpoint and
select **Charlotte Cloud**. To configure from a terminal:

```sh
charlotte-settings ai providers
charlotte-settings ai cloud set https://your-ai.example.com
charlotte-settings ai cloud models
```

The service and browser must use the same Supabase identity project. No
hard-coded hosted endpoint or vendor credential is shipped in the browser.

## Signed browser updates and patches

Cloud updates are **release announcements + verified asset staging**, not
remote code execution. The service reads a *server-owned* JSON list of
release binaries, such as `updates/releases.example.json`. Each record
contains version, channel, OS, architecture, HTTPS CDN URL, SHA-256 and byte
size. The service signs a canonical representation with a server-held
Ed25519 private key and returns it at:

```text
GET /v1/updates/manifest?channel=stable&platform=windows&arch=x64&version=0.4.0
```

The desktop validates an **embedded Ed25519 public key**, the release's
platform, newer version and signature. A user-triggered `stage` command
then downloads the asset, checks its exact signed size and SHA-256 digest,
and writes a `.pending` file inside the Charlotte profile. No downloaded
file is launched or patched into the CEF install automatically.

Generate signing keys on a trusted release machine (never commit the private
key or trust a verification key downloaded from the gateway):

```sh
node services/tools/create-update-key.mjs
```

Set `UPDATE_SIGNING_PRIVATE_KEY_PEM_BASE64` in the service's secret
manager and set `CHARLOTTE_RELEASE_PUBKEY_HEX` **when compiling** the
official Rust browser release. If this public key was not compiled into the
browser, update checks are disabled. Asset URLs must be HTTPS under
`UPDATE_ASSET_HOST`, and the CDN must host those actual signed release
binaries (the sample manifest has fictitious hash/size and is NOT a
real release!).

```sh
charlotte-settings updates check stable
charlotte-settings updates stage stable
```

The installer/apply mechanism is deliberately separate. Shipping OS-specific
automatic patch application requires signed installer/notarization checks,
atomic rollback and additional security testing. **The service does not yet
automatically install updates on users' devices.**

## Test and build

```sh
cd services
npm test
npm run check
# or Docker: docker build -t charlotte-cloud-services .
```

The native Rust/CEF browser is built with `browser/build.bat` on Windows
or `sh browser/build.sh` on Linux/macOS. The existing CEF SDK and
`bundle-cef-app` prerequisites still apply.

## Security and release limitations

- Only explicitly configured accounts use Cloud; BYOK keys remain local.
- Cloud routing may send user prompts and, only with separately consented
  browser vision mode, screenshots to external model providers.
- No arbitrary user URLs or model-selected vendor credentials are accepted by
  the service. Remote provider endpoints are fixed in source.
- The service has an in-process limiter, not yet audited global quota/billing,
  abuse protection, high-availability tracing or automatic credential refresh.
- There is no complete automated installer/patcher; only signed staging.
- No real cloud deployment, model/API credential or live user account was
  configured during PR implementation.
- Native CEF/Rust compilation is unverified until GitHub runner provisioning
  is restored and successful browser builds, unit tests and end-to-end
  account/vision/MCP workflows pass.
