import { describe, it, expect } from 'vitest'
import { OpenShield } from '../src/index.js'

describe('OpenShield', () => {
  it('blocks ad requests', () => {
    const shield = new OpenShield()
    shield.addFilters(`
||ads.example.com^
||tracker.doubleclick.net^
||analytics.evil.com^$third-party
    `)

    const result = shield.check(
      'https://ads.example.com/banner.js',
      'https://mysite.com/page',
      'script',
    )
    expect(result.blocked).toBe(true)
  })

  it('does not block clean URLs', () => {
    const shield = new OpenShield()
    shield.addFilters('||ads.example.com^')

    const result = shield.check(
      'https://clean.example.com/page.html',
      'https://mysite.com',
      'document',
    )
    expect(result.blocked).toBe(false)
  })

  it('respects exception rules', () => {
    const shield = new OpenShield()
    shield.addFilters(`
||ads.example.com^
@@||ads.example.com/safe-ad^
    `)

    // The exception should allow this specific path
    const blocked = shield.check(
      'https://ads.example.com/banner.js',
      'https://mysite.com',
      'script',
    )
    expect(blocked.blocked).toBe(true)

    const allowed = shield.check(
      'https://ads.example.com/safe-ad/creative.js',
      'https://mysite.com',
      'script',
    )
    expect(allowed.blocked).toBe(false)
    expect(allowed.exception).toBe(true)
  })

  it('returns cosmetic selectors', () => {
    const shield = new OpenShield()
    shield.addFilters(`
##.ad-banner
##.sponsored-content
example.com##.site-specific-ad
    `)

    const cosmetics = shield.getCosmeticsForUrl('https://example.com/page')
    expect(cosmetics.hidingSelectors).toContain('.ad-banner')
    expect(cosmetics.hidingSelectors).toContain('.sponsored-content')
    expect(cosmetics.hidingSelectors).toContain('.site-specific-ad')
  })

  it('does not return site-specific selectors for other domains', () => {
    const shield = new OpenShield()
    shield.addFilters(`
##.ad-banner
example.com##.site-specific-ad
    `)

    const cosmetics = shield.getCosmeticsForUrl('https://other.com/page')
    expect(cosmetics.hidingSelectors).toContain('.ad-banner')
    expect(cosmetics.hidingSelectors).not.toContain('.site-specific-ad')
  })

  it('reports stats', () => {
    const shield = new OpenShield()
    shield.addFilters(`
||ads.com^
@@||safe.com^
##.ad
    `)

    const stats = shield.getStats()
    expect(stats.networkFilterCount).toBe(1)
    expect(stats.exceptionFilterCount).toBe(1)
    expect(stats.cosmeticFilterCount).toBe(1)
  })

  it('respects $important filters over exceptions', () => {
    const shield = new OpenShield()
    shield.addFilters(`
||malware.com^$important
@@||malware.com^
    `)

    const result = shield.check(
      'https://malware.com/payload.js',
      'https://mysite.com',
      'script',
    )
    expect(result.blocked).toBe(true)
  })

  it('handles third-party filter correctly', () => {
    const shield = new OpenShield()
    shield.addFilters('||tracker.com^$third-party')

    // Same-origin should not be blocked
    const sameOrigin = shield.check(
      'https://tracker.com/script.js',
      'https://tracker.com/page',
      'script',
    )
    expect(sameOrigin.blocked).toBe(false)

    // Third-party should be blocked
    const thirdParty = shield.check(
      'https://tracker.com/script.js',
      'https://example.com/page',
      'script',
    )
    expect(thirdParty.blocked).toBe(true)
  })
})
