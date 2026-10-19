import React from 'react';
import {connectCompanion,setCompanionVisible,setCompanionPaused,type CompanionState} from '../platform/companion';
import {acceptState} from '../platform/companion-state.ts';
export function CompanionControls():React.JSX.Element {
  const [state,setState]=React.useState<CompanionState|null>(null),[error,setError]=React.useState(''),[busy,setBusy]=React.useState(false);
  const receive=React.useCallback((value:CompanionState)=>setState(current=>acceptState(current,value)),[]);
  React.useEffect(()=>connectCompanion(receive,setError),[receive]);
  async function act(work:()=>Promise<CompanionState>){setBusy(true);setError('');try{receive(await work());}catch{setError('Unable to update AYLUL. Please try again.');}finally{setBusy(false);}}
  return <div><p role="status">{state?.error?'AYLUL is unavailable. You can retry.':state?.visible?'AYLUL is on your desktop.':state?'AYLUL is hidden.':'Connecting to AYLUL…'}</p>
    <div className="actions"><button disabled={busy||state===null} onClick={()=>void act(()=>setCompanionVisible(state?.error?true:!state?.visible))}>{state?.error?'Retry AYLUL':state?.visible?'Hide AYLUL':'Show AYLUL'}</button>
    <button disabled={busy||state===null} onClick={()=>void act(()=>setCompanionPaused(!state?.paused))}>{state?.paused?'Resume motion':'Pause motion'}</button></div>
    {(error||state?.error)&&<p role="alert">{error||'AYLUL is unavailable. Account and tray controls remain available.'}</p>}
  </div>;
}
