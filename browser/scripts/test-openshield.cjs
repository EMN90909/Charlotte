'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const ts = require('typescript');
const cache = new Map();

// Test the actual TypeScript source with dependencies already installed at
// browser root; no npm install or independent OpenShield build is needed.
function load(filename) {
  filename = path.resolve(filename);
  if (cache.has(filename)) return cache.get(filename).exports;
  const mod = { exports: {} };
  cache.set(filename, mod);
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
  }).outputText;
  const fn = vm.runInThisContext('(function(require,module,exports){' + output + '\n})', { filename });
  fn((name) => name.startsWith('.')
    ? load(path.resolve(path.dirname(filename), name.replace(/\.js$/, '.ts')))
    : require(name), mod, mod.exports);
  return mod.exports;
}
const root = path.resolve(__dirname, '../src/main/adblock');
const { OpenShield } = load(path.join(root, 'src/index.ts'));
const { ADS, TRACKERS } = load(path.join(root, 'rules.ts'));

test('engine loads directly from source and blocks known ad hosts', () => {
  const shield = new OpenShield();
  shield.addFilters([...ADS].map(d => '||' + d + '^').join('\n'));
  assert.equal(shield.check('https://ads.doubleclick.net/a.js','https://example.com','script').blocked, true);
  assert.equal(shield.check('https://cdn.example.com/a.js','https://example.com','script').blocked, false);
});
test('privacy hosts are included in strict layer', () => {
  const shield = new OpenShield();
  shield.addFilters([...TRACKERS].map(d => '||' + d + '^').join('\n'));
  assert.equal(shield.check('https://www.google-analytics.com/collect','https://example.com','xmlhttprequest').blocked, true);
});
test('ABP exceptions and hostname boundaries remain intact', () => {
  const shield = new OpenShield();
  shield.addFilters('||ads.example.com^\n@@||ads.example.com/okay^');
  assert.equal(shield.check('https://ads.example.com/a','https://page.com','script').blocked, true);
  assert.equal(shield.check('https://ads.example.com/okay/creative.js','https://page.com','script').blocked, false);
  // ABP ^ separator excludes dots: `/okay^` must NOT match okay.js.
  assert.equal(shield.check('https://ads.example.com/okay.js','https://page.com','script').blocked, true);
  assert.equal(shield.check('https://ads.example.com.evil.net/a','https://page.com','script').blocked, false);
});
test('cosmetic index resolves domain-specific selectors', () => {
  const shield = new OpenShield();
  shield.addFilters('##.sponsored\nexample.com##.ad-banner\nexample.com##+js(fake)');
  const result = shield.getCosmeticsForUrl('https://example.com');
  assert.equal(result.hidingSelectors.includes('.ad-banner'), true);
  assert.equal(result.hidingSelectors.includes('.sponsored'), true);
  // Adapter intentionally never executes scriptlets in filter lists.
  assert.equal(result.scriptletInjections.length, 1);
});

test('domain-scoped scriptlet exceptions disable a matching injection', () => {
  const shield = new OpenShield();
  shield.addFilters('example.com##+js(set-constant, adReady, false)\nexample.com#@#+js(set-constant, adReady, false)');
  assert.equal(shield.getCosmeticsForUrl('https://example.com').scriptletInjections.length, 0);
});
test('generic scriptlets cannot affect every website', () => {
  const shield = new OpenShield();
  shield.addFilters('##+js(abort-on-property-read, adReady)');
  assert.equal(shield.getCosmeticsForUrl('https://example.com').scriptletInjections.length, 0);
});

const { parseSafeScriptlets, compileSafeScriptletSource } = load(path.join(root, 'scriptlets.ts'));
test('strict scriptlets accept only limited commands and valid property paths', () => {
  const rules = parseSafeScriptlets([
    'set-constant(adReady, false)',
    'abort-on-property-read(window.alert)',
    'set-constant(__proto__.polluted, true)',
    'set-constant(navigator.plugins, false)',
    'set-constant(flag, alert(1))',
    'set-constant(adReady, false)',
    'abort-on-property-read(adsDisabled)',
  ]);
  assert.equal(rules.length, 2);
  const source = compileSafeScriptletSource(['set-constant(adReady, false)']);
  assert.equal(typeof source, 'string');
  assert.equal(source.includes("eval("), false);
  assert.equal(compileSafeScriptletSource(['unknown(adsDisabled)']), null);
});
