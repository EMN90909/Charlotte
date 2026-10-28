import { describe, it, expect } from 'vitest'
import { NetworkFilter } from '../src/filters/network.js'
import { Request } from '../src/request.js'

describe('NetworkFilter.parse', () => {
  it('parses basic blocking filter', () => {
    const f = NetworkFilter.parse('||ads.example.com^')!
    expect(f).not.toBeNull()
    expect(f.isException).toBe(false)
    expect(f.rawLine).toBe('||ads.example.com^')
  })

  it('parses exception filter', () => {
    const f = NetworkFilter.parse('@@||example.com^')!
    expect(f).not.toBeNull()
    expect(f.isException).toBe(true)
  })

  it('parses filter with options', () => {
    const f = NetworkFilter.parse('||ads.example.com^$third-party,script')!
    expect(f).not.toBeNull()
  })

  it('parses regex filter', () => {
    const f = NetworkFilter.parse('/banner\\d+/')!
    expect(f).not.toBeNull()
  })

  it('parses filter with domain option', () => {
    const f = NetworkFilter.parse('||ads.com^$domain=example.com|~sub.example.com')!
    expect(f).not.toBeNull()
    expect(f.domains).not.toBeNull()
    expect(f.domains!.block.has('example.com')).toBe(true)
    expect(f.domains!.allow.has('sub.example.com')).toBe(true)
  })

  it('parses redirect filter', () => {
    const f = NetworkFilter.parse('||ads.com^$redirect=noop.js')!
    expect(f.redirect).toBe('noop.js')
  })
})

describe('NetworkFilter.match', () => {
  it('blocks matching hostname anchor', () => {
    const f = NetworkFilter.parse('||ads.example.com^')!
    const req = new Request('https://ads.example.com/banner.js', 'https://page.com', 'script')
    expect(f.match(req)).toBe(true)
  })

  it('blocks matching subdomain', () => {
    const f = NetworkFilter.parse('||example.com^')!
    const req = new Request('https://sub.example.com/page', 'https://other.com', 'document')
    expect(f.match(req)).toBe(true)
  })

  it('does not match unrelated domain', () => {
    const f = NetworkFilter.parse('||ads.example.com^')!
    const req = new Request('https://clean.site.com/page', 'https://page.com', 'document')
    expect(f.match(req)).toBe(false)
  })

  it('respects third-party option', () => {
    const f = NetworkFilter.parse('||tracker.com^$third-party')!
    // Same-origin request should NOT match
    const sameOrigin = new Request('https://tracker.com/track', 'https://tracker.com', 'script')
    expect(f.match(sameOrigin)).toBe(false)

    // Third-party request SHOULD match
    const thirdParty = new Request('https://tracker.com/track', 'https://example.com', 'script')
    expect(f.match(thirdParty)).toBe(true)
  })

  it('respects resource type option', () => {
    const f = NetworkFilter.parse('||ads.com^$script')!
    const scriptReq = new Request('https://ads.com/ad.js', 'https://page.com', 'script')
    const imageReq = new Request('https://ads.com/ad.png', 'https://page.com', 'image')
    expect(f.match(scriptReq)).toBe(true)
    expect(f.match(imageReq)).toBe(false)
  })

  it('matches plain substring filter', () => {
    const f = NetworkFilter.parse('doubleclick')!
    const req = new Request('https://ad.doubleclick.net/ad.js', 'https://page.com', 'script')
    expect(f.match(req)).toBe(true)
  })

  it('matches regex filter', () => {
    const f = NetworkFilter.parse('/banner\\d+\\.js/')!
    const req = new Request('https://ads.com/banner123.js', 'https://page.com', 'script')
    expect(f.match(req)).toBe(true)
  })

  it('respects domain option', () => {
    const f = NetworkFilter.parse('||ads.com^$domain=example.com')!
    const matchReq = new Request('https://ads.com/ad.js', 'https://example.com/page', 'script')
    const noMatchReq = new Request('https://ads.com/ad.js', 'https://other.com/page', 'script')
    expect(f.match(matchReq)).toBe(true)
    expect(f.match(noMatchReq)).toBe(false)
  })
})
