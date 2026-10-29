/** FNV-1a 32-bit hash */
export function fastHash(str: string): number {
  let hash = 0x811c9dc5
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i)
    hash = (hash * 0x01000193) | 0
  }
  return hash >>> 0
}

/** Minimum token length to consider for indexing */
const MIN_TOKEN_LENGTH = 5

/** Common tokens to skip (too frequent to be useful for indexing) */
const SKIP_TOKENS = new Set([
  'https', 'http', 'www', 'com', 'net', 'org',
])

/**
 * Extract and hash alphanumeric tokens from a string.
 * Used for both building the reverse index and querying it.
 */
export function tokenize(input: string): number[] {
  const tokens: number[] = []
  let start = -1

  for (let i = 0; i <= input.length; i++) {
    const ch = i < input.length ? input.charCodeAt(i) : 0
    const isAlnum =
      (ch >= 48 && ch <= 57) ||  // 0-9
      (ch >= 65 && ch <= 90) ||  // A-Z
      (ch >= 97 && ch <= 122)    // a-z

    if (isAlnum) {
      if (start === -1) start = i
    } else {
      if (start !== -1) {
        const len = i - start
        if (len >= MIN_TOKEN_LENGTH) {
          const token = input.slice(start, i).toLowerCase()
          if (!SKIP_TOKENS.has(token)) {
            tokens.push(fastHash(token))
          }
        }
        start = -1
      }
    }
  }

  return tokens
}

/** Extract hostname from a URL string without using URL constructor */
export function parseHostname(url: string): string {
  let start = 0

  // Skip protocol
  const protoEnd = url.indexOf('://')
  if (protoEnd !== -1) {
    start = protoEnd + 3
  }

  // Skip auth (user:pass@)
  const atSign = url.indexOf('@', start)
  if (atSign !== -1) {
    const slashPos = url.indexOf('/', start)
    if (slashPos === -1 || atSign < slashPos) {
      start = atSign + 1
    }
  }

  // Find end of hostname
  let end = url.length
  for (let i = start; i < url.length; i++) {
    const ch = url.charCodeAt(i)
    if (ch === 47 /* / */ || ch === 58 /* : */ || ch === 63 /* ? */ || ch === 35 /* # */) {
      end = i
      break
    }
  }

  return url.slice(start, end).toLowerCase()
}

/**
 * Extract the general domain (eTLD+1) from a hostname.
 * Uses a simple heuristic: take the last 2 parts, or last 3 for known
 * two-part TLDs like .co.uk, .com.au, etc.
 */
const TWO_PART_TLDS = new Set([
  'co.uk', 'co.jp', 'co.kr', 'co.nz', 'co.za', 'co.in', 'co.id',
  'com.au', 'com.br', 'com.cn', 'com.mx', 'com.tw', 'com.hk', 'com.sg',
  'com.ar', 'com.tr', 'com.ua', 'com.pk', 'com.ng', 'com.eg', 'com.ph',
  'net.au', 'net.br', 'net.cn',
  'org.uk', 'org.au', 'org.br',
  'ac.uk', 'gov.uk', 'gov.au', 'gov.br', 'gov.in',
  'edu.au', 'edu.br', 'edu.cn',
  'ne.jp', 'or.jp', 'ac.jp',
])

export function getGeneralDomain(hostname: string): string {
  const parts = hostname.split('.')
  if (parts.length <= 2) return hostname

  // Check if last two parts form a known two-part TLD
  const lastTwo = parts.slice(-2).join('.')
  if (TWO_PART_TLDS.has(lastTwo)) {
    return parts.length >= 3 ? parts.slice(-3).join('.') : hostname
  }

  return parts.slice(-2).join('.')
}

/** Check if a request is third-party relative to a source */
export function isThirdParty(requestHostname: string, sourceHostname: string): boolean {
  const requestDomain = getGeneralDomain(requestHostname)
  const sourceDomain = getGeneralDomain(sourceHostname)
  return requestDomain !== sourceDomain
}

/** Check if hostname matches a domain constraint (including subdomains) */
export function matchesDomain(hostname: string, domain: string): boolean {
  if (hostname === domain) return true
  return hostname.endsWith('.' + domain)
}
