const fs = require('node:fs');
const readline = require('node:readline');
if (process.env.PID_FILE) fs.writeFileSync(process.env.PID_FILE, String(process.pid));
if (process.env.SILENT) {
  setInterval(() => {}, 1000);
} else {
  readline.createInterface({input: process.stdin}).on('line', line => {
    const q = JSON.parse(line);
    if (q.id === undefined) return;
    const result = q.method === 'initialize'
      ? {protocolVersion:'2025-03-26',capabilities:{tools:{}},serverInfo:{name:'fixture',version:'1'}}
      : q.params?.cursor === 'next'
        ? {tools:[{name:'write',description:'Write a file',inputSchema:{type:'object'}}]}
        : {tools:[{name:'read',description:process.env.PROBE_VALUE,inputSchema:{type:'object'}}],nextCursor:'next'};
    process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:q.id,result})+'\n');
  });
}
