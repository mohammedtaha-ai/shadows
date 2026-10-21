import type { AuthAction, AuthView } from '@/desktop/auth'

type AccountAction = { action: AuthAction | 'cancel'; label: string; confirm?: string; danger?: boolean }

export function accountView(auth: AuthView | null, busy: boolean, loggingIn: boolean): { message: string; actions: AccountAction[] } {
  if (busy && loggingIn) return { message: 'Finish signing in in your browser.', actions: [{ action: 'cancel', label: 'Cancel sign-in' }] }
  const check: AccountAction = { action: 'status', label: 'Check again' }
  if (!auth) return { message: busy ? 'Checking your account…' : 'Your session could not be verified.', actions: [check] }
  const login: AccountAction = { action: auth.hasEnrollment ? 'login' : 'enroll', label: 'Sign in' }
  switch (auth.state) {
    case 'signed_out': return { message: 'You are not signed in. Everything works on this machine.', actions: [login] }
    case 'signed_in': return { message: 'Signed in', actions: [
      { action: 'logout', label: 'Sign out' },
      { action: 'logout_all', label: 'Sign out all sessions', confirm: 'Sign out all backend sessions for this account?' },
      { action: 'revoke_device', label: 'Revoke this device', confirm: 'Revoke this device registration and its sessions?', danger: true },
    ] }
    case 'unavailable': return { message: 'The account server is unavailable. Your device stays registered.', actions: [check] }
    case 'verification_refused': return { message: 'Your session could not be verified.', actions: [{ ...login, label: 'Sign in again' }, check] }
    case 'device_revoked': return { message: 'This device was revoked.', actions: [{ action: 'enroll', label: 'Register this device again' }] }
  }
}
