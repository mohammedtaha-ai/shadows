import { useEffect, useState } from 'react'
import { authAction, cancelLogin, type AuthAction, type AuthView } from '@/desktop/auth'
import { Button } from '@/components/ui/button'
import { accountView } from '../account-actions'

export function AccountSection() {
  const [auth, setAuth] = useState<AuthView | null>(null)
  const [busy, setBusy] = useState(true)
  const [loggingIn, setLoggingIn] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let active = true
    void authAction('status').then(view => { if (active) setAuth(view) })
      .catch(() => { if (active) setError('Unable to check account. Retry below.') })
      .finally(() => { if (active) setBusy(false) })
    return () => { active = false }
  }, [])

  async function action(kind: AuthAction) {
    if (busy) return
    setBusy(true); setLoggingIn(kind === 'login' || kind === 'enroll'); setError(null)
    try { setAuth(await authAction(kind)) }
    catch { setError('Unable to complete account action. Please retry.') }
    finally { setBusy(false); setLoggingIn(false) }
  }
  const view = accountView(auth, busy, loggingIn)
  return (
    <section className="space-y-4" aria-label="Account state">
      <p role="status">{view.message}</p>
      {auth?.state === 'signed_in' && auth.principal && <p>Account: {auth.principal.accountId}</p>}
      <div className="space-y-3">
        {view.actions.map(item => (
          <div key={item.action} className={item.danger ? 'mt-6 border-t border-border pt-4' : undefined}>
            <Button type="button" variant={item.danger ? 'destructive' : 'outline'} disabled={busy && item.action !== 'cancel'} onClick={() => {
              if (item.action === 'cancel') { void cancelLogin().catch(() => setError('Unable to cancel sign-in.')); return }
              if (!item.confirm || window.confirm(item.confirm)) void action(item.action)
            }}>{item.label}</Button>
          </div>
        ))}
      </div>
      {error && <p role="alert">{error}</p>}
      {auth?.providerLogout === 'pending' && <p>Backend session revoked. Provider sign-out was handed to your browser; completion is not confirmed here.</p>}
    </section>
  )
}
