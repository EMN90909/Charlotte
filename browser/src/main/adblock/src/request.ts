import { RequestType, REQUEST_TYPE_MAP } from './types.js'
import { parseHostname, getGeneralDomain, isThirdParty, tokenize } from './utils.js'

export class Request {
  readonly url: string
  readonly hostname: string
  readonly domain: string
  readonly sourceHostname: string
  readonly sourceDomain: string
  readonly type: RequestType
  readonly isThirdParty: boolean
  readonly tokens: number[]

  constructor(url: string, sourceUrl: string, type: string | RequestType) {
    this.url = url.toLowerCase()
    this.hostname = parseHostname(url)
    this.domain = getGeneralDomain(this.hostname)
    this.sourceHostname = sourceUrl ? parseHostname(sourceUrl) : ''
    this.sourceDomain = this.sourceHostname ? getGeneralDomain(this.sourceHostname) : ''
    this.type = typeof type === 'string'
      ? (REQUEST_TYPE_MAP[type.toLowerCase()] ?? RequestType.Other)
      : type
    this.isThirdParty = this.sourceHostname
      ? isThirdParty(this.hostname, this.sourceHostname)
      : false
    this.tokens = tokenize(this.url)
  }
}
