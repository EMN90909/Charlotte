import { FilterEngine } from './engine.js'
import { FilterListManager } from './lists/manager.js'
import type { BuiltInList } from './lists/defaults.js'
import type {
  CheckResult,
  CosmeticResult,
  EngineStats,
  OpenShieldOptions,
} from './types.js'

export class OpenShield {
  private engine: FilterEngine
  private manager: FilterListManager

  constructor(options?: OpenShieldOptions) {
    this.engine = new FilterEngine()
    this.manager = new FilterListManager(this.engine, options?.fetch)
  }

  /** Load built-in filter lists by name */
  async loadFilterLists(lists: BuiltInList[]): Promise<void> {
    await this.manager.loadBuiltInLists(lists)
  }

  /** Load a custom filter list from a URL */
  async loadFilterListFromUrl(url: string): Promise<void> {
    await this.manager.loadFromUrl(url)
  }

  /** Add raw filter rules directly */
  addFilters(rawFilterText: string): void {
    this.manager.loadFromText(rawFilterText)
  }

  /** Check if a network request should be blocked */
  check(url: string, sourceUrl: string, resourceType: string): CheckResult {
    return this.engine.check(url, sourceUrl, resourceType)
  }

  /** Get cosmetic filter results (CSS selectors, styles, scriptlets) for a page URL */
  getCosmeticsForUrl(url: string): CosmeticResult {
    return this.engine.getCosmeticsForUrl(url)
  }

  /** Get engine statistics */
  getStats(): EngineStats {
    const stats = this.engine.getStats()
    return { ...stats, filterListCount: this.manager.listCount }
  }
}

// Re-export types and classes for advanced usage
export type { CheckResult, CosmeticResult, EngineStats, OpenShieldOptions }
export type { BuiltInList } from './lists/defaults.js'
export { FilterEngine } from './engine.js'
export { Request } from './request.js'
export { parseFilterList } from './parser.js'
export { NetworkFilter } from './filters/network.js'
export { CosmeticFilter } from './filters/cosmetic.js'
export { RequestType } from './types.js'
