import {useEffect,useState} from 'react'
import {isDesktop} from '../../desktop/runtime'
import {authAction,cancelLogin,type AuthAction,type AuthView} from '../../desktop/auth'
import {AccountPanel} from './account-panel'
import {CompanionControls} from './companion-controls'
import {ConnectionSettings} from './connection-settings'

export function NativeSettings(){
  return isDesktop()?<NativeSections/>:null
}
function NativeSections(){
  const [auth,setAuth]=useState<AuthView|null>(null),[busy,setBusy]=useState(false)
  const [loggingIn,setLoggingIn]=useState(false),[error,setError]=useState<string|null>(null)
  useEffect(()=>{
    let active=true
    void authAction('status').then(view=>{if(active)setAuth(view)}).catch(()=>{if(active)setError('Unable to check account. Retry below.')})
    return ()=>{active=false}
  },[])
  async function action(kind:AuthAction){
    setBusy(true);setLoggingIn(kind==='login'||kind==='enroll');setError(null)
    try{setAuth(await authAction(kind))}
    catch{setError('Unable to complete account action. Please retry.')}
    finally{setBusy(false);setLoggingIn(false)}
  }
  return <div className="space-y-8 [&_button]:rounded [&_button]:border [&_button]:px-3 [&_button]:py-2 [&_.actions]:flex [&_.actions]:flex-wrap [&_.actions]:gap-2 [&_h2]:mb-3 [&_h2]:font-medium">
    <AccountPanel auth={auth} busy={busy} loggingIn={loggingIn} onAction={kind=>{void action(kind)}} onCancel={()=>{void cancelLogin().catch(()=>setError('Unable to cancel sign-in.'))}}/>
    {error&&<p role="alert">{error}</p>}
    <section aria-label="Desktop character"><h2>AYLUL character</h2><CompanionControls/></section>
    <ConnectionSettings/>
  </div>
}
