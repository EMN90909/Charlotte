import { describe, it, expect } from 'vitest'
import {
  fastHash,
  tokenize,
  parseHostname,
  getGeneralDomain,
  isThirdParty,
  matchesDomain,
} from '../src/utils.js'

describe('fastHash', () => {
  it('returns consistent hashes', () => {
    expect(fastHash('hello')).toBe(fastHash('hello'))
  })

  it('returns different hashes for different strings', () => {
    expect(fastHash('hello')).not.toBe(fastHash('world'))
  })

  it('returns a positive number', () => {
    expect(fastHash('test')).toBeGreaterThan(0)
  })
})

describe('tokenize', () => {
  it('extracts tokens of 5+ characters', () => {
    const tokens = tokenize('hello world test ab')
    // 'hello' and 'world' qualify, 'test' is 4 chars, 'ab' is 2
    expect(tokens.length).toBe(2)
  })

  it('skips common tokens', () => {
    const tokens = tokenize('https://www.example.com/tracking')
    // 'https', 'www', 'com' are skipped; 'example' and 'tracking' should remain
    expect(tokens.length).toBe(2)
  })

  it('returns empty array for short input', () => {
    expect(tokenize('ab cd')).toEqual([])
  })
})

describe('parseHostname', () => {
  it('extracts hostname from http URL', () => {
    expect(parseHostname('http://example.com/path')).toBe('example.com')
  })

  it('extracts hostname from https URL', () => {
    expect(parseHostname('https://www.example.com:443/path?q=1')).toBe('www.example.com')
  })

  it('handles URL with auth', () => {
    expect(parseHostname('http://user:pass@example.com/path')).toBe('example.com')
  })

  it('lowercases hostname', () => {
    expect(parseHostname('https://Example.COM/Path')).toBe('example.com')
  })
})

describe('getGeneralDomain', () => {
  it('returns domain for simple hostname', () => {
    expect(getGeneralDomain('example.com')).toBe('example.com')
  })

  it('extracts eTLD+1 from subdomain', () => {
    expect(getGeneralDomain('sub.example.com')).toBe('example.com')
  })

  it('handles two-part TLDs', () => {
    expect(getGeneralDomain('sub.example.co.uk')).toBe('example.co.uk')
  })

  it('handles deep subdomains', () => {
    expect(getGeneralDomain('a.b.c.example.com')).toBe('example.com')
  })
})

describe('isThirdParty', () => {
  it('returns false for same domain', () => {
    expect(isThirdParty('cdn.example.com', 'www.example.com')).toBe(false)
  })

  it('returns true for different domains', () => {
    expect(isThirdParty('ads.doubleclick.net', 'example.com')).toBe(true)
  })
})

describe('matchesDomain', () => {
  it('matches exact domain', () => {
    expect(matchesDomain('example.com', 'example.com')).toBe(true)
  })

  it('matches subdomain', () => {
    expect(matchesDomain('sub.example.com', 'example.com')).toBe(true)
  })

  it('does not match unrelated domain', () => {
    expect(matchesDomain('notexample.com', 'example.com')).toBe(false)
  })
})
