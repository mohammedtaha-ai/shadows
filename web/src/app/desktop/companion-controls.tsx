import {useEffect,useState} from 'react'
import {connectCompanion,setCompanionVisible,setCompanionPaused,type CompanionState} from '@/desktop'
export function CompanionControls(){
  const [state,setState]=useState<CompanionState|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false)
  const receive=(value:CompanionState)=>setState(current=>!current||value.revision>current.revision?value:current)
  useEffect(()=>connectCompanion(receive,setError),[])
  async function act(work:()=>Promise<CompanionState>){
    setBusy(true);setError('')
    try{receive(await work())}catch{setError('Unable to update AYLUL. Please try again.')}finally{setBusy(false)}
  }
  return <div><p role="status">{state?.error?'AYLUL is unavailable. You can retry.':state?.visible?'AYLUL is on your desktop.':state?'AYLUL is hidden.':'Connecting to AYLUL…'}</p>
    <div className="mt-3 grid gap-3">
      <label className="flex gap-2"><input type="checkbox" role="switch" checked={state?.visible??false} disabled={busy||state===null} onChange={event=>{void act(()=>setCompanionVisible(event.target.checked))}}/>Show AYLUL on the desktop</label>
      <label className="flex gap-2"><input type="checkbox" role="switch" checked={state!==null&&!state.paused} disabled={busy||state===null} onChange={event=>{void act(()=>setCompanionPaused(!event.target.checked))}}/>Motion</label>
      {state?.error&&<button disabled={busy} onClick={()=>{void act(()=>setCompanionVisible(true))}}>Retry AYLUL</button>}
    </div>
    {(error||state?.error)&&<p role="alert">{error||'AYLUL is unavailable. Account and tray controls remain available.'}</p>}
  </div>
}
