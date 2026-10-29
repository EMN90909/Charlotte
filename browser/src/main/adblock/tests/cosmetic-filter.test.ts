import { describe, it, expect } from 'vitest'
import { CosmeticFilter } from '../src/filters/cosmetic.js'

describe('CosmeticFilter.parse', () => {
  it('parses generic hiding filter', () => {
    const f = CosmeticFilter.parse('##.ad-banner')!
    expect(f).not.toBeNull()
    expect(f.selector).toBe('.ad-banner')
    expect(f.isGeneric).toBe(true)
    expect(f.isException).toBe(false)
  })

  it('parses domain-specific hiding filter', () => {
    const f = CosmeticFilter.parse('example.com##.ad-banner')!
    expect(f).not.toBeNull()
    expect(f.selector).toBe('.ad-banner')
    expect(f.isGeneric).toBe(false)
    expect(f.domains.block.has('example.com')).toBe(true)
  })

  it('parses exception filter', () => {
    const f = CosmeticFilter.parse('example.com#@#.ad-banner')!
    expect(f).not.toBeNull()
    expect(f.isException).toBe(true)
  })

  it('parses multi-domain filter', () => {
    const f = CosmeticFilter.parse('example.com,other.com##.ad-banner')!
    expect(f.domains.block.has('example.com')).toBe(true)
    expect(f.domains.block.has('other.com')).toBe(true)
  })

  it('parses negated domain', () => {
    const f = CosmeticFilter.parse('example.com,~sub.example.com##.ad-banner')!
    expect(f.domains.block.has('example.com')).toBe(true)
    expect(f.domains.allow.has('sub.example.com')).toBe(true)
  })

  it('parses scriptlet injection', () => {
    const f = CosmeticFilter.parse('example.com##+js(abort-on-property-read, adblock)')!
    expect(f.scriptlet).not.toBeNull()
    expect(f.scriptlet!.name).toBe('abort-on-property-read')
    expect(f.scriptlet!.args).toEqual(['adblock'])
  })
})

describe('CosmeticFilter.matchesHostname', () => {
  it('generic filter matches any hostname', () => {
    const f = CosmeticFilter.parse('##.ad-banner')!
    expect(f.matchesHostname('example.com')).toBe(true)
    expect(f.matchesHostname('other.com')).toBe(true)
  })

  it('domain filter matches specified domain', () => {
    const f = CosmeticFilter.parse('example.com##.ad-banner')!
    expect(f.matchesHostname('example.com')).toBe(true)
    expect(f.matchesHostname('sub.example.com')).toBe(true)
    expect(f.matchesHostname('other.com')).toBe(false)
  })

  it('respects negated domains', () => {
    const f = CosmeticFilter.parse('example.com,~sub.example.com##.ad-banner')!
    expect(f.matchesHostname('example.com')).toBe(true)
    expect(f.matchesHostname('sub.example.com')).toBe(false)
  })
})
