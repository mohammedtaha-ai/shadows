import { describe, expect, it } from 'vitest'
import type { AuthView } from '@/desktop'
import { accountView } from './account-actions'

const auth = (state: AuthView['state'], hasEnrollment = true): AuthView => ({ state, hasEnrollment, principal: null, providerLogout: null })

describe('account actions', () => {
  it('accountView gives each state its actions', () => {
    const rows: [AuthView, string, string[]][] = [
      [auth('signed_out', false), 'You are not signed in. Everything works on this machine.', ['Sign in']],
      [auth('signed_in'), 'Signed in', ['Sign out', 'Sign out all sessions', 'Revoke this device']],
      [auth('unavailable'), 'The account server is unavailable. Your device stays registered.', ['Check again']],
      [auth('verification_refused'), 'Your session could not be verified.', ['Sign in again', 'Check again']],
      [auth('device_revoked'), 'This device was revoked.', ['Register this device again']],
    ]
    for (const [view, message, labels] of rows) {
      expect(accountView(view, false, false).message).toBe(message)
      expect(accountView(view, false, false).actions.map(action => action.label)).toEqual(labels)
      expect(accountView(view, true, true)).toEqual({ message: 'Finish signing in in your browser.', actions: [{ action: 'cancel', label: 'Cancel sign-in' }] })
    }
    expect(accountView(auth('signed_out', false), false, false).actions[0].action).toBe('enroll')
    expect(accountView(auth('signed_out'), false, false).actions[0].action).toBe('login')
    expect(accountView(auth('device_revoked'), false, false).actions[0].action).toBe('enroll')
    expect(accountView(auth('unavailable'), false, false).actions[0].action).toBe('status')
    expect(accountView(auth('signed_in'), false, false).actions.slice(1)).toEqual([
      { action: 'logout_all', label: 'Sign out all sessions', confirm: 'Sign out all backend sessions for this account?' },
      { action: 'revoke_device', label: 'Revoke this device', confirm: 'Revoke this device registration and its sessions?', danger: true },
    ])
  })
})
