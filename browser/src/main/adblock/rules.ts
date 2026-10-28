/**
 * Charlotte's zero-dependency network filter. No DOM scans, regexes on the
 * request hot path, remote code, or per-request filesystem work.
 *
 * This is a deliberately curated host-based starter list, NOT full EasyList
 * or cosmetic-filter support. Extend and review these sets as needed.
 * All comparisons use exact domain labels (never includes/endsWith alone).
 */
import type { AdBlockSettings } from '../../shared/types';

const ADS = new Set(`
2mdn.net
ad-delivery.net
ad-maven.com
adnxs.com
adroll.com
adsafeprotected.com
adskeeper.com
adsrvr.org
adsterra.com
adsterratools.com
advertising.com
advertserve.com
adform.net
adfox.ru
adition.com
adk2x.com
adkernel.com
admanmedia.com
adman.gr
admatic.com.tr
admob.com
adnow.com
adpushup.com
ads.pubmatic.com
adsby.bidtheatre.com
adsensecustomsearchads.com
adskeeper.co.uk
adsmarket.com
adsnative.com
adspirit.de
adtech.de
adtelligent.com
adthrive.com
adverticum.net
advertising.yahoo.com
amung.us
aniview.com
bannersnack.com
beap.gemini.yahoo.com
bidr.io
bidswitch.net
bidtheatre.com
casalemedia.com
clickadu.com
clickaine.com
contextweb.com
criteo.com
criteo.net
doubleclick.net
exoclick.com
exosrv.com
ezoic.net
flashtalking.com
googleadservices.com
googlesyndication.com
gumgum.com
hilltopads.net
inmobi.com
lijit.com
media.net
mgid.com
moatads.com
monetag.com
nativeads.com
openx.net
outbrain.com
popads.net
popcash.net
popunder.net
propellerads.com
pubmatic.com
revcontent.com
rubiconproject.com
serving-sys.com
sharethrough.com
smartadserver.com
smartyads.com
sovrn.com
spotx.tv
spotxchange.com
taboola.com
teads.tv
themoneytizer.com
trafficjunky.net
triplelift.com
yieldmo.com
yieldlab.net
yieldmanager.com
zemanta.com
`.trim().split(/\s+/));

const TRACKERS = new Set(`
addthis.com
ads-twitter.com
analytics.tiktok.com
bat.bing.com
bluekai.com
branch.io
chartbeat.com
clarity.ms
clicktale.net
comscore.com
contentsquare.net
demdex.net
dotomi.com
facebook.net
fullstory.com
google-analytics.com
googletagmanager.com
hotjar.com
hotjar.io
imrworldwide.com
inspectlet.com
kissmetrics.io
krxd.net
mixpanel.com
mparticle.com
newrelic.com
omtrdc.net
optimizely.com
permutive.com
quantserve.com
scorecardresearch.com
segment.io
segment.com
sentry-cdn.com
statsig.com
tealiumiq.com
trackjs.com
`.trim().split(/\s+/));

/** Hostname or any *label-separated* parent in a rule set. */
export function matchesDomain(hostname: string, domains: ReadonlySet<string>): boolean {
  let candidate = hostname.toLowerCase().replace(/\.$/, '');
  while (candidate) {
    if (domains.has(candidate)) return true;
    const dot = candidate.indexOf('.');
    if (dot < 0) return false;
    candidate = candidate.slice(dot + 1);
  }
  return false;
}

/** Bare domains only: no URLs, paths, ports, wildcards or public suffixes. */
export function normalizeAllowedDomain(raw: string): string | null {
  const host = raw.trim().toLowerCase().replace(/\.$/, '');
  if (host.length > 253 || !host.includes('.') || /[^a-z0-9.-]/.test(host)) return null;
  const labels = host.split('.');
  if (labels.some((label) => !label || label.length > 63 || !/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(label))) return null;
  if (labels[labels.length - 1].length < 2) return null;
  return host;
}

export function normalizeAdBlockSettings(input: unknown, fallback: AdBlockSettings): AdBlockSettings {
  if (!input || typeof input !== 'object') return { mode: fallback.mode, allowedSites: [...fallback.allowedSites] };
  const value = input as Partial<AdBlockSettings>;
  const mode = value.mode === 'off' || value.mode === 'balanced' || value.mode === 'strict'
    ? value.mode : fallback.mode;
  const sites = Array.isArray(value.allowedSites) ? value.allowedSites : fallback.allowedSites;
  const valid = new Set<string>();
  for (const site of sites.slice(0, 128)) {
    if (typeof site !== 'string') continue;
    const domain = normalizeAllowedDomain(site);
    if (domain) valid.add(domain);
  }
  return { mode, allowedSites: [...valid] };
}

function httpHostname(value: string): string | null {
  try {
    const url = new URL(value);
    if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
    return url.hostname.toLowerCase().replace(/\.$/, '');
  } catch {
    return null;
  }
}

/**
 * Pure, deterministic matching for ordinary tab subresources. Main-frame
 * navigations are excluded by the Electron adapter, so visiting a site never
 * yields an ad-block error page. No costly rule compilation per request.
 */
export function shouldBlockRequest(requestUrl: string, topLevelUrl: string, settings: AdBlockSettings): boolean {
  if (settings.mode === 'off') return false;
  const host = httpHostname(requestUrl);
  if (!host) return false;

  const pageHost = httpHostname(topLevelUrl);
  if (pageHost) {
    // Keep the current site itself functional, even when it hosts ads.
    if (host === pageHost) return false;
    // The exemption follows the site and its subdomains, not unrelated suffixes.
    if (matchesDomain(pageHost, new Set(settings.allowedSites))) return false;
  }

  if (matchesDomain(host, ADS)) return true;
  return settings.mode === 'strict' && matchesDomain(host, TRACKERS);
}
