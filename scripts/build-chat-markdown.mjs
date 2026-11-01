import {readFile,readdir,writeFile} from 'node:fs/promises';

const root=new URL('../',import.meta.url);
const bundle=(await readFile(new URL('web/node_modules/markdown-it/dist/browser/markdown-it.esm.min.mjs',root),'utf8')).replace(/^\s*\/\/# sourceMappingURL=.*$/gm,'').trimEnd()+'\n';
const notices=[];
for(const name of ['markdown-it','mdurl','argparse','entities','uc.micro','linkify-it','punycode.js']){
  const directory=new URL(`web/node_modules/${name}/`,root);
  const manifest=JSON.parse(await readFile(new URL('package.json',directory),'utf8'));
  const license=(await readdir(directory)).find(file=>/^licen[sc]e(?:[.-]|$)/i.test(file));
  if(!license)throw Error(`Missing license for ${name}`);
  notices.push(`${name} ${manifest.version} (${manifest.license})\n\n${await readFile(new URL(license,directory),'utf8')}`);
}
for(const [path,expected] of [['web/lib/markdown-it.mjs',bundle],['web/public/vendor/MARKDOWN-LICENSE.txt',notices.join('\n\n---\n\n').trimEnd()+'\n']]){
  const file=new URL(path,root);
  if(process.argv.includes('--check')){
    if(await readFile(file,'utf8')!==expected)throw Error(`Stale ${path}; run node scripts/build-chat-markdown.mjs`);
  }else await writeFile(file,expected,'utf8');
}
console.log('PASS: pinned browser Markdown module and dependency notices match');
