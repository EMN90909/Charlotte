import { describe, it, expect } from 'vitest'
import { classifyLine, parseFilterList } from '../src/parser.js'
import { FilterType } from '../src/types.js'

describe('classifyLine', () => {
  it('classifies empty lines', () => {
    expect(classifyLine('')).toBe(FilterType.Empty)
    expect(classifyLine('   ')).toBe(FilterType.Empty)
  })

  it('classifies comments', () => {
    expect(classifyLine('! This is a comment')).toBe(FilterType.Comment)
    expect(classifyLine('[Adblock Plus 2.0]')).toBe(FilterType.Comment)
  })

  it('classifies cosmetic filters', () => {
    expect(classifyLine('##.ad-banner')).toBe(FilterType.Cosmetic)
    expect(classifyLine('example.com##.ad')).toBe(FilterType.Cosmetic)
    expect(classifyLine('example.com#@#.ad')).toBe(FilterType.Cosmetic)
  })

  it('classifies network filters', () => {
    expect(classifyLine('||ads.example.com^')).toBe(FilterType.Network)
    expect(classifyLine('@@||example.com^')).toBe(FilterType.Network)
    expect(classifyLine('/banner\\d+/')).toBe(FilterType.Network)
  })
})

describe('parseFilterList', () => {
  it('parses a filter list with mixed content', () => {
    const text = `! Title: Test List
[Adblock Plus 2.0]

||ads.example.com^
||tracker.net^$third-party
@@||example.com^
##.ad-banner
example.com##.sidebar-ad
`
    const result = parseFilterList(text)
    expect(result.networkFilters.length).toBe(2)
    expect(result.exceptionFilters.length).toBe(1)
    expect(result.cosmeticFilters.length).toBe(2)
  })

  it('handles empty input', () => {
    const result = parseFilterList('')
    expect(result.networkFilters.length).toBe(0)
    expect(result.exceptionFilters.length).toBe(0)
    expect(result.cosmeticFilters.length).toBe(0)
  })
})
