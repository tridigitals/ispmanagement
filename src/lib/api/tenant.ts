import { getTokenOrThrow, safeInvoke } from './core';

export const tenant = {
  getSelf: (): Promise<any> => safeInvoke('get_current_tenant', { token: getTokenOrThrow() }),

  updateSelf: (data: {
    name?: string;
    customDomain?: string;
    enforce2fa?: boolean;
  }): Promise<any> =>
    safeInvoke('update_current_tenant', {
      token: getTokenOrThrow(),
      name: data.name,
      customDomain: data.customDomain,
      enforce2fa: data.enforce2fa,
    }),
};
