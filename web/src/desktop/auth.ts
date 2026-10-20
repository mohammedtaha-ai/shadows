import {nativeCall as invoke} from './native';
export type AuthView = {
  state: 'signed_out' | 'signed_in' | 'verification_refused' | 'unavailable' | 'device_revoked';
  hasEnrollment: boolean;
  principal: { accountId: string; deviceId: string; sessionId: string } | null;
  providerLogout: 'pending' | 'complete' | null;
};
export type AuthAction = 'status' | 'login' | 'enroll' | 'logout' | 'logout_all' | 'revoke_device';
export function authAction(action: AuthAction): Promise<AuthView> { return invoke('auth_action', { action }); }
export function cancelLogin(): Promise<void> { return invoke('cancel_login'); }
