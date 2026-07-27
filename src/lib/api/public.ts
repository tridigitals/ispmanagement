import { safeInvoke } from './core';
import type { AuthResponse, CustomerRegistrationInviteValidation } from './types';

const currentDomain = () =>
  typeof window !== 'undefined' ? window.location.host || window.location.hostname : undefined;

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
    safeInvoke('validate_customer_registration_invite_by_domain', {
      invite_token: token,
      domain: currentDomain(),
    }),
  registerCustomerByDomain: (
    email: string,
    password: string,
    name: string,
    inviteToken?: string | null,
    phone?: string | null,
  ): Promise<AuthResponse> =>
    // Body must match PublicCustomerRegisterDto (camelCase, deny_unknown_fields).
    // Do not send domain / invite_token snake_case — BE rejects unknown fields.
    safeInvoke('register_customer_by_domain', {
      email,
      password,
      name,
      phone: phone ?? undefined,
      inviteToken: inviteToken ?? undefined,
    }),
};
