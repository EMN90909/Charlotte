# OpenShield-js

A pure TypeScript ad-blocking engine for Node.js and the browser. Block ads, trackers, and annoyances using the same filter list format trusted by millions of users worldwide.

Zero dependencies. Works everywhere JavaScript runs.

## Installation

```bash
npm install openshield-js
```

## Quick Start

```ts
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()

// Load popular filter lists
await shield.loadFilterLists(['easylist', 'easyprivacy'])

// Check if a network request should be blocked
const result = shield.check(
  'https://ads.doubleclick.net/ad.js',  // request URL
  'https://mysite.com/page',            // page URL
  'script'                               // resource type
)

console.log(result.blocked) // true

// Get CSS selectors to hide ad elements on a page
const cosmetics = shield.getCosmeticsForUrl('https://example.com')
console.log(cosmetics.hidingSelectors) // ['.ad-banner', '#sidebar-ad', ...]
```

## Features

- **Pure TypeScript** — no native binaries, no WebAssembly, no build steps required
- **Zero Dependencies** — nothing to audit, nothing to break
- **Cross-Platform** — works in Node.js, browsers, Deno, Bun, and edge runtimes
- **Dual Output** — ships both ESM and CommonJS, use it anywhere
- **Fast Matching** — token-based reverse index for near-instant URL lookups, even with 70,000+ filters loaded
- **Full Filter Syntax** — supports Adblock Plus and uBlock Origin filter formats
- **Network Filtering** — block requests by URL pattern, domain, resource type, and party context
- **Cosmetic Filtering** — hide ad elements with CSS selector rules
- **8 Built-in Lists** — popular community-maintained filter lists ready to use
- **TypeScript First** — full type definitions included out of the box

## API Reference

### `new OpenShield(options?)`

Create a new OpenShield instance.

```ts
const shield = new OpenShield()

// With custom fetch (for environments without global fetch)
const shield = new OpenShield({
  fetch: myCustomFetchFunction
})
```

### `shield.loadFilterLists(lists)`

Load one or more built-in filter lists. Returns a Promise.

```ts
await shield.loadFilterLists(['easylist', 'easyprivacy'])
```

**Available built-in lists:**

| List ID | Description |
|---------|-------------|
| `easylist` | Primary ad-blocking list |
| `easyprivacy` | Tracker and privacy protection |
| `peter-lowe` | Peter Lowe's ad and tracking server list |
| `fanboy-annoyance` | Annoyances (popups, cookie notices, etc.) |
| `fanboy-social` | Social media widget blocking |
| `ublock-filters` | uBlock Origin community filters |
| `ublock-badware` | Badware and malicious site protection |
| `ublock-privacy` | Additional privacy filters |

### `shield.loadFilterListFromUrl(url)`

Load a custom filter list from any URL.

```ts
await shield.loadFilterListFromUrl('https://example.com/my-filters.txt')
```

### `shield.addFilters(text)`

Add raw filter rules directly as a string.

```ts
shield.addFilters(`
||ads.example.com^
||tracker.example.net^$third-party
##.ad-banner
`)
```

### `shield.check(url, sourceUrl, resourceType)`

Check if a network request should be blocked.

```ts
const result = shield.check(
  'https://ads.example.com/banner.js',
  'https://mysite.com/page',
  'script'
)
```

**Parameters:**
- `url` — the URL of the request to check
- `sourceUrl` — the URL of the page making the request
- `resourceType` — one of: `document`, `subdocument`, `stylesheet`, `script`, `image`, `font`, `object`, `xmlhttprequest`, `xhr`, `ping`, `media`, `websocket`, `other`, `popup`

**Returns:**

```ts
{
  blocked: boolean      // true if the request should be blocked
  exception: boolean    // true if an exception rule allowed it
  redirect: string | null  // redirect URL if applicable
  filter: string | null    // the filter rule that matched
}
```

### `shield.getCosmeticsForUrl(url)`

Get cosmetic filter results for a page — CSS selectors to hide ad elements.

```ts
const cosmetics = shield.getCosmeticsForUrl('https://example.com/page')
```

