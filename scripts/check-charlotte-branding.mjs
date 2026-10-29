// Keep rights, branding and repository layout checks independent of Electron.
// The website is still TypeScript; the browser and standalone server are Rust.
import assert from 'node:assert/strict';
import {readFileSync,existsSync,readdirSync} from 'node:fs';
import path from 'node:path';
const read = path => readFileSync(path,'utf8');
const original=read('LICENSE');
assert.equal(read('browser/LICENSE'),original);
assert.match(original,/Copyright \(c\) 2026 Samuel Abolo/);
assert.match(read('browser/COMMERCIAL-LICENSE.md'),/does not revoke or narrow those pre-existing MIT permissions/);
assert.match(read('README.md'),/original MIT grant/);
assert.match(read('browser/Cargo.toml'),/name = "Charlotte"/);
assert.match(read('browser/Cargo.toml'),/cef = /);
assert.ok(existsSync('browser/src/main/mod.rs'));
assert.ok(existsSync('browser/src/renderer/mod.rs'));
assert.ok(existsSync('browser/src/ui_server.rs'));
assert.ok(existsSync('browser/Makefile'));
assert.ok(existsSync('server/Cargo.toml'));
assert.ok(!existsSync('server/go.mod'));
for (const legacy of ['browser/package.json','browser/package-lock.json','browser/forge.config.ts',
                       'browser/src/main/index.ts','browser/src/preload/chrome.ts']){
    assert.ok(!existsSync(legacy),\`Legacy Electron source is still active: \${legacy}\`);
}
assert.match(read('website/src/lib/site.ts'),/name: 'Charlotte'/);
assert.match(read('website/src/app/licensing/page.tsx'),/Samuel Abolo/);
assert.match(read('website/next.config.ts'),/output: 'export'/);
assert.ok(!existsSync('website/src/app/api/status/route.ts'));
assert.ok(existsSync('server/deploy/charlotte-ops.service'));
assert.ok(existsSync('brand/charlotte-mark.svg'));
const ignoredDirs=new Set(['.git','node_modules','.next','out','dist','bin','target','.venv']);
const binary=/\.(png|jpe?g|webp|ico|icns|woff2?|ttf|pdf|gif|zip|exe|deb|rpm)$/i;
const legacyName=/paperkite/i;
const intentional=new Set(['CHARLOTTE_NAMING.md','scripts/check-charlotte-branding.mjs',
  'website/src/lib/site.ts','server/deploy/README.md','server/deploy/setup-droplet.sh','server/deploy/MIGRATION.md']);
function scan(dir='.'){
    for(const ent of readdirSync(dir,{withFileTypes:true})){
        if(ent.isDirectory()&&ignoredDirs.has(ent.name)) continue;
        const filename=path.join(dir,ent.name).replaceAll('\\','/').replace(/^\.\//,'');
        assert.ok(!legacyName.test(filename),\`Unexpected legacy file name: \${filename}\`);
        if(ent.isDirectory()){scan(filename);continue;}
        if(!ent.isFile()||binary.test(filename))continue;
        if(legacyName.test(read(filename))){
            assert.ok(intentional.has(filename),\`Legacy branding in: \${filename}\`);
        }
    }
}
scan();
console.log('Native Rust browser, server, rights and Charlotte naming checks passed.');
