import React from 'react';
import { createRoot } from 'react-dom/client';
import './shell.css';
import { hideToTray } from '../platform/windows';
import { authAction, cancelLogin, type AuthView, type AuthAction } from '../platform/auth';
import { CompanionControls } from './CompanionControls';
import { useAylulState } from '../data/aylul-state';
import { SessionsPanel } from '../features/sessions/SessionsPanel';
import { ApprovalsPanel } from '../features/approvals/ApprovalsPanel';
import { AssistantPage } from '../features/assistant/AssistantPage';

const Studio = import.meta.env.DEV
  ? React.lazy(() => import('../features/companion/studio/CompanionStudio').then(module => ({default:module.CompanionStudio})))
  : null;
class PresenceBoundary extends React.Component<{children:React.ReactNode},{failed:boolean}> {
  state={failed:false};
  static getDerivedStateFromError(){return {failed:true};}
  render(){return this.state.failed?<p role="alert">AYLUL view is unavailable. Account and tray actions remain available.</p>:this.props.children;}
}

function Shell() {
  const aylul=useAylulState();
  const [studioOpen,setStudioOpen]=React.useState(false);
  const [error,setError]=React.useState('');
  const [auth,setAuth]=React.useState<AuthView|null>(null);
  const [busy,setBusy]=React.useState(false);
  const [loggingIn,setLoggingIn]=React.useState(false);
  const once=React.useRef(false);
  async function act(action:AuthAction) {
    setBusy(true);setError('');setLoggingIn(action==='login'||action==='enroll');
    try {setAuth(await authAction(action));}
    catch(e) {setError(message(String(e))); if(action!=='status') {try {setAuth(await authAction('status'));} catch {setAuth(null);}}}
    finally {setBusy(false);setLoggingIn(false);}
  }
  React.useEffect(()=>{if(!once.current){once.current=true;void act('status');}},[]);
  return <main>
    <p className="eyebrow">NYLRA · Windows</p>
    <h1>Your companion, when you need it.</h1>
    <p>Keep this panel nearby, or close it to the tray and reopen it when you need it.</p>
    <AssistantPage auth={busy?null:auth} />
    <section aria-labelledby="account"><h2 id="account">Account</h2>
      <p role="status">{busy ? (loggingIn?'Complete sign-in in your browser.':'Checking your account…') : auth?.state==='signed_in'?'Signed in':auth?.state==='verification_refused'?'Your session could not be verified. Sign in again or retry.':auth?.state==='unavailable'?'The backend is unavailable. Your device registration is preserved.':auth?.state==='device_revoked'?'This device registration was revoked.':'Signed out'}</p>
      {auth?.principal && !busy && <p>Account: {auth.principal.accountId}<br/>Device: {auth.principal.deviceId}</p>}
      <div className="actions">
        <button disabled={busy||auth===null} onClick={()=>void act(auth?.hasEnrollment?'login':'enroll')}>Sign in</button>
        <button disabled={busy} onClick={()=>void act('status')}>Check session</button>
        {auth?.hasEnrollment && <button disabled={busy} onClick={()=>{if(window.confirm('Register a new installation? Your previous registration stays separate.'))void act('enroll');}}>Register as a new device</button>}
        {(auth?.state==='signed_in'||auth?.state==='verification_refused'||auth?.state==='unavailable') && <>
          <button disabled={busy} onClick={()=>void act('logout')}>Sign out</button>
          <button disabled={busy} onClick={()=>{if(window.confirm('Sign out all backend sessions for this account?'))void act('logout_all');}}>Sign out all sessions</button>
          <button disabled={busy} onClick={()=>{if(window.confirm('Revoke this device registration and its sessions?'))void act('revoke_device');}}>Revoke this device</button>
        </>}
        {loggingIn && <button onClick={()=>void cancelLogin().catch(()=>setError('Unable to cancel sign-in.'))}>Cancel sign-in</button>}
      </div>
      {auth?.providerLogout==='pending' && <p>Backend session revoked. Provider sign-out was handed to your browser; completion is not confirmed here.</p>}
    </section>
    <SessionsPanel state={aylul.state} refresh={aylul.refresh} />
    <ApprovalsPanel state={aylul.state} refresh={aylul.refresh} />
    <section aria-labelledby="presence"><h2 id="presence">AYLUL</h2>
      <PresenceBoundary><CompanionControls /></PresenceBoundary>
      {Studio && <>
        <button type="button" onClick={()=>setStudioOpen(value=>!value)}>{studioOpen?'Close development studio':'Open development studio'}</button>
        {studioOpen && <PresenceBoundary><React.Suspense fallback={<p>Loading development studio…</p>}><Studio /></React.Suspense></PresenceBoundary>}
      </>}
    </section>
    <button onClick={()=>void hideToTray().catch(()=>setError('Unable to hide the panel. Please try again.'))}>Hide to tray</button>
    {error && <p role="alert">{error}</p>}
    <footer>Open from the tray or press Ctrl+Alt+Shift+F12.<br /><span lang="ar" dir="rtl">أيلول معك حين تحتاجه.</span></footer>
  </main>;
}
function message(code:string) {
  const messages:Record<string,string>={request_refused:'Sign-in was refused. Retry with your existing registration, or explicitly register a new device.',verification_refused:'Session verification was refused. Your device registration is preserved.',backend_unavailable:'Cannot reach the backend. Please retry.',cancelled:'Sign-in cancelled.',login_timeout:'Sign-in timed out. Start again.',storage_invalid:'Protected account storage could not be read. No identity was accepted.',storage_unavailable:'Unable to save protected account storage.',certificate_unavailable:'Development certificate is unavailable. Check the native launch configuration.'};
  return messages[code]??'Unable to complete this action. Please retry.';
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><Shell /></React.StrictMode>);
