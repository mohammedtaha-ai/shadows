import { providerTitle } from '../agents/provider-presets'
import {useEffect,useState} from 'react'
import {useQuery} from '@tanstack/react-query'
import {agentsQuery,providersQuery,providerPresetsQuery} from '@/api/aylul-profiles'
import {noteCompanionSend} from '../../desktop/companion-send'
import {retryConnection} from '../../desktop/native'
import {connectBinding,useCompanionBinding,selectCompanionAgent,setCompanionPreviews,type BindingView} from '../../desktop/presence'
export function CompanionSettings(){
  const agents=useQuery(agentsQuery),providers=useQuery(providersQuery),presets=useQuery(providerPresetsQuery)
  const [binding,setBinding]=useState<BindingView|null>(null),[error,setError]=useState<string|null>(null),[busy,setBusy]=useState(false)
  const receive=(view:BindingView)=>setBinding(previous=>previous&&previous.revision>=view.revision?previous:view)
  useEffect(()=>connectBinding(receive,()=>setError('Unable to load companion settings. Please retry.')),[])
  async function select(agent:string|null){
    setBusy(true);setError(null)
    try{receive(await selectCompanionAgent(agent))}
    catch{setError('Unable to select this agent. Please retry.')}finally{setBusy(false)}
  }
  async function previews(enabled:boolean){
    setBusy(true);setError(null)
    try{receive(await setCompanionPreviews(enabled))}
    catch{setError('Unable to save reply previews. Please retry.')}finally{setBusy(false)}
  }
  return <div id="companion-settings" className="mt-4 grid gap-3">
    <label className="grid gap-1">Companion agent
      <select aria-label="Companion agent" disabled={busy||!binding||!agents.data||!providers.data}
        value={binding?.agent??''} onChange={event=>{void select(event.target.value||null)}} className="rounded border bg-background p-2">
        <option value="">Choose an agent</option>
        {binding?.agent&&agents.data&&!agents.data.agents.some(agent=>agent.slug===binding.agent&&!agent.deleted)&&<option disabled value={binding.agent}>Selected agent unavailable</option>}
        {agents.data?.agents.filter(agent=>!agent.deleted).map(agent=><option key={agent.slug} value={agent.slug}>
          {agent.name} · {(() => { const provider=providers.data?.find(provider=>provider.slug===agent.provider); return provider?providerTitle(provider,presets.data??[]):agent.provider })()} · {agent.default_model}
        </option>)}
      </select>
    </label>
    <button disabled={busy} onClick={()=>{void retryConnection().catch(()=>setError("Unable to retry local Core."))}}>Retry companion connection</button>
    <p className="text-sm text-muted-foreground">This character stays with the selected agent. Opening another conversation does not change it.</p>
    <label className="flex gap-2"><input type="checkbox" role="switch" checked={binding?.previews??true} disabled={busy||!binding}
      onChange={event=>{void previews(event.target.checked)}}/>Show a short reply preview</label>
    {error&&!binding&&<button disabled={busy} onClick={()=>{void select(null)}}>Clear companion selection</button>}
    {(error||agents.error||providers.error)&&<p role="alert">{error??'Unable to load agents. Character visibility and account actions remain available.'}</p>}
  </div>
}

export function UseWithCompanion({threadId,profile,harness}:{threadId:string;profile:string|null;harness:string}){
  const binding=useCompanionBinding()
  const [busy,setBusy]=useState(false)
  if(harness!=='aylul'||!profile||binding?.agent!==profile)return null
  return <button disabled={busy} className="rounded border px-2 py-1 text-xs" onClick={()=>{
    setBusy(true);void noteCompanionSend(threadId).finally(()=>setBusy(false))
  }}>Use with companion</button>
}
