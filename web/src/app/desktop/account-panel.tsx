import type {AuthView,AuthAction} from '../../desktop/auth';
type Props={auth:AuthView|null;busy:boolean;loggingIn:boolean;onAction:(action:AuthAction)=>void;onCancel:()=>void};
export function AccountPanel({auth,busy,loggingIn,onAction,onCancel}:Props){
  const status=busy?(loggingIn?'Complete sign-in in your browser.':'Checking your account…'):auth?.state==='signed_in'?'Signed in':auth?.state==='verification_refused'?'Your session could not be verified. Sign in again or retry.':auth?.state==='unavailable'?'The backend is unavailable. Your device registration is preserved.':auth?.state==='device_revoked'?'This device registration was revoked.':'Signed out';
  return <section aria-labelledby="account"><h2 id="account">Account</h2>
    <p role="status">{status}</p>
    {auth?.principal && !busy && <p>Account: {auth.principal.accountId}<br/>Device: {auth.principal.deviceId}</p>}
    <div className="actions">
      <button disabled={busy||auth===null} onClick={()=>onAction(auth?.hasEnrollment?'login':'enroll')}>Sign in</button>
      <button disabled={busy} onClick={()=>onAction('status')}>Check session</button>
      {auth?.hasEnrollment && <button disabled={busy} onClick={()=>{if(window.confirm('Register a new installation? Your previous registration stays separate.'))onAction('enroll');}}>Register as a new device</button>}
      {(auth?.state==='signed_in'||auth?.state==='verification_refused'||auth?.state==='unavailable') && <>
        <button disabled={busy} onClick={()=>onAction('logout')}>Sign out</button>
        <button disabled={busy} onClick={()=>{if(window.confirm('Sign out all backend sessions for this account?'))onAction('logout_all');}}>Sign out all sessions</button>
        <button disabled={busy} onClick={()=>{if(window.confirm('Revoke this device registration and its sessions?'))onAction('revoke_device');}}>Revoke this device</button>
      </>}
      {loggingIn && <button onClick={onCancel}>Cancel sign-in</button>}
    </div>
    {auth?.providerLogout==='pending' && <p>Backend session revoked. Provider sign-out was handed to your browser; completion is not confirmed here.</p>}
  </section>;
}
