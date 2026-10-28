import type { FilterListConfig } from '../types.js'
import { DEFAULT_LISTS, type BuiltInList } from './defaults.js'
import { fetchFilterList } from './fetcher.js'
import { FilterEngine } from '../engine.js'

export class FilterListManager {
  private engine: FilterEngine
  private customFetch?: typeof globalThis.fetch
  private loadedLists = new Map<string, FilterListConfig>()

  constructor(engine: FilterEngine, customFetch?: typeof globalThis.fetch) {
    this.engine = engine
    this.customFetch = customFetch
  }

  /** Load one or more built-in filter lists by name */
  async loadBuiltInLists(listIds: BuiltInList[]): Promise<void> {
    const promises = listIds.map(id => this.loadBuiltInList(id))
    await Promise.all(promises)
  }

  /** Load a single built-in filter list */
  async loadBuiltInList(listId: BuiltInList): Promise<void> {
    const config = DEFAULT_LISTS[listId]
    if (!config) {
      throw new Error(`Unknown built-in filter list: ${listId}`)
    }
    await this.loadFromUrl(config.url, config)
  }

  /** Load a filter list from a custom URL */
  async loadFromUrl(url: string, config?: Partial<FilterListConfig>): Promise<void> {
    const text = await fetchFilterList(url, this.customFetch)

    const listConfig: FilterListConfig = {
      id: config?.id ?? url,
      name: config?.name ?? url,
      url,
      enabled: config?.enabled ?? true,
    }

    this.engine.addFilters(text)
    this.loadedLists.set(listConfig.id, listConfig)
  }

  /** Load filters from raw text */
  loadFromText(text: string, id?: string): void {
    this.engine.addFilters(text)
    if (id) {
      this.loadedLists.set(id, {
        id,
        name: id,
        url: '',
        enabled: true,
      })
    }
  }

  /** Get the number of loaded filter lists */
  get listCount(): number {
    return this.loadedLists.size
  }

  /** Get all loaded list configs */
  getLoadedLists(): FilterListConfig[] {
    return Array.from(this.loadedLists.values())
  }
}
