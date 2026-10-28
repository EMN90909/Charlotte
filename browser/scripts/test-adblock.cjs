'use strict';
const { readFileSync } = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const { test } = require('node:test');
const ts = require('typescript');

// Keep tests dependency-free: TypeScript is already installed for builds.
// The matcher has type-only imports and no dependency on the Electron process.
const file = path.join(__dirname, '../src/main/adblock/rules.ts');
const js = ts.transpileModule(readFileSync(file, 'utf8'), {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
}).outputText;
const moduleRef = { exports: {} };
vm.runInNewContext(js, { module: moduleRef, exports: moduleRef.exports, URL, Set });
const { matchesDomain, normalizeAllowedDomain, normalizeAdBlockSettings, shouldBlockRequest } = moduleRef.exports;
const balanced = { mode: 'balanced', allowedSites: [] };
const strict = { mode: 'strict', allowedSites: [] };

test('blocks known ad hosts and subdomains without confusing suffixes', () => {
  assert.equal(shouldBlockRequest('https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js', 'https://news.example.com/', balanced), true);
  assert.equal(shouldBlockRequest('https://foo.doubleclick.net/pixel', 'https://news.example.com/', balanced), true);
  assert.equal(shouldBlockRequest('https://notdoubleclick.net/pixel', 'https://news.example.com/', balanced), false);
  assert.equal(matchesDomain('evilgoogleadservices.com', new Set(['googleadservices.com'])), false);
  assert.equal(matchesDomain('ads.googleadservices.com', new Set(['googleadservices.com'])), true);
});

test('balanced permits trackers, strict filters them', () => {
  assert.equal(shouldBlockRequest('https://www.google-analytics.com/collect', 'https://shop.example/', balanced), false);
  assert.equal(shouldBlockRequest('https://www.google-analytics.com/collect', 'https://shop.example/', strict), true);
  assert.equal(shouldBlockRequest('https://www.google-analytics.com/collect', 'https://shop.example/', { mode: 'off', allowedSites: [] }), false);
});

test('site exceptions apply to the browsing site, not an unrelated suffix', () => {
  const cfg = { mode: 'strict', allowedSites: ['example.com'] };
  assert.equal(shouldBlockRequest('https://doubleclick.net/ad', 'https://shop.example.com/', cfg), false);
  assert.equal(shouldBlockRequest('https://doubleclick.net/ad', 'https://notexample.com/', cfg), true);
  assert.equal(shouldBlockRequest('https://doubleclick.net/ad', 'https://example.com.evil.org/', cfg), true);
});

test('non-HTTP resources and malformed URLs are ignored', () => {
  assert.equal(shouldBlockRequest('data:text/plain,ads', 'https://example.com/', strict), false);
  assert.equal(shouldBlockRequest('file:///local/index.html', 'https://example.com/', strict), false);
  assert.equal(shouldBlockRequest('invalid URL', 'https://example.com/', strict), false);
  assert.equal(shouldBlockRequest('https://doubleclick.net/file', 'https://doubleclick.net/', strict), false);
});

test('invalid exception entries are rejected; duplicates are normalized', () => {
  assert.equal(normalizeAllowedDomain('EXAMPLE.com.'), 'example.com');
  assert.equal(normalizeAllowedDomain('https://example.com'), null);
  assert.equal(normalizeAllowedDomain('example.com:123'), null);
  assert.equal(normalizeAllowedDomain('evil..com'), null);
  assert.equal(normalizeAllowedDomain('com'), null);
  assert.equal(normalizeAllowedDomain('foo-.example.com'), null);
  const result = normalizeAdBlockSettings({ mode: 'strict', allowedSites: ['EXAMPLE.COM', 'example.com', 'bad/path', '', 123] }, balanced);
  assert.deepEqual(JSON.parse(JSON.stringify(result)), { mode: 'strict', allowedSites: ['example.com'], allowedHosts: [], blockedHosts: [] });
  assert.equal(normalizeAdBlockSettings({ mode: 'bogus', allowedSites: 'bad' }, balanced).mode, 'balanced');
});


test('manual host allow overrides manual block and excludes malformed domains', () => {
  const result = normalizeAdBlockSettings({
    mode: 'strict', allowedSites: [], allowedHosts: ['ADS.EXAMPLE.COM', 'https://evil.com'],
    blockedHosts: ['ads.example.com', 'metrics.example.com'],
  }, balanced);
  assert.deepEqual(JSON.parse(JSON.stringify(result.allowedHosts)), ['ads.example.com']);
  assert.deepEqual(JSON.parse(JSON.stringify(result.blockedHosts)), ['metrics.example.com']);
});
