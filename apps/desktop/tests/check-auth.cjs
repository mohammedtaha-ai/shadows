// Read-only contract/docs and backend-fixture compatibility checks.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),cp=require('node:child_process');
const root=path.resolve(__dirname,'../../..');
const YAML=require(path.join(root,'.superpowers/validation/node_modules/yaml'));
const changed=[...new Set([...cp.execFileSync('git',['diff','--name-only'],{cwd:root,encoding:'utf8'}).trim().split(/\r?\n/),...cp.execFileSync('git',['ls-files','--others','--exclude-standard'],{cwd:root,encoding:'utf8'}).trim().split(/\r?\n/)])].filter(Boolean);
let yaml=0,links=0;
for(const file of changed){
 const p=path.join(root,file);if(!fs.statSync(p).isFile())continue;
 const text=fs.readFileSync(p,'utf8');
 if(file.endsWith('.yaml')){const c=YAML.parse(text);assert.ok(fs.existsSync(path.resolve(root,c.source)),file);yaml++;}
 if(file.endsWith('.md'))for(const m of text.matchAll(/\]\(([^)]+)\)/g)){
  const target=m[1].replace(/^<|>$/g,'').split('#')[0];if(!target||/^[a-z]+:/i.test(target))continue;
  assert.ok(fs.existsSync(path.resolve(path.dirname(p),target)),`${file}: ${target}`);links++;
 }
}
const fixture=fs.readFileSync(path.join(__dirname,'fixtures/auth-v1.1.json'),'utf8');
assert.deepEqual(JSON.parse(fixture),JSON.parse(fs.readFileSync(path.join(root,'../shadows-backend/api/auth-v1.fixtures.json'),'utf8')));
assert.equal(JSON.parse(fixture).contractVersion,'1.1.0');
console.log(JSON.stringify({result:'PASS',yamlParsed:yaml,linksChecked:links,backendFixtureConformance:'PASS'}));