**Returns:**

```ts
{
  hidingSelectors: string[]      // CSS selectors to hide (e.g., '.ad-banner')
  injectStyles: string[]         // CSS styles to inject
  scriptletInjections: string[]  // Scriptlets to execute
}
```

### `shield.getStats()`

Get engine statistics.

```ts
const stats = shield.getStats()
// {
//   networkFilterCount: 45000,
//   exceptionFilterCount: 5000,
//   cosmeticFilterCount: 30000,
//   filterListCount: 2
// }
```

## Framework Integration

### React.js

Use the `useOpenShield` hook to hide ad elements and check URLs in your React app.

```tsx
// hooks/useOpenShield.ts
import { useEffect, useRef, useState } from 'react'
import { OpenShield } from 'openshield-js'

export function useOpenShield(filterRules: string) {
  const shieldRef = useRef<OpenShield | null>(null)
  const [ready, setReady] = useState(false)

  useEffect(() => {
    const shield = new OpenShield()
    shield.addFilters(filterRules)
    shieldRef.current = shield
    setReady(true)
  }, [filterRules])

  const check = (url: string, sourceUrl: string, type: string) => {
    if (!shieldRef.current) return { blocked: false }
    return shieldRef.current.check(url, sourceUrl, type)
  }

  const getCosmetics = (url: string) => {
    if (!shieldRef.current) return { hidingSelectors: [], injectStyles: [], scriptletInjections: [] }
    return shieldRef.current.getCosmeticsForUrl(url)
  }

  return { ready, check, getCosmetics }
}
```

```tsx
// components/SafeContent.tsx
import { useOpenShield } from '../hooks/useOpenShield'

const FILTER_RULES = `
||ads.example.com^
||tracker.net^$third-party
##.ad-banner
##.sponsored-content
`

export function SafeContent() {
  const { ready, check, getCosmetics } = useOpenShield(FILTER_RULES)

  // Check a URL before loading it
  const handleLinkClick = (url: string) => {
    const result = check(url, window.location.href, 'document')
    if (result.blocked) {
      console.log('Blocked:', url)
      return
    }
    window.open(url)
  }

  // Inject cosmetic filters as CSS
  useEffect(() => {
    if (!ready) return
    const { hidingSelectors } = getCosmetics(window.location.href)
    if (hidingSelectors.length === 0) return

    const style = document.createElement('style')
    style.textContent = hidingSelectors
      .map(s => `${s} { display: none !important; }`)
      .join('\n')
    document.head.appendChild(style)

    return () => { style.remove() }
  }, [ready])

  return <div>Your protected content here</div>
}
```

### Next.js (Middleware)

Block ad/tracker requests at the edge before they reach your users.

```ts
// middleware.ts
import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()
shield.addFilters(`
||ads.doubleclick.net^
||tracker.example.com^
||analytics.evil.com^$third-party
`)

export function middleware(request: NextRequest) {
  // Check all outgoing fetch/script URLs referenced by page
  const url = request.nextUrl.toString()
  const referer = request.headers.get('referer') || ''

  const result = shield.check(url, referer, 'document')

  if (result.blocked) {
    return new NextResponse('Blocked', { status: 403 })
  }

  return NextResponse.next()
}

export const config = {
  matcher: ['/api/proxy/:path*'],
}
```

### Next.js (React Server Component)

Pre-compute cosmetic filters on the server and send clean CSS to the client.

```tsx
// app/layout.tsx
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()
shield.addFilters(`
##.ad-banner
##.sponsored-content
##.tracking-pixel
`)

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const cosmetics = shield.getCosmeticsForUrl('https://mysite.com')

  const blockingCSS = cosmetics.hidingSelectors
    .map(s => `${s} { display: none !important; }`)
    .join('\n')

  return (
    <html>
      <head>
        <style dangerouslySetInnerHTML={{ __html: blockingCSS }} />
      </head>
      <body>{children}</body>
    </html>
  )
}
```

### Express.js

