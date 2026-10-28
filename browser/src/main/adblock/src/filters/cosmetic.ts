import { matchesDomain } from '../utils.js'

export class CosmeticFilter {
  readonly rawLine: string
  readonly selector: string
  readonly isException: boolean
  readonly isGeneric: boolean
  readonly domains: { allow: Set<string>; block: Set<string> }
  readonly style: string | null
  readonly scriptlet: { name: string; args: string[] } | null

  private constructor(
    rawLine: string,
    selector: string,
    isException: boolean,
    domains: { allow: Set<string>; block: Set<string> },
    style: string | null,
    scriptlet: { name: string; args: string[] } | null,
  ) {
    this.rawLine = rawLine
    this.selector = selector
    this.isException = isException
    this.isGeneric = domains.block.size === 0 && domains.allow.size === 0
    this.domains = domains
    this.style = style
    this.scriptlet = scriptlet
  }

  static parse(line: string): CosmeticFilter | null {
    // Find the cosmetic separator: ##, #@#, #?#, #$#
    let sepIdx = -1
    let sepLen = 2
    let isException = false
    let isStyleInjection = false

    for (let i = 0; i < line.length - 1; i++) {
      if (line.charCodeAt(i) === 35) { // #
        const next = line.charCodeAt(i + 1)

        if (next === 35) { // ##
          sepIdx = i
          sepLen = 2
          break
        }
        if (next === 64 && line.charCodeAt(i + 2) === 35) { // #@#
          sepIdx = i
          sepLen = 3
          isException = true
          break
        }
        if (next === 63 && line.charCodeAt(i + 2) === 35) { // #?# (extended CSS)
          sepIdx = i
          sepLen = 3
          break
        }
        if (next === 36 && line.charCodeAt(i + 2) === 35) { // #$# (style injection / scriptlet)
          sepIdx = i
          sepLen = 3
          isStyleInjection = true
          break
        }
      }
    }

    if (sepIdx === -1) return null

    const domainPart = line.slice(0, sepIdx).trim()
    let selector = line.slice(sepIdx + sepLen).trim()

    if (!selector) return null

    // Parse domains
    const domains = parseCosmeticDomains(domainPart)

    // Check for scriptlet injection: +js(...) or #$#+js(...)
    let scriptlet: { name: string; args: string[] } | null = null
    let style: string | null = null

    if (selector.startsWith('+js(') && selector.endsWith(')')) {
      const inner = selector.slice(4, -1)
      const args = inner.split(',').map(s => s.trim())
      scriptlet = { name: args[0], args: args.slice(1) }
      selector = ''
    } else if (isStyleInjection) {
      style = selector
      // Extract the selector part if it's a CSS injection like ".ad { display: none }"
      const braceIdx = selector.indexOf('{')
      if (braceIdx !== -1) {
        const cssSelector = selector.slice(0, braceIdx).trim()
        const cssBody = selector.slice(braceIdx).trim()
        selector = cssSelector
        style = cssBody
      }
    }

    return new CosmeticFilter(line, selector, isException, domains, style, scriptlet)
  }

  /** Check if this cosmetic filter applies to a given hostname */
  matchesHostname(hostname: string): boolean {
    if (this.isGeneric) return true

    // Check allow (negated) domains first
    for (const d of this.domains.allow) {
      if (matchesDomain(hostname, d)) return false
    }

    // If there are block (positive) domains, must match at least one
    if (this.domains.block.size > 0) {
      for (const d of this.domains.block) {
        if (matchesDomain(hostname, d)) return true
      }
      return false
    }

    return true
  }
}

function parseCosmeticDomains(domainPart: string): { allow: Set<string>; block: Set<string> } {
  const allow = new Set<string>()
  const block = new Set<string>()

  if (!domainPart) return { allow, block }

  const parts = domainPart.split(',')
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
