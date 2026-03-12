import { getTokenOrThrow, safeInvoke } from './core';
import type { AuthSettings, EmailVerificationReadiness, Setting, SmtpConnectionTestResult } from './types';

export const settings = {
  getAll: (): Promise<Setting[]> => safeInvoke('get_all_settings', { token: getTokenOrThrow() }),

  getPublicSettings: (): Promise<{
    app_name?: string;
    app_description?: string;
    default_locale?: string;
    app_timezone?: string;
    currency_code?: string;
    base_currency_code?: string;
    maintenance_mode?: boolean;
    maintenance_message?: string;
    payment_midtrans_enabled?: boolean;
    payment_midtrans_client_key?: string;
    payment_midtrans_is_production?: boolean;
    payment_manual_enabled?: boolean;
  }> => safeInvoke('get_public_settings'),

  getAuthSettings: (): Promise<AuthSettings> => safeInvoke('get_auth_settings'),

  get: (key: string): Promise<Setting | null> =>
    safeInvoke('get_setting', { token: getTokenOrThrow(), key }),

  getValue: (key: string): Promise<string | null> =>
    safeInvoke('get_setting_value', { token: getTokenOrThrow(), key }),

  getEmailVerificationReadiness: (): Promise<EmailVerificationReadiness> =>
    safeInvoke('get_email_verification_readiness', { token: getTokenOrThrow() }),

  upsert: (key: string, value: string, description?: string): Promise<Setting> =>
    safeInvoke('upsert_setting', { token: getTokenOrThrow(), key, value, description }),

  uploadLogo: (fileBase64: string): Promise<string> =>
    safeInvoke('upload_logo', { token: getTokenOrThrow(), content: fileBase64 }),

  getLogo: (token?: string): Promise<string | null> => safeInvoke('get_logo', { token }),

  delete: (key: string): Promise<void> =>
    safeInvoke('delete_setting', { token: getTokenOrThrow(), key }),

  sendTestEmail: (toEmail: string): Promise<string> =>
    safeInvoke('send_test_email', { token: getTokenOrThrow(), toEmail }),

  testSmtpConnection: (): Promise<SmtpConnectionTestResult> =>
    safeInvoke('test_smtp_connection', { token: getTokenOrThrow() }),

  getAppVersion: async (): Promise<string> => {
    const res = await safeInvoke('get_app_version');
    if (typeof res === 'object' && res !== null && 'version' in res) {
      return (res as any).version;
    }
    return (res as string) || '0.0.0';
  },
};
