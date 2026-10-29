import { type CheckResult, type CosmeticResult, type EngineStats } from './types.js'
import { Request } from './request.js'
import { parseFilterList } from './parser.js'
import { NetworkFilterIndex } from './matching/network-index.js'
import { CosmeticFilterIndex } from './matching/cosmetic-index.js'
import { parseHostname } from './utils.js'

export class FilterEngine {
  private networkIndex = new NetworkFilterIndex()
  private exceptionIndex = new NetworkFilterIndex()
  private cosmeticIndex = new CosmeticFilterIndex()
  private importantIndex = new NetworkFilterIndex()

  /** Parse and add filters from raw filter list text */
  addFilters(text: string): void {
    const { networkFilters, exceptionFilters, cosmeticFilters } = parseFilterList(text)

    for (const filter of networkFilters) {
      if (filter.isImportant) {
        this.importantIndex.add(filter)
      } else {
        this.networkIndex.add(filter)
      }
    }

    this.exceptionIndex.addAll(exceptionFilters)
    this.cosmeticIndex.addAll(cosmeticFilters)
  }

  /** Check if a network request should be blocked */
  check(url: string, sourceUrl: string, resourceType: string): CheckResult {
    const request = new Request(url, sourceUrl, resourceType)

    // 1. Check $important filters first (cannot be overridden by exceptions)
    const importantMatch = this.importantIndex.match(request)
    if (importantMatch) {
      return {
        blocked: true,
        exception: false,
        redirect: importantMatch.redirect,
        filter: importantMatch.rawLine,
      }
    }

    // 2. Check blocking filters
    const blockMatch = this.networkIndex.match(request)
    if (!blockMatch) {
      return { blocked: false, exception: false, redirect: null, filter: null }
    }

    // 3. Check exception filters
    const exceptionMatch = this.exceptionIndex.match(request)
    if (exceptionMatch) {
      return {
        blocked: false,
        exception: true,
        redirect: null,
        filter: exceptionMatch.rawLine,
      }
    }

    return {
      blocked: true,
      exception: false,
      redirect: blockMatch.redirect,
      filter: blockMatch.rawLine,
    }
  }

  /** Get cosmetic filter results for a page URL */
  getCosmeticsForUrl(url: string): CosmeticResult {
    const hostname = parseHostname(url)
    return this.cosmeticIndex.getCosmeticsForHostname(hostname)
  }

  /** Get engine statistics */
  getStats(): EngineStats {
    return {
      networkFilterCount: this.networkIndex.size + this.importantIndex.size,
      exceptionFilterCount: this.exceptionIndex.size,
      cosmeticFilterCount: this.cosmeticIndex.size,
      filterListCount: 0, // Updated by OpenShield wrapper
    }
  }
}
