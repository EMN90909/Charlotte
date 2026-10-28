# Charlotte source languages

Charlotte is a multi-language monorepo. Its runtime components continue to use the languages in which they were written; rebranding does not require rewriting stable application code.

| Language / syntax | Source locations | Role |
| --- | --- | --- |
| **TypeScript** (`.ts`, `.tsx`) | `browser/src/`, `browser/forge.config.ts`, `website/src/`, `website/next.config.ts` | Electron main/preload, React interfaces, Next.js application and browser/website configuration |
| **Go** (`.go`) | `chat-service/cmd/`, `chat-service/internal/`, `chat-service/test/` | HTTP/gRPC chat backend, operations, API handlers and tests |
| **CSS** (`.css`) | `browser/src/renderer/`, `website/src/` | Browser, chat, marketing-page styling |
| **JavaScript** (`.js`, `.mjs`) | `browser/scripts/`, `scripts/`, `website/eslint.config.mjs` | Packaging checks, smoke tests, lint configuration |
| **Shell** (`.sh`) | `scripts/`, `chat-service/deploy/`, `chat-service/test/` | Versioning, releases, deployment and smoke tests |
| **HTML** (`.html`) | `browser/src/renderer/chrome/`, `browser/src/renderer/chat/` | Electron renderer document entrypoints |
| **Protocol Buffers** (`.proto`) | `chat-service/proto/` | Backend wire schema; Go bindings are generated into `chat-service/gen/` |
| **Ruby** (`.rb`) | `browser/packaging/homebrew/` | Homebrew Cask package template |

Other repository formats include `JSON` (package manifests, lockfiles), `YAML` (GitHub Actions), `SVG` (vector artwork) and `Markdown` (documentation). These are not substitutes for the primary application languages.

## Why GitHub may show different percentages

GitHub's **Languages** chart is automatically calculated by GitHub Linguist from eligible repository files. It is not a manually editable list. File sizes and exclusions affect its proportions, and it may take time to recalculate after a push. `.gitattributes` intentionally excludes generated Go protobuf bindings, package lockfiles and exported assets from language statistics without removing them from source control.

## Development commands

```sh
# TypeScript desktop
cd browser
npm ci
npx tsc --noEmit
npm start

# Next.js / TypeScript website
cd ../website
npm ci
npx tsc --noEmit
npm run build

# Go backend
cd ../chat-service
go test ./...
```

The repository contains **no approved full-source language migration**. A move from TypeScript or Go to another programming language would require a separately scoped port, feature parity and regression testing. Preserve the original copyright notices and third-party licences when changing source code. See [LICENSE](LICENSE) and [browser/COMMERCIAL-LICENSE.md](browser/COMMERCIAL-LICENSE.md).
