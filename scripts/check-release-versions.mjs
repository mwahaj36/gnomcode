import assert from 'node:assert/strict';
import {readFile, access} from 'node:fs/promises';
const root=new URL('../',import.meta.url);
const read=path=>readFile(new URL(path,root),'utf8');
const pkg=JSON.parse(await read('web/package.json'));
const version=pkg.version;
assert.match(version,/^\d+\.\d+\.\d+$/);
for(const path of ['npm/spawnt/package.json','web/package-lock.json','apps/desktop/package.json']) assert.equal(JSON.parse(await read(path)).version,version,path);
const shim=JSON.parse(await read('npm/spawnt/package.json'));
for(const [name,dependencyVersion] of Object.entries(shim.optionalDependencies)) {
 assert.equal(dependencyVersion,version,name);
 assert.equal(JSON.parse(await read(`npm/platform/${name.replace('@spawnt/cli-','')}/package.json`)).version,version,name);
}
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
for(const route of ['downloads','docs']) {
 const html=(await read(`web/.next/server/app/${route}.html`)).replace(/<!--.*?-->/g,'');
 assert.ok(html.includes(`v${version}`),`${route}: rendered version`);
 if(process.argv.includes('--artifacts')) assert.ok(html.includes(`/downloads/Spawnt-v${version}-windows-x64.zip`),`${route}: archive link`);

 assert.doesNotMatch(html,/v1\.(?:3|4)\.\d+/,'no obsolete product version in rendered routes');
}
if(process.argv.includes('--artifacts')) for(const name of [`Spawnt-v${version}-windows-x64.zip`,`spawnt-v${version}-linux-x86_64.tar.gz`,`spawnt-v${version}-linux-aarch64.tar.gz`,`SHA256SUMS-v${version}.txt`]) await access(new URL('web/public/downloads/'+name,root));
console.log(`PASS: v${version} version consistency${process.argv.includes("--artifacts") ? " and release artifact availability" : " (release artifact availability not checked)"}`);
