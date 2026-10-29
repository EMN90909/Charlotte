import { FilterType } from './types.js'
import { NetworkFilter } from './filters/network.js'
import { CosmeticFilter } from './filters/cosmetic.js'

export interface ParseResult {
  networkFilters: NetworkFilter[]
  exceptionFilters: NetworkFilter[]
  cosmeticFilters: CosmeticFilter[]
}

/** Detect what type of filter a line represents */
export function classifyLine(line: string): FilterType {
  const trimmed = line.trim()

  if (trimmed.length === 0) return FilterType.Empty
  if (trimmed.startsWith('!') || trimmed.startsWith('[')) return FilterType.Comment

  // Cosmetic filter indicators: ##, #@#, #?#, #$#
  if (isCosmeticFilter(trimmed)) return FilterType.Cosmetic

  return FilterType.Network
}

/** Check if a line contains a cosmetic filter separator */
function isCosmeticFilter(line: string): boolean {
  // Skip exception prefix for detection
  const str = line.startsWith('@@') ? line.slice(2) : line

  // Look for ## #@# #?# #$# patterns not at the start (domain part precedes or empty)
  for (let i = 0; i < str.length - 1; i++) {
    if (str.charCodeAt(i) === 35) { // #
      const next = str.charCodeAt(i + 1)
      if (next === 35) return true                          // ##
      if (next === 64 && str.charCodeAt(i + 2) === 35) return true  // #@#
      if (next === 63 && str.charCodeAt(i + 2) === 35) return true  // #?#
      if (next === 36 && str.charCodeAt(i + 2) === 35) return true  // #$#
    }
  }

  return false
}

/** Parse a filter list text into categorized filter objects */
export function parseFilterList(text: string): ParseResult {
  const networkFilters: NetworkFilter[] = []
  const exceptionFilters: NetworkFilter[] = []
  const cosmeticFilters: CosmeticFilter[] = []

  const lines = text.split('\n')

  for (const line of lines) {
    const trimmed = line.trim()
    const type = classifyLine(trimmed)

    if (type === FilterType.Empty || type === FilterType.Comment) {
      continue
    }

    if (type === FilterType.Cosmetic) {
      const filter = CosmeticFilter.parse(trimmed)
      if (filter) cosmeticFilters.push(filter)
      continue
    }

    // Network filter
    const filter = NetworkFilter.parse(trimmed)
    if (filter) {
      if (filter.isException) {
        exceptionFilters.push(filter)
      } else {
        networkFilters.push(filter)
      }
    }
  }

  return { networkFilters, exceptionFilters, cosmeticFilters }
}
