import { CosmeticFilter } from '../filters/cosmetic.js'
import { matchesDomain } from '../utils.js'
import type { CosmeticResult } from '../types.js'

/**
 * Index for cosmetic filters, organized by domain for fast lookup.
 */
export class CosmeticFilterIndex {
  /** Filters that apply to all domains */
  private genericFilters: CosmeticFilter[] = []
  /** Filters specific to domains */
  private domainFilters = new Map<string, CosmeticFilter[]>()
  /** Exception filters */
  private exceptions: CosmeticFilter[] = []

  private count = 0

  get size(): number {
    return this.count
  }

  add(filter: CosmeticFilter): void {
    this.count++

    if (filter.isException) {
      this.exceptions.push(filter)
      return
    }

    if (filter.isGeneric) {
      this.genericFilters.push(filter)
      return
    }

    // Index by each positive domain
    for (const domain of filter.domains.block) {
      let list = this.domainFilters.get(domain)
      if (!list) {
        list = []
        this.domainFilters.set(domain, list)
      }
      list.push(filter)
    }
  }

  addAll(filters: CosmeticFilter[]): void {
    for (const filter of filters) {
      this.add(filter)
    }
  }

  /** Get all cosmetic results applicable to a hostname */
  getCosmeticsForHostname(hostname: string): CosmeticResult {
    const hidingSelectors: string[] = []
    const injectStyles: string[] = []
    const scriptletInjections: string[] = []

    // Collect exception selectors for this hostname
    const exceptedSelectors = new Set<string>()
    const exceptedScriptlets = new Set<string>()
    for (const ex of this.exceptions) {
      if (!ex.matchesHostname(hostname)) continue
      if (ex.scriptlet) {
        exceptedScriptlets.add(`${ex.scriptlet.name}(${ex.scriptlet.args.join(', ')})`)
      } else {
        exceptedSelectors.add(ex.selector)
      }
    }

    // Process a filter and add to results if not excepted
    const processFilter = (filter: CosmeticFilter) => {
      if (filter.scriptlet) {
        // Generic scriptlets from an external list must never run globally.
        // Respect site-scoped #@# exceptions as well.
        const injection = `${filter.scriptlet.name}(${filter.scriptlet.args.join(', ')})`
        if (!filter.isGeneric && !exceptedScriptlets.has(injection)) {
          scriptletInjections.push(injection)
        }
      } else if (filter.style) {
        if (!exceptedSelectors.has(filter.selector)) {
          injectStyles.push(`${filter.selector} ${filter.style}`)
        }
      } else if (filter.selector) {
        if (!exceptedSelectors.has(filter.selector)) {
          hidingSelectors.push(filter.selector)
        }
      }
    }

    // Generic filters
    for (const filter of this.genericFilters) {
      if (filter.matchesHostname(hostname)) {
        processFilter(filter)
      }
    }

    // Domain-specific filters: check the hostname and all parent domains
    const parts = hostname.split('.')
    for (let i = 0; i < parts.length - 1; i++) {
      const domain = parts.slice(i).join('.')
      const filters = this.domainFilters.get(domain)
      if (filters) {
        for (const filter of filters) {
          if (filter.matchesHostname(hostname)) {
            processFilter(filter)
          }
        }
      }
    }

    return { hidingSelectors, injectStyles, scriptletInjections }
  }
}
