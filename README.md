# Charlotte

Charlotte is a Noethric-branded commercial browser distribution where anyone viewing the same webpage
can talk to each other in real time. Open a page, land in its chat room,
no accounts, no invites. Bring an AI agent into the conversation, or run
the whole chat backend yourself.

This repository holds three projects:

| Directory | What it is | Stack |
|-----------|------------|-------|
| [`browser/`](browser) | The Charlotte desktop browser | Electron, TypeScript, React |
| [`server/`](chat-service) | The chat backend the browser talks to | Go |
| [`website/`](website) | The marketing site, protocol docs, and live status page | Next.js, TypeScript, React |

## Source languages

Charlotte retains its existing working language stack. **TypeScript and TSX** power the Electron browser and Next.js website, while **Go** powers the chat service. Supporting source includes **CSS**, **JavaScript**, **Shell scripts**, a **Protocol Buffers** schema, and a **Ruby** Homebrew Cask template. JSON and YAML configure tooling and automation; HTML provides renderer entrypoints.

Read [LANGUAGES.md](LANGUAGES.md) for the source-language inventory, major directories, and validation commands. GitHub detects the displayed language breakdown automatically through Linguist; [.gitattributes](.gitattributes) explicitly marks generated files so they do not distort that display. No source language has been migrated.

## Features

- **Live per-page chat.** Every tab hashes its URL into a room. Anyone
  else on that URL right now can talk to you there.
- **Private servers.** `chat-service` is a small, stateless Go service.
  Run your own instance and point the browser at it instead of a shared
  one.
- **AI agents.** Chat with Claude, GPT, Gemini, or a local Ollama model
  about the page you're on, from a panel next to the tab.
- **MCP client and server.** Add any Model Context Protocol server to the
  browser for your agents to use. Charlotte also exposes the page, room,
  and chat as MCP tools of its own.
- **Live audio translation.** Whisper-powered, with support for swapping
  in custom models.
- **Whole-page translation.** Open-source or commercial models, your
  choice.
- **Existing rights preserved.** The original browser, protocol and server carry Samuel Abolo's 2026 MIT notice; new Noethric-owned commercial additions may be proprietary.

See [server/PROTOCOL.md](server/PROTOCOL.md) for the wire
protocol, or the same thing rendered at [`website/`](website)'s
`/protocol` page.

## Quickstart

Run the chat backend, then the browser, in two terminals:

```bash
# terminal 1: server
cd server
CHAT_JWT_SECRET=dev-secret make run

# terminal 2: browser
cd browser
npm install
npm start
```

The browser defaults to `http://localhost:8080`, matching `chat-service`'s
default HTTP port. Change the server it talks to from Settings → Chat
Servers if you're pointing at something else.

## Building from scratch

### server (Go)

**Standalone build:** `server/` can be copied to a separate directory and built without the browser, website or parent scripts. From inside it, `make build` creates `bin/charlotte-server`, `make verify-standalone` validates an isolated copy, and `make docker` builds a non-root image. See [server/README.md](server/README.md) for cross-platform binaries and deployment settings.


Requires Go 1.23+.

```bash
cd server
make build          # go build ./...
CHAT_JWT_SECRET=dev-secret make run   # go run ./cmd/server, HTTP :8080, gRPC :50051
make test           # unit + integration tests
```

`make proto` regenerates the protobuf/gRPC code from
`proto/chat/chat.proto` and requires `protoc` plus the Go gRPC plugins on
your `PATH`. You only need this if you're editing the `.proto` file, the
generated code is already checked into `gen/`.

To run it in Docker instead:

```bash
cd server
make docker                              # builds charlotte-server:local
docker run -p 8080:8080 -p 50051:50051 -e CHAT_JWT_SECRET=dev-secret charlotte-server:local
```

