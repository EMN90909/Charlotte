import { NetworkFilter } from '../filters/network.js'
import type { Request } from '../request.js'

/**
 * Token-based reverse index for fast network filter matching.
 *
 * Instead of testing every filter against every request (O(n)),
 * we extract tokens from filter patterns and build a hash map.
 * At match time, we extract tokens from the request URL and only
 * test filters in the matching buckets — typically O(1) average.
 */
export class NetworkFilterIndex {
  /** Map from token hash to list of filters containing that token */
  private buckets = new Map<number, NetworkFilter[]>()
  /** Filters with no usable tokens (wildcards, regex, etc.) — always checked */
  private wildcardFilters: NetworkFilter[] = []

  get size(): number {
    let count = this.wildcardFilters.length
    for (const bucket of this.buckets.values()) {
      count += bucket.length
    }
    return count
  }

  /** Add a filter to the index */
  add(filter: NetworkFilter): void {
    const tokens = filter.tokens

    if (tokens.length === 0) {
      this.wildcardFilters.push(filter)
      return
    }

    // Pick the best token: the one with the smallest existing bucket
    let bestToken = tokens[0]
    let bestSize = this.getBucketSize(tokens[0])

    for (let i = 1; i < tokens.length; i++) {
      const size = this.getBucketSize(tokens[i])
      if (size < bestSize) {
        bestSize = size
        bestToken = tokens[i]
      }
    }

    let bucket = this.buckets.get(bestToken)
    if (!bucket) {
      bucket = []
      this.buckets.set(bestToken, bucket)
    }
    bucket.push(filter)
  }

  /** Add multiple filters to the index */
  addAll(filters: NetworkFilter[]): void {
    for (const filter of filters) {
      this.add(filter)
    }
  }

  /** Find the first matching filter for a request, or null */
  match(request: Request): NetworkFilter | null {
    // Check wildcard filters (no tokens — must always be tested)
    for (const filter of this.wildcardFilters) {
      if (filter.match(request)) return filter
    }

    // Check token-indexed buckets
    const requestTokens = request.tokens
    const checked = new Set<NetworkFilter>()

    for (const token of requestTokens) {
      const bucket = this.buckets.get(token)
      if (!bucket) continue

      for (const filter of bucket) {
        if (checked.has(filter)) continue
        checked.add(filter)
        if (filter.match(request)) return filter
      }
    }

    return null
  }

  private getBucketSize(token: number): number {
    return this.buckets.get(token)?.length ?? 0
  }
}
