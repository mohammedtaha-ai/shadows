import {useEffect,useState,type FormEvent} from 'react'
import {connectionConfig,saveConnection,retryConnection,type LaunchConfig} from '../../desktop/native'
const empty:LaunchConfig={configurationId:'',executable:'',db:'',node:'',adapter:'',harness:'',aylulConfig:'',origin:'http://127.0.0.1:4318'}
const fields=[
  ['executable','Shadows executable'],['db','Existing database'],['node','Node executable'],
  ['adapter','ACP adapter'],['harness','Claude Code executable'],['aylulConfig','AYLUL configuration'],
  ['origin','Local Core address'],
] as const
export function ConnectionSettings(){
  const [config,setConfig]=useState(empty),[error,setError]=useState<string|null>(null),[busy,setBusy]=useState(false)
  useEffect(()=>{void connectionConfig().then(value=>{if(value)setConfig(value)}).catch(()=>setError('core_config_unavailable'))},[])
  async function save(event:FormEvent){
    event.preventDefault();setBusy(true);setError(null)
    try{await saveConnection(config)}catch(reason){setError(typeof reason==='string'?reason:'core_config_invalid')}finally{setBusy(false)}
  }
  return <form onSubmit={event=>{void save(event)}} className="grid max-w-xl gap-3" aria-label="Local Core settings">
    <h2 className="text-lg font-medium">Local Core</h2>
    <p className="text-sm text-muted-foreground">Use your existing local files. Core starts automatically and remains running when you quit.</p>
    {fields.map(([key,label])=><label key={key} className="grid gap-1 text-sm">{label}<input required value={config[key]} onChange={event=>setConfig({...config,[key]:event.target.value})} className="rounded border bg-background px-3 py-2"/></label>)}
    {error&&<p role="alert">{error}</p>}
    <div className="flex gap-3"><button disabled={busy} type="submit" className="rounded border px-3 py-2">Save connection</button>
      <button disabled={busy} type="button" onClick={()=>{void retryConnection().catch(()=>setError('core_unavailable'))}} className="rounded border px-3 py-2">Retry</button></div>
  </form>
}
