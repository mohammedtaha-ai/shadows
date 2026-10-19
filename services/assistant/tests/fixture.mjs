import {createServer} from 'node:http';
export async function hermesFixture() {
  const sessions = new Set(), runs = new Map();
  let submits = 0, lostReply = false, retentionSeconds = 60, immediate = true;
  const server = createServer(async (req, res) => {
    let body = ''; for await (const chunk of req) body += chunk;
    if (req.headers.authorization !== 'Bearer synthetic-service-key') {res.writeHead(401).end(); return;}
    const data = body ? JSON.parse(body) : {};
    res.setHeader('content-type','application/json');
    const reply = value => res.end(JSON.stringify(value));
    if (req.url === '/v1/capabilities') return reply({object:'hermes.api_server.capabilities',platform:'hermes-agent',auth:{type:'bearer',required:true},runtime:{mode:'server_agent'},features:{runs_idempotency:{supported:true,durable:true,retention_seconds:retentionSeconds}}});
    if (req.url === '/api/sessions' && req.method === 'POST') {sessions.add(data.id); return reply({object:'hermes.session',session:{id:data.id}});}
    if (req.url.startsWith('/api/sessions/')) {const id=req.url.split('/').at(-1); if(!sessions.has(id)){res.writeHead(404).end();return;} return reply({object:'hermes.session',session:{id}});}
    if (req.url === '/v1/runs' && req.method === 'POST') {
      submits++;
      const key=req.headers['idempotency-key']; let run=runs.get(key); const replayed=!!run;
      if(!run){run={object:'hermes.run',run_id:'run_'+String(runs.size+1),session_id:data.session_id,status:immediate?'completed':'running',output:immediate?'LOCAL_REPLY':'',completed:immediate};runs.set(key,run);}
      if(lostReply){lostReply=false;req.socket.destroy();return;}
      return reply({run_id:run.run_id,status:'started',replayed});
    }
    const id=req.url.split('/')[3], run=[...runs.values()].find(row=>row.run_id===id);
    if(!run){res.writeHead(404).end();return;}
    if(req.url.endsWith('/stop')){run.status='stopping';return reply({run_id:id,status:'stopping'});}
    return reply(run);
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  return {origin:`http://127.0.0.1:${server.address().port}`, get submits(){return submits;},get executions(){return runs.size;},
    loseReply(){lostReply=true;}, retention(seconds){retentionSeconds=seconds;}, running(){immediate=false;}, cancelRuns(){for(const run of runs.values())run.status='cancelled';}, async close(){server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}};
}