Running your own server long-term? `cmd/ops` is a small daemon that
auto-deploys new releases for you (CI notifies it, it downloads,
health-checks, and rolls back on failure) instead of you manually
swapping binaries. See
[`server/packaging/README.md`](server/packaging/README.md#auto-deploy-via-the-ops-daemon).

### browser (Electron)

Requires Node 20+.

```bash
cd browser
npm install
npm start            # run in dev mode
npm run package       # package the app for your current platform
npm run make          # package and build platform installers (.zip, .exe, .deb, .rpm)
```

Packaged installers land in `browser/out/make/`. See
[`browser/packaging/README.md`](browser/packaging/README.md) for install
commands per platform and the current package manager story (Homebrew,
apt).

### website (Next.js)

Requires Node 20+.

```bash
cd website
npm install
npm run dev
```

## Releasing

This is a monorepo with three independently-versioned projects, so a
release is scoped by which one you're actually releasing, not "cut a
release for the whole repo." Each project keeps its own
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) file and
[semver](https://semver.org/) version. The flow, for whichever project
changed:

```bash
make browser-bump-patch              # or chat-service- / website-, or -minor / -major
# review CHANGELOG.md, then commit
git commit -am "browser: v1.0.1"
make browser-tag                     # creates the tag, prints the push command
git push origin browser-v1.0.1       # this is what actually kicks off CI
```

`bump-*` only touches files (bumps `package.json` for browser/website, a
plain `VERSION` file for chat-service, and moves the changelog's
`[Unreleased]` section under a dated heading), no git side effects.
`tag` creates a local annotated `<project>-vX.Y.Z` tag but never pushes,
pushing is the deliberate step that triggers a release. Each `make
<project>-*` target can also be run directly inside that project's
directory (`cd browser && make bump-patch`).

Pushing a tag triggers the matching workflow, which builds **only** that
project, since the tag prefix is what tells CI what changed:

- `browser-v*` → [`release-browser.yml`](.github/workflows/release-browser.yml)
  builds Windows/Linux installers and attaches them to a GitHub Release
  (created automatically from the tag). macOS isn't built by CI right now
  (no Apple Developer credentials for signing yet) - Mac users build from
  source instead. See
  [`browser/packaging/README.md`](browser/packaging/README.md).
- `chat-service-v*` → [`release-chat-service.yml`](.github/workflows/release-chat-service.yml)
  cross-compiles binaries for linux/darwin/windows (amd64 + arm64),
  attaches them to a GitHub Release, pushes a multi-arch Docker image to
  the release's native server artifacts, and, if `OPS_WEBHOOK_URL` is
  configured, notifies a running `cmd/ops` daemon to deploy it (download,
  health-checked swap, automatic rollback on failure). See
  [`server/packaging/README.md`](server/packaging/README.md).
- `website-v*` isn't wired to a workflow, the site deploys however you've
  set up hosting (Vercel, etc.); the tag just keeps its version/changelog
  consistent with the other two.

## License

**Original work:** MIT, copyright (c) 2026 Samuel Abolo. The [original licence](LICENSE) and each component's MIT notice remain unchanged. Previously granted MIT rights remain in force and may be exercised by recipients of that original code.

**Charlotte commercial distribution:** Noethric is the commercial product brand/distributor. Only separately owned or authorized Noethric material is subject to proprietary terms. Read [browser/COMMERCIAL-LICENSE.md](browser/COMMERCIAL-LICENSE.md). Renaming a product, making this repository private or adding a proprietary notice does **not** transfer the original author's copyright or terminate the original MIT grant. A separate signed copyright assignment or exclusive licence would be needed to change ownership of the original work.

Third-party dependencies retain their individual licences; preserve all applicable notices in binary distributions.

## Charlotte account authentication

The Electron browser includes Supabase email/password sign-in and sign-up in **Settings → Account**. Credentials and refresh tokens are handled by the privileged Electron main process, not an untrusted webpage. Configure `CHARLOTTE_SUPABASE_URL` and `CHARLOTTE_SUPABASE_PUBLISHABLE_KEY` with a Charlotte-approved Supabase project. The Go server provides a verification-only `GET /auth/me` endpoint using matching `SUPABASE_URL` and `SUPABASE_PUBLISHABLE_KEY`. See [server/README.md](server/README.md). The legacy chat username protocol is independent.

## Render static site

The `website/` Next.js application exports to `website/out` for Render Static Sites. Build from repository root with `cd website && npm ci && npm run build`, publish `website/out`. No Node.js server runs on the static host. The status page does its network checks client-side, and private GitHub releases might not appear in download listings. See [website/README.md](website/README.md).
