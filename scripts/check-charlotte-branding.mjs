import assert from 'node:assert/strict';
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';

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


// Fail on accidental reintroduction of the previous product name.
// Only existing external resources and documented migration procedures
// may mention the historical spelling.
const legacyAllowList = new Set([
  'browser/src/main/index.ts', // existing profile directory
  'browser/src/main/chatServerStore.ts', // real deployed hostname
  'website/src/lib/site.ts', // real deployed hostname
  'server/deploy/README.md', // deployed hostname used in instructions
  'server/deploy/setup-droplet.sh', // guard for old installation
  'server/deploy/MIGRATION.md', // explicit safe migration guide
  'CHARLOTTE_NAMING.md', // explains the compatibility exceptions
]);
const legacyTokens = /paperkite/i;
const skipDirectories = new Set(['.git', 'node_modules', '.next', 'out', 'dist', 'bin', '.venv']);
const binaryExt = /\.(png|jpe?g|webp|ico|icns|woff2?|ttf|pdf|gif|zip|exe|deb|rpm)$/i;
function auditDirectory(directory = '.') {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && skipDirectories.has(entry.name)) continue;
    const filename = path.join(directory, entry.name);
    const normalized = filename.replaceAll('\\', '/').replace(/^\.\//, '');
    assert.ok(!legacyTokens.test(normalized), `Old-brand filename remains: ${normalized}`);
    if (entry.isDirectory()) {
      auditDirectory(filename);
      continue;
    }
    if (!entry.isFile() || binaryExt.test(entry.name)) continue;
    const body = read(filename);
    if (legacyTokens.test(body)) {
      assert.ok(legacyAllowList.has(normalized), `Old-brand reference remains: ${normalized}`);
      if (normalized === 'browser/src/main/index.ts') {
        assert.match(body, /'paperkite-browser'/);
      } else if (normalized === 'browser/src/main/chatServerStore.ts' || normalized === 'website/src/lib/site.ts' || normalized === 'server/deploy/README.md') {
        assert.match(body, /paperkite-chat-service\.samuelabolo\.dev/);
      }
    }
  }
}
auditDirectory();
assert.match(read('browser/src/preload/chrome.ts'), /exposeInMainWorld\('charlotte'/);
assert.match(read('browser/src/preload/chat.ts'), /exposeInMainWorld\('charlotteChat'/);
assert.match(read('browser/src/renderer/chrome/global.d.ts'), /charlotte: CharlotteChromeApi/);
assert.match(read('browser/src/renderer/chat/global.d.ts'), /charlotteChat: CharlotteChatApi/);
assert.ok(existsSync('server/deploy/charlotte-ops.service'), 'Charlotte systemd service unit');
assert.ok(!existsSync('server/deploy/paperkite-ops.service'), 'old systemd unit removed');
assert.ok(existsSync('brand/charlotte-mark.svg'), 'Charlotte-named identity assets');
console.log('Complete Charlotte naming audit passed with intentional compatibility exceptions.');
