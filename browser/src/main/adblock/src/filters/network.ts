import {
  RequestType,
  REQUEST_TYPE_MAP,
  ALL_REQUEST_TYPES,
  type DomainConstraint,
} from '../types.js'
import { fastHash, tokenize, matchesDomain } from '../utils.js'
import type { Request } from '../request.js'

/** Bitmask flags for network filter features */
const enum FilterFlag {
  IsException    = 1 << 0,
  IsHostnameAnchor = 1 << 1,  // ||
  IsLeftAnchor   = 1 << 2,    // |
  IsRightAnchor  = 1 << 3,    // |
  IsRegex        = 1 << 4,
  IsImportant    = 1 << 5,
  ThirdParty     = 1 << 6,
  FirstParty     = 1 << 7,
  HasRedirect    = 1 << 8,
  HasCsp         = 1 << 9,
}

export class NetworkFilter {
  readonly rawLine: string
  readonly pattern: string
  readonly flags: number
  readonly typeMask: number
  readonly domains: DomainConstraint | null
  readonly redirect: string | null
  readonly csp: string | null
  readonly tokens: number[]

  private _regex: RegExp | null = null

  private constructor(
    rawLine: string,
    pattern: string,
    flags: number,
    typeMask: number,
    domains: DomainConstraint | null,
    redirect: string | null,
    csp: string | null,
    tokens: number[],
  ) {
    this.rawLine = rawLine
    this.pattern = pattern
    this.flags = flags
    this.typeMask = typeMask
    this.domains = domains
    this.redirect = redirect
    this.csp = csp
    this.tokens = tokens
  }

  get isException(): boolean {
    return (this.flags & FilterFlag.IsException) !== 0
  }

  get isImportant(): boolean {
    return (this.flags & FilterFlag.IsImportant) !== 0
  }

  static parse(line: string): NetworkFilter | null {
    let flags = 0
    let typeMask = 0
    let domains: DomainConstraint | null = null
    let redirect: string | null = null
    let csp: string | null = null

    let filterStr = line.trim()

    // Exception rule
    if (filterStr.startsWith('@@')) {
      flags |= FilterFlag.IsException
      filterStr = filterStr.slice(2)
    }

    // Split filter from options at the last unescaped $
    let optionsStr: string | null = null
    const dollarIdx = findOptionsIndex(filterStr)
    if (dollarIdx !== -1) {
      optionsStr = filterStr.slice(dollarIdx + 1)
      filterStr = filterStr.slice(0, dollarIdx)
    }

    // Parse options
    if (optionsStr) {
      const parts = optionsStr.split(',')
      for (const part of parts) {
        const opt = part.trim().toLowerCase()

        if (opt === 'important') {
          flags |= FilterFlag.IsImportant
        } else if (opt === 'third-party' || opt === '3p') {
          flags |= FilterFlag.ThirdParty
        } else if (opt === '~third-party' || opt === '~3p' || opt === 'first-party' || opt === '1p') {
          flags |= FilterFlag.FirstParty
        } else if (opt.startsWith('domain=')) {
          domains = parseDomainOption(part.trim().slice(7))
        } else if (opt.startsWith('redirect=') || opt.startsWith('redirect-rule=')) {
          redirect = part.trim().split('=', 2)[1]
          flags |= FilterFlag.HasRedirect
        } else if (opt.startsWith('csp=')) {
          csp = part.trim().slice(4)
          flags |= FilterFlag.HasCsp
        } else if (opt.startsWith('~')) {
          // Negated type option
          const typeName = opt.slice(1)
          const typeVal = REQUEST_TYPE_MAP[typeName]
          if (typeVal !== undefined) {
            // If no type mask set yet, start with all
            if (typeMask === 0) typeMask = ALL_REQUEST_TYPES
            typeMask &= ~typeVal
          }
        } else {
          // Positive type option
          const typeVal = REQUEST_TYPE_MAP[opt]
          if (typeVal !== undefined) {
            typeMask |= typeVal
          }
        }
      }
    }

    // If no type constraints, match all types
    if (typeMask === 0) typeMask = ALL_REQUEST_TYPES

    // Regex filter
    if (filterStr.startsWith('/') && filterStr.endsWith('/') && filterStr.length > 2) {
      flags |= FilterFlag.IsRegex
      const pattern = filterStr.slice(1, -1)
      const tokens: number[] = [] // Regex filters go to wildcard bucket
      return new NetworkFilter(line, pattern, flags, typeMask, domains, redirect, csp, tokens)
    }

    // Hostname anchor ||
    if (filterStr.startsWith('||')) {
      flags |= FilterFlag.IsHostnameAnchor
      filterStr = filterStr.slice(2)
    } else if (filterStr.startsWith('|')) {
      flags |= FilterFlag.IsLeftAnchor
      filterStr = filterStr.slice(1)
    }

    // Right anchor |
    if (filterStr.endsWith('|')) {
      flags |= FilterFlag.IsRightAnchor
      filterStr = filterStr.slice(0, -1)
    }

    const pattern = filterStr
    const tokens = tokenize(pattern)

    return new NetworkFilter(line, pattern, flags, typeMask, domains, redirect, csp, tokens)
  }

