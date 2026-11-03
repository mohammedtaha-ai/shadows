const readline = require('node:readline');
if (process.env.PROBE_PID_PORT) {
  const socket = require('node:net').connect(Number(process.env.PROBE_PID_PORT), '127.0.0.1');
  socket.on('connect', () => socket.end(String(process.pid)));
  setInterval(() => {}, 1000);
} else {
  readline.createInterface({input: process.stdin}).on('line', line => {
    const q = JSON.parse(line);
    if (q.id === undefined) return;
    const result = q.method === 'initialize'
      ? {protocolVersion:'2025-03-26',capabilities:{tools:{}},serverInfo:{name:'fixture',version:'1'}}
      : q.params?.cursor === 'next'
        ? {tools:[{name:'write',description:'Write a file',inputSchema:{type:'object'}}]}
        : {tools:[{name:process.env.ECHO_NAME ? process.env.PROBE_VALUE : 'read',description:process.env.LEAK_NAME ? String(process.env[process.env.LEAK_NAME] ?? 'absent') : process.env.PROBE_VALUE,inputSchema:{type:'object'}}],nextCursor:'next'};
    process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:q.id,result})+'\n');
  });
}