```ts
import express from 'express'
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()
await shield.loadFilterLists(['easylist', 'easyprivacy'])

const app = express()

app.use((req, res, next) => {
  const result = shield.check(req.url, req.headers.referer || '', 'document')
  if (result.blocked) {
    return res.status(403).send('Blocked')
  }
  next()
})
```

### Fastify

```ts
import Fastify from 'fastify'
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()
await shield.loadFilterLists(['easylist'])

const app = Fastify()

app.addHook('onRequest', async (request, reply) => {
  const result = shield.check(request.url, request.headers.referer || '', 'document')
  if (result.blocked) {
    reply.code(403).send('Blocked')
  }
})
```

### Hono (Cloudflare Workers / Edge)

```ts
import { Hono } from 'hono'
import { OpenShield } from 'openshield-js'

const app = new Hono()

const shield = new OpenShield()
shield.addFilters(`
||ads.example.com^
||tracker.net^$third-party
`)

app.use('*', async (c, next) => {
  const result = shield.check(c.req.url, c.req.header('referer') || '', 'document')
  if (result.blocked) {
    return c.text('Blocked', 403)
  }
  await next()
})

export default app
```

### Vue.js

```vue
<!-- composables/useOpenShield.ts -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { OpenShield } from 'openshield-js'

const shield = ref<OpenShield | null>(null)
const ready = ref(false)

onMounted(() => {
  const instance = new OpenShield()
  instance.addFilters(`
    ||ads.example.com^
    ##.ad-banner
    ##.sponsored
  `)
  shield.value = instance
  ready.value = true

  // Apply cosmetic filters
  const { hidingSelectors } = instance.getCosmeticsForUrl(window.location.href)
  const style = document.createElement('style')
  style.textContent = hidingSelectors
    .map(s => `${s} { display: none !important; }`)
    .join('\n')
  document.head.appendChild(style)
})

function checkUrl(url: string): boolean {
  if (!shield.value) return false
  return shield.value.check(url, window.location.href, 'document').blocked
}
</script>
```

### Svelte

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { OpenShield } from 'openshield-js'

  let blocked: string[] = []

  onMount(async () => {
    const shield = new OpenShield()
    await shield.loadFilterLists(['easylist'])

    // Apply cosmetic filters
    const { hidingSelectors } = shield.getCosmeticsForUrl(window.location.href)
    const style = document.createElement('style')
    style.textContent = hidingSelectors
      .map(s => `${s} { display: none !important; }`)
      .join('\n')
    document.head.appendChild(style)

    // Check URLs
    const urls = ['https://ads.doubleclick.net/ad.js', 'https://github.com']
    blocked = urls.filter(url =>
      shield.check(url, window.location.href, 'script').blocked
    )
  })
</script>

<p>Blocked {blocked.length} requests</p>
```

### DNS-Level Blocking (Node.js)

```ts
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()
await shield.loadFilterLists(['easylist', 'peter-lowe'])

function shouldBlockDomain(domain: string): boolean {
  return shield.check(`https://${domain}/`, '', 'document').blocked
}

shouldBlockDomain('ads.doubleclick.net') // true
shouldBlockDomain('github.com')          // false
```

### Browser Extension / Content Script

```ts
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()
await shield.loadFilterLists(['easylist'])

// Hide ad elements on the current page
const cosmetics = shield.getCosmeticsForUrl(window.location.href)

for (const selector of cosmetics.hidingSelectors) {
  const style = document.createElement('style')
  style.textContent = `${selector} { display: none !important; }`
  document.head.appendChild(style)
}
```

### Custom Filter Rules

```ts
import { OpenShield } from 'openshield-js'

const shield = new OpenShield()

shield.addFilters(`
! Block specific ad domains
||ads.example.com^
||tracker.example.net^

! Block third-party tracking scripts
||analytics.evil.com^$third-party,script

! Allow a specific safe resource
@@||cdn.example.com/safe-widget.js^

! Hide ad elements on specific sites
example.com##.sidebar-ad
example.com##.popup-overlay
##.ad-banner
`)

