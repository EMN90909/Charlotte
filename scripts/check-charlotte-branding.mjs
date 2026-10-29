import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const original = read('LICENSE');
const browserOriginal = read('browser/LICENSE');
const notice = read('browser/COMMERCIAL-LICENSE.md');
const cargo = read('browser/Cargo.toml');

assert.equal(browserOriginal, original, 'retain the original MIT licence verbatim');
assert.match(original, /Copyright \(c\) 2026 Samuel Abolo/);
assert.match(notice, /does not revoke or narrow those pre-existing MIT permissions/);
assert.match(notice, /Noethric/);
assert.match(cargo, /name = "Charlotte"/);
assert.match(cargo, /cef = "=156\.2\.0\+156\.0\.14"/);
assert.match(read('browser/README.md'), /Rust \+ CEF/);
assert.match(read('browser/MIGRATION.md'), /Electron → Rust/);
assert.match(read('website/src/lib/site.ts'), /name: 'Charlotte'/);
assert.match(read('website/src/app/licensing/page.tsx'), /Samuel Abolo/);
assert.match(read('LANGUAGES.md'), /Rust/);
assert.match(read('.gitattributes'), /\*\.rs linguist-language=Rust/);
assert.ok(existsSync('server/go.mod'), 'renamed Go server exists');
assert.ok(!existsSync('chat-service'), 'old chat-service directory removed');
assert.match(read('server/go.mod'), /module github.com\/EMN90909\/Charlotte\/server/);
assert.match(read('website/next.config.ts'), /output: 'export'/);
assert.ok(existsSync('brand/charlotte-mark.svg'), 'Charlotte-named identity assets');
console.log('Charlotte native branding, licensing and server-layout checks passed.');
