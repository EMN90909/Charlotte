# Charlotte website

This is the Next.js marketing website for **Charlotte**, the Noethric-branded desktop browser. It provides product information, protocol documentation, the chat-server status display, and browser download links.

## Local development

Requires Node.js 20 or later.

```sh
npm ci
npm run dev
npm run build
```

## Release/download availability

The website looks for browser release tags named `browser-v*` in `EMN90909/Charlotte`. The repository is currently private, so public requests to the GitHub releases API may return 404. In that case, download pages must display the unavailable fallback; do not claim downloadable public builds unless they are accessible.

The legacy chat backend remains configured to preserve protocol compatibility.

## Rights

The original project and its source files remain under the MIT licence, copyright (c) 2026 Samuel Abolo. The Charlotte commercial distribution may additionally include Noethric-owned proprietary material, which is subject to separate rights. See [root README](../README.md), [original MIT licence](LICENSE), and [commercial rights notice](../browser/COMMERCIAL-LICENSE.md). These notices do not reassign the original author's copyright.
