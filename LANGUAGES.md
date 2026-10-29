# Charlotte source languages

Charlotte is a multi-language monorepo. The native desktop browser is Rust and
CEF; the web site and backend keep their appropriate runtimes.

| Language / syntax | Source locations | Role |
| --- | --- | --- |
| **Rust** (`.rs`) | `browser/src/` | Native Charlotte host, CEF lifecycle and packaging contract |
| **TypeScript** (`.ts`, `.tsx`) | `website/src/`, `website/next.config.ts` | Next.js marketing and download site |
| **Go** (`.go`) | `server/cmd/`, `server/internal/`, `server/test/` | HTTP/gRPC chat backend, operations and tests |
| **CSS** (`.css`) | `website/src/` | Marketing-page styling |
| **JavaScript** (`.mjs`) | `scripts/`, `website/eslint.config.mjs` | Repository checks and lint configuration |
| **Shell** (`.sh`) | `browser/scripts/`, `scripts/`, `server/deploy/` | Native builds, releases, deployment and smoke tests |
| **Protocol Buffers** (`.proto`) | `server/proto/` | Backend wire schema |
| **Ruby** (`.rb`) | packaging templates | Homebrew package template |

GitHub's Languages chart is calculated by Linguist. Generated bindings,
dependency manifests and exported assets are intentionally excluded in
`.gitattributes`.

## Development commands

```sh
# Native browser
cd browser
scripts/build.sh

# Next.js website
cd ../website
npm ci
npm run build

# Go backend
cd ../server
go test ./...
```

Preserve the original copyright notices and third-party licences when
changing source code. See [LICENSE](LICENSE) and
[browser/COMMERCIAL-LICENSE.md](browser/COMMERCIAL-LICENSE.md).
