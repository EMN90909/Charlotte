import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';

const read = (p) => readFileSync(p, 'utf8');
const original = read('LICENSE');
const browserOriginal = read('browser/LICENSE');
const pkg = JSON.parse(read('browser/package.json'));
const lock = JSON.parse(read('browser/package-lock.json'));
const notice = read('browser/COMMERCIAL-LICENSE.md');

assert.equal(pkg.name, 'charlotte-browser');
assert.equal(pkg.productName, 'Charlotte');
assert.equal(lock.name, pkg.name);
assert.equal(lock.packages[''].name, pkg.name);
assert.equal(lock.packages[''].license, pkg.license);
assert.equal(pkg.license, 'SEE COMMERCIAL-LICENSE.md');

assert.equal(browserOriginal, original, 'retain the original MIT licence verbatim');
assert.match(original, /Copyright \(c\) 2026 Samuel Abolo/);
assert.match(notice, /does not revoke or narrow those pre-existing MIT permissions/);
assert.match(notice, /Noethric/);
assert.match(read('README.md'), /original MIT grant/);
assert.match(read('browser/src/renderer/chrome/index.html'), /<title>Charlotte<\/title>/);
assert.match(read('browser/src/renderer/chat/index.html'), /<title>Charlotte Chat<\/title>/);
assert.match(read('browser/scripts/smoke-test.js'), /'Charlotte', 'Charlotte Chat'/);
assert.match(read('browser/src/main/index.ts'), /app\.setPath\('userData', previousUserDataPath\)/);
assert.match(read('website/src/lib/site.ts'), /name: 'Charlotte'/);
assert.match(read('website/src/app/licensing/page.tsx'), /Samuel Abolo/);

const languageGuide = read('LANGUAGES.md');
const linguistRules = read('.gitattributes');
assert.match(languageGuide, /TypeScript/);
assert.match(languageGuide, /Go/);
assert.match(languageGuide, /Protocol Buffers/);
assert.match(linguistRules, /server\/gen\/\*\* linguist-generated=true/);
assert.match(linguistRules, /\*\.tsx linguist-language=TypeScript/);
assert.match(read('README.md'), /\[LANGUAGES\.md\]/);

assert.ok(existsSync('server/go.mod'), 'renamed Go server exists');
assert.ok(!existsSync('chat-service'), 'old chat-service directory removed');
assert.match(read('server/go.mod'), /module github.com\/EMN90909\/Charlotte\/server/);
assert.match(read('website/next.config.ts'), /output: 'export'/);
assert.ok(!existsSync('website/src/app/api/status/route.ts'), 'no request-time API in the static site');
console.log('Charlotte branding, static export and server-layout checks passed.');