const result = shield.check(
  'https://ads.example.com/banner.js',
  'https://mysite.com',
  'script'
)
console.log(result.blocked) // true
```

## Filter Syntax

OpenShield supports the standard Adblock Plus / uBlock Origin filter syntax:

### Network Filters

```
||example.com^              → Block all requests to example.com
||example.com^$script       → Block only script requests
||example.com^$third-party  → Block only when loaded from other sites
@@||example.com^            → Exception: allow requests to example.com
/banner\d+\.js/             → Block URLs matching a regex
```

### Cosmetic Filters

```
##.ad-banner                → Hide elements with class "ad-banner" on all sites
example.com##.ad-banner     → Hide only on example.com
example.com#@#.ad-banner    → Exception: don't hide on example.com
```

### Filter Options

| Option | Description |
|--------|-------------|
| `$script` | Match script requests |
| `$image` | Match image requests |
| `$stylesheet` | Match CSS requests |
| `$xmlhttprequest` | Match XHR/fetch requests |
| `$third-party` | Match only third-party requests |
| `$first-party` | Match only same-origin requests |
| `$domain=example.com` | Match only on specific source domains |
| `$important` | Override exception rules |
| `$redirect=resource` | Redirect instead of blocking |

## Compatibility

| Runtime | Support |
|---------|---------|
| Node.js 18+ | Full |
| Deno | Full |
| Bun | Full |
| Modern Browsers | Full |
| Cloudflare Workers | Full |
| Vercel Edge | Full |

## License

MIT


## Embedded in Charlotte's executable

OpenShield is now imported from `browser/src/main/adblock/src/index.ts` by
`openshieldAdapter.ts` and then by the main-process `adblocker.ts`.
The top-level `browser/forge.config.ts` and Vite main entry
(`browser/src/main/index.ts`) build **one Charlotte application** with its
ad blocker in the same packaged `app.asar`. The nested `package.json`,
`tsup.config.ts`, and `package-lock.json` are for developing OpenShield as
a standalone library; **none is installed, built, spawned, or required by
Charlotte's own npm/build/release scripts**. No separate adblock process,
server, DLL or executable is needed.

The Windows release is a Charlotte installer `.exe` plus the usual installed
Electron executable and runtime files. Electron is not a single standalone
portable `.exe` file; the adblocker itself nonetheless has no separate binary.

Network filtering runs through one `webRequest.onBeforeRequest` hook for
browsing tabs only, with Off/Balanced/Strict and persisted per-site exceptions.
The built-in hostname rules work offline. EasyList (Balanced + Strict) and
EasyPrivacy (Strict) download as *data* in the background, are capped at 5MB
per list, cached under `userData/adblock-lists`, and refreshed on startup
when older than seven days. Failures leave built-in/cached protection active.
Cosmetic filtering injects **selector-only hiding CSS** into browsing pages;
filter-list scriptlets and arbitrary styles are never executed. The engine
currently implements a useful subset of ABP rules, not full uBlock Origin
compatibility.

Run from `browser/`:

```bash
npm ci
npm run test:openshield
npm run test:adblock
npx tsc --noEmit
npm run make
```


## Live Shields inspector and safe scriptlets

The main-process request hook synchronously checks browsing-tab resources
(images, JavaScript, audio/video, styles, fonts, frames and others).
It measures local rule evaluation with `performance.now()` and holds at most
120 sanitized decisions **in memory**. The Settings UI displays the latest 60
hostnames with Allow/Block, rule source, resource category and local evaluation
time. It batches updates every ~300ms. No raw URLs, paths, query strings,
cookies or resource bodies are recorded. Millisecond processing is measured,
not guaranteed.

Controls include request filters, current-site scope and saved manual host
Allow/Block/Reset overrides. Priority: Off or main document, site exception,
manual allowed host, manual blocked host, ads list, then Strict tracker list.

Strict mode supports a tiny reviewed scriptlet subset: `set-constant` for
restricted scalar values and `abort-on-property-read` on safe property paths.
Only site-scoped scriptlets, never arbitrary filter-list JavaScript, are
compiled into fixed JS templates. Execution after DOM ready can affect dynamic
ad scripts but not every early inline script. Reload to undo existing page
script effects. This does not claim complete Brave Shields parity.