  /** Check if this filter matches a request */
  match(request: Request): boolean {
    // Check resource type
    if ((this.typeMask & request.type) === 0) return false

    // Check third-party constraint
    if ((this.flags & FilterFlag.ThirdParty) !== 0 && !request.isThirdParty) return false
    if ((this.flags & FilterFlag.FirstParty) !== 0 && request.isThirdParty) return false

    // Check domain constraint
    if (this.domains !== null && !this.matchDomains(request)) return false

    // Check pattern
    return this.matchPattern(request)
  }

  private matchDomains(request: Request): boolean {
    const domains = this.domains!
    const sourceHostname = request.sourceHostname

    // If there are block domains, source must match at least one
    if (domains.block.size > 0) {
      let matched = false
      for (const d of domains.block) {
        if (matchesDomain(sourceHostname, d)) {
          matched = true
          break
        }
      }
      if (!matched) return false
    }

    // If there are allow (negated) domains, source must NOT match any
    for (const d of domains.allow) {
      if (matchesDomain(sourceHostname, d)) return false
    }

    return true
  }

  private matchPattern(request: Request): boolean {
    const url = request.url

    // Regex filter
    if ((this.flags & FilterFlag.IsRegex) !== 0) {
      return this.getRegex().test(url)
    }

    // Hostname anchor: ||pattern
    if ((this.flags & FilterFlag.IsHostnameAnchor) !== 0) {
      return this.matchHostnameAnchor(request)
    }

    // Convert pattern with wildcards/separator to regex if needed
    if (this.pattern.includes('*') || this.pattern.includes('^')) {
      return this.getRegex().test(url)
    }

    // Plain pattern matching
    const pattern = this.pattern.toLowerCase()
    const isLeftAnchored = (this.flags & FilterFlag.IsLeftAnchor) !== 0
    const isRightAnchored = (this.flags & FilterFlag.IsRightAnchor) !== 0

    if (isLeftAnchored && isRightAnchored) {
      return url === pattern
    } else if (isLeftAnchored) {
      return url.startsWith(pattern)
    } else if (isRightAnchored) {
      return url.endsWith(pattern)
    } else {
      return url.includes(pattern)
    }
  }

  private matchHostnameAnchor(request: Request): boolean {
    const hostname = request.hostname
    const pattern = this.pattern.toLowerCase()

    // Separate hostname part from path part
    const slashIdx = pattern.indexOf('/')
    const pathOrSep = pattern.indexOf('^')
    const splitIdx = slashIdx !== -1 ? slashIdx : (pathOrSep !== -1 ? pathOrSep : pattern.length)

    const hostnamePattern = pattern.slice(0, splitIdx)
    const rest = pattern.slice(splitIdx)

    // Check hostname match (exact or subdomain)
    if (!matchesDomain(hostname, hostnamePattern)) return false

    // If there's a path/separator part, match against full URL
    if (rest.length > 0) {
      if (rest.includes('*') || rest.includes('^')) {
        return this.getRegex().test(request.url)
      }
      // Find the hostname in the URL and check what follows
      const hostIdx = request.url.indexOf(hostnamePattern)
      if (hostIdx === -1) return false
      const afterHost = request.url.slice(hostIdx + hostnamePattern.length)
      return afterHost.startsWith(rest) || (rest === '^' && isSeparator(afterHost.charCodeAt(0)))
    }

    return true
  }

