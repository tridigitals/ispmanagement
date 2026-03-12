import { safeInvoke } from './core';
import type { User } from './types';

export const install = {
  checkIsInstalled: async (): Promise<boolean> => {
    const res = await safeInvoke('is_installed');
    if (typeof res === 'object' && res !== null && 'installed' in res) {
      return (res as any).installed;
    }
    return res as boolean;
  },

  installApp: async (
    adminName: string,
    adminEmail: string,
    adminPassword: string,
    appName?: string,
    appUrl?: string,
  ): Promise<User> => {
    const res = await safeInvoke('install_app', {
      adminName,
      adminEmail,
      adminPassword,
      appName,
      appUrl,
    });

    if (typeof res === 'object' && res !== null && 'user' in res) {
      return (res as any).user;
    }
    return res as User;
  },
};
