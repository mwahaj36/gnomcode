import assert from 'node:assert/strict';
import {readFile, access} from 'node:fs/promises';
const root=new URL('../',import.meta.url);
const read=path=>readFile(new URL(path,root),'utf8');
const pkg=JSON.parse(await read('web/package.json'));
const version=pkg.version;
assert.match(version,/^\d+\.\d+\.\d+$/);
for(const path of ['npm/package.json','npm/fleethud/package.json','web/package-lock.json']) assert.equal(JSON.parse(await read(path)).version,version,path);
assert.match(await read('Cargo.toml'),new RegExp('\\[workspace\\.package\\][\\s\\S]*?version = "'+version.replaceAll('.','\\.')+'"'));
const setup=await read('scripts/Setup.cs');
assert.ok(setup.includes(`AssemblyVersion("${version}.0")`));
assert.ok(setup.includes(`"DisplayVersion", "${version}"`));
assert.ok(setup.includes(`lblVer.Text = "v${version}`));
for(const extension of ['sh','ps1']) {
 const source=await read(`scripts/install.${extension}`);
 assert.equal(source,await read(`web/public/downloads/install.${extension}`),'public installer mirror');
 assert.ok(source.includes(`="${version}"`) || source.includes(`= "${version}"`));
}
for(const path of ['web/app/downloads/page.tsx','web/app/downloads/PlatformDownloads.tsx']) {
 const source=await read(path);
 assert.ok(source.includes('package.json'),'website version must use package metadata');
 assert.doesNotMatch(source,/v\d+\.\d+\.\d+/,'no hardcoded product version in website copy');
}
assert.ok((await read("web/app/downloads/PlatformDownloads.tsx")).includes("/downloads/SHA256SUMS-v${version}.txt"),"checksum links use the shared version");
for(const route of ['downloads','docs']) {
 const html=(await read(`web/.next/server/app/${route}.html`)).replace(/<!--.*?-->/g,'');
 assert.ok(html.includes(`v${version}`),`${route}: rendered version`);
 assert.ok(html.includes(`/downloads/FleetHUD-v${version}-windows-x64.zip`),`${route}: archive link`);

 assert.doesNotMatch(html,/v1\.(?:3|4)\.\d+/,'no obsolete product version in rendered routes');
}
for(const name of [`FleetHUD-v${version}-windows-x64.zip`,`fleethud-v${version}-linux-x86_64.tar.gz`,`fleethud-v${version}-linux-aarch64.tar.gz`,`SHA256SUMS-v${version}.txt`]) await access(new URL('web/public/downloads/'+name,root));
console.log(`PASS: v${version} synchronized across website source/rendered routes, manifests, installers and packages`);
