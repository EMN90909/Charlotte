# Charlotte network ad blocking

A zero-runtime-dependency, local, host-based blocker for requests originating
from **browsing tabs**, implemented with Electron's
`session.defaultSession.webRequest.onBeforeRequest`.

- **Balanced** (default): block known ad-delivery hostnames.
- **Strict**: block the same hosts plus common analytics/tracker hostnames.
- **Off**: do not block.
- Per-site exceptions are saved to `userData/adblock.json`; only the
  active browsing site's domain (and its subdomains) is excepted.
- Top-level navigations, internal chrome/chat, local resources and other
  non-tab requests are not intercepted. On changes, newly created requests
  adopt the new mode immediately; reload to re-fetch existing resources.
- Hot path uses prebuilt hash sets, O(1) tab membership and domain-label suffix
  matching; site exceptions are compiled on settings updates. No
  network filter downloads, regular expression matching or DOM traversal.
- Blocked count is session-only, delivered to React at most once per 250 ms.

**Scope:** This is a curated *network-level* list, not an EasyList-compatible
parser. It cannot block first-party advertisements, cosmetic placeholders,
inline sponsored content or every new ad network. Host lists are committed
in `rules.ts` and need maintenance. The blocker should not be advertised
as having uBlock Origin's full coverage. To expand coverage, add a reviewed,
licensed and updateable filter format rather than fetching remote executable
code, and keep Electron's single-listener limitation in mind.

Run `npm run test:adblock` in `browser/` to exercise the pure matcher.
