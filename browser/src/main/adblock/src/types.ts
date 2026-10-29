/** Resource types for network filter option matching */
export enum RequestType {
  Document = 1 << 0,
  Subdocument = 1 << 1,
  Stylesheet = 1 << 2,
  Script = 1 << 3,
  Image = 1 << 4,
  Font = 1 << 5,
  Object = 1 << 6,
  XmlHttpRequest = 1 << 7,
  Ping = 1 << 8,
  Media = 1 << 9,
  WebSocket = 1 << 10,
  Other = 1 << 11,
  Popup = 1 << 12,
}

/** Map string resource type names to enum values */
export const REQUEST_TYPE_MAP: Record<string, RequestType> = {
  document: RequestType.Document,
  subdocument: RequestType.Subdocument,
  stylesheet: RequestType.Stylesheet,
  script: RequestType.Script,
  image: RequestType.Image,
  font: RequestType.Font,
  object: RequestType.Object,
  xmlhttprequest: RequestType.XmlHttpRequest,
  xhr: RequestType.XmlHttpRequest,
  ping: RequestType.Ping,
  media: RequestType.Media,
  websocket: RequestType.WebSocket,
  other: RequestType.Other,
  popup: RequestType.Popup,
}

/** All resource type bits combined */
export const ALL_REQUEST_TYPES =
  RequestType.Document |
  RequestType.Subdocument |
  RequestType.Stylesheet |
  RequestType.Script |
  RequestType.Image |
  RequestType.Font |
  RequestType.Object |
  RequestType.XmlHttpRequest |
  RequestType.Ping |
  RequestType.Media |
  RequestType.WebSocket |
  RequestType.Other |
  RequestType.Popup

/** Result of checking a URL against the engine */
export interface CheckResult {
  blocked: boolean
  exception: boolean
  redirect: string | null
  filter: string | null
}

/** Cosmetic filter results for a given page URL */
export interface CosmeticResult {
  hidingSelectors: string[]
  injectStyles: string[]
  scriptletInjections: string[]
}

/** Filter list configuration */
export interface FilterListConfig {
  id: string
  name: string
  url: string
  enabled: boolean
}

/** Engine statistics */
export interface EngineStats {
  networkFilterCount: number
  exceptionFilterCount: number
  cosmeticFilterCount: number
  filterListCount: number
}

/** Options for OpenShield constructor */
export interface OpenShieldOptions {
  fetch?: typeof globalThis.fetch
}

/** Domain constraint parsed from $domain= option */
export interface DomainConstraint {
  allow: Set<string>
  block: Set<string>
}

/** Internal filter classification */
export enum FilterType {
  Network,
  Cosmetic,
  Comment,
  Empty,
}