  private getRegex(): RegExp {
    if (this._regex === null) {
      this._regex = compilePattern(this.pattern, this.flags)
    }
    return this._regex
  }
}

/** Check if a character is an ABP separator (anything except alphanumeric, _, -, ., %) */
function isSeparator(ch: number): boolean {
  if (Number.isNaN(ch) || ch === 0) return true
  // Alphanumeric
  if ((ch >= 48 && ch <= 57) || (ch >= 65 && ch <= 90) || (ch >= 97 && ch <= 122)) return false
  // _ - . %
  if (ch === 95 || ch === 45 || ch === 46 || ch === 37) return false
  return true
}

/** Compile an ABP pattern to a RegExp */
function compilePattern(pattern: string, flags: number): RegExp {
  let regexStr = ''
  const isHostnameAnchor = (flags & FilterFlag.IsHostnameAnchor) !== 0
  const isLeftAnchor = (flags & FilterFlag.IsLeftAnchor) !== 0
  const isRightAnchor = (flags & FilterFlag.IsRightAnchor) !== 0
  const isRegex = (flags & FilterFlag.IsRegex) !== 0

  if (isRegex) {
    try {
      return new RegExp(pattern, 'i')
    } catch {
      return /^$/ // Invalid regex, match nothing
    }
  }

  // Escape special regex chars except * and ^
  regexStr = pattern.replace(/([.+?${}()|[\]\\])/g, '\\$1')

  // Replace ABP wildcards
  regexStr = regexStr.replace(/\*/g, '.*')
  regexStr = regexStr.replace(/\^/g, '(?:[^\\w\\d._%-]|$)')

  if (isHostnameAnchor) {
    regexStr = '(?:^https?://(?:[^/]*\\.)?)'  + regexStr
  } else if (isLeftAnchor) {
    regexStr = '^' + regexStr
  }

  if (isRightAnchor) {
    regexStr = regexStr + '$'
  }

  try {
    return new RegExp(regexStr, 'i')
  } catch {
    return /^$/ // Fallback for invalid patterns
  }
}

/** Find the index of the options separator ($) that's not inside a regex */
function findOptionsIndex(filter: string): number {
  // If the filter is a regex (starts and ends with /), $ inside doesn't mean options
  if (filter.startsWith('/') && filter.endsWith('/') && filter.length > 2) {
    return -1
  }

  // Find the last $ that is likely the options separator
  // We search from the right to handle patterns that contain literal $
  for (let i = filter.length - 1; i >= 0; i--) {
    if (filter.charCodeAt(i) === 36) { // $
      // Check if what follows looks like valid options
      const after = filter.slice(i + 1).toLowerCase()
      if (looksLikeOptions(after)) {
        return i
      }
    }
  }

  return -1
}

/** Known option keywords */
const KNOWN_OPTIONS = new Set([
  'script', 'image', 'stylesheet', 'object', 'xmlhttprequest', 'xhr',
  'subdocument', 'ping', 'media', 'font', 'websocket', 'other', 'popup',
  'document', '~script', '~image', '~stylesheet', '~object',
  '~xmlhttprequest', '~xhr', '~subdocument', '~ping', '~media', '~font',
  '~websocket', '~other', '~popup', '~document',
  'third-party', '~third-party', '3p', '~3p', 'first-party', '1p',
  'important', 'match-case', 'badfilter',
])

function looksLikeOptions(str: string): boolean {
  const parts = str.split(',')
  // At least the first option should be a known keyword or a key=value
  const first = parts[0].trim()
  return KNOWN_OPTIONS.has(first) ||
    first.startsWith('domain=') ||
    first.startsWith('redirect=') ||
    first.startsWith('redirect-rule=') ||
    first.startsWith('csp=') ||
    first.startsWith('rewrite=')
}

/** Parse $domain=example.com|~sub.example.com */
function parseDomainOption(value: string): DomainConstraint {
  const allow = new Set<string>()
  const block = new Set<string>()
  const parts = value.split('|')

  for (const part of parts) {
    const domain = part.trim().toLowerCase()
    if (!domain) continue
    if (domain.startsWith('~')) {
      allow.add(domain.slice(1))
    } else {
      block.add(domain)
    }
  }

  return { allow, block }
}
