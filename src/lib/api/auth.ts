import { getTokenOrThrow, safeInvoke } from './core';
import type { AuthResponse, TrustedDevice, User } from './types';

export const auth = {
  register: (email: string, password: string, name: string): Promise<AuthResponse> =>
    safeInvoke('register', { email, password, name }),

  login: (email: string, password: string): Promise<AuthResponse> =>
    safeInvoke('login', { email, password }),

  logout: (token: string): Promise<void> => safeInvoke('logout', { token }),

  changePassword: (token: string, oldPassword: string, newPassword: string): Promise<void> =>
    safeInvoke('change_password', { token, old_password: oldPassword, new_password: newPassword }),

  getCurrentUser: (token: string): Promise<User> => safeInvoke('get_current_user', { token }),

  validateToken: (token: string): Promise<boolean> => safeInvoke('validate_token', { token }),

  verifyEmail: (token: string): Promise<AuthResponse> => safeInvoke('verify_email', { token }),

  forgotPassword: (email: string): Promise<void> => safeInvoke('forgot_password', { email }),

  resetPassword: (token: string, password: string): Promise<void> =>
    safeInvoke('reset_password', { token, password }),

  enable2FA: (): Promise<{ secret: string; qr: string }> =>
    safeInvoke('enable_2fa', { token: getTokenOrThrow() }),

  verify2FASetup: (secret: string, code: string): Promise<{ recovery_codes: string[] }> =>
    safeInvoke('verify_2fa_setup', { token: getTokenOrThrow(), secret, code }),

  disable2FA: (code: string): Promise<void> =>
    safeInvoke('disable_2fa', { token: getTokenOrThrow(), code }),

  request2FADisableCode: (): Promise<void> =>
    safeInvoke('request_2fa_disable_code', { token: getTokenOrThrow() }),

  resetUser2FA: (userId: string): Promise<void> =>
    safeInvoke('reset_user_2fa', { token: getTokenOrThrow(), userId }),

  verifyLogin2FA: (tempToken: string, code: string, trustDevice?: boolean): Promise<AuthResponse> =>
    safeInvoke('verify_login_2fa', { tempToken, code, trustDevice }),

  requestEmailOtp: (tempToken: string): Promise<void> =>
    safeInvoke('request_email_otp', { tempToken }),

  verifyEmailOtp: (tempToken: string, code: string, trustDevice?: boolean): Promise<AuthResponse> =>
    safeInvoke('verify_email_otp', { tempToken, code, trustDevice }),

  get2FAMethods: (): Promise<string[]> => safeInvoke('get_2fa_methods', {}),

  set2FAPreference: (method: string): Promise<void> =>
    safeInvoke('set_2fa_preference', { token: getTokenOrThrow(), method }),

  requestEmail2FASetup: (): Promise<void> =>
    safeInvoke('request_email_2fa_setup', { token: getTokenOrThrow() }),

  verifyEmail2FASetup: (code: string): Promise<void> =>
    safeInvoke('verify_email_2fa_setup', { token: getTokenOrThrow(), code }),

  listTrustedDevices: (): Promise<TrustedDevice[]> =>
    safeInvoke('list_trusted_devices', { token: getTokenOrThrow() }),

  revokeTrustedDevice: (deviceId: string): Promise<void> =>
    safeInvoke('revoke_trusted_device', { token: getTokenOrThrow(), deviceId }),
};
