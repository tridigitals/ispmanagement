import { safeInvoke } from './core';
import type { AuthResponse, CustomerRegistrationInviteValidation } from './types';

export const publicApi = {
  getTenant: (slug: string): Promise<any> => safeInvoke('get_tenant_by_slug', { slug }),
  getTenantByDomain: (domain: string): Promise<any> =>
    safeInvoke('get_tenant_by_domain', { domain }),
  getCustomerRegistrationStatusByDomain: (
    domain: string,
  ): Promise<{
    enabled: boolean;
    global_registration_enabled: boolean;
    tenant_self_registration_enabled: boolean;
  }> => safeInvoke('get_customer_registration_status_by_domain', { domain }),
  validateCustomerRegistrationInviteByDomain: (
    token: string,
  ): Promise<CustomerRegistrationInviteValidation> =>
    safeInvoke('validate_customer_registration_invite_by_domain', { token }),
  registerCustomerByDomain: (
    email: string,
    password: string,
    name: string,
    inviteToken?: string | null,
  ): Promise<AuthResponse> =>
    safeInvoke('register_customer_by_domain', {
      email,
      password,
      name,
      inviteToken: inviteToken ?? undefined,
      invite_token: inviteToken ?? undefined,
    }),
};
