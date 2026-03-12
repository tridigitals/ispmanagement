import { getTokenOrThrow, safeInvoke } from './core';
import type { PaginatedResponse, User, UserAddress } from './types';

export const users = {
  list: (page?: number, perPage?: number): Promise<PaginatedResponse<User>> =>
    safeInvoke('list_users', { token: getTokenOrThrow(), page, perPage }),

  get: (id: string): Promise<User> => safeInvoke('get_user', { token: getTokenOrThrow(), id }),

  create: (email: string, password: string, name: string): Promise<User> =>
    safeInvoke('create_user', { token: getTokenOrThrow(), email, password, name }),

  update: (
    id: string,
    data: {
      email?: string;
      name?: string;
      role?: string;
      isActive?: boolean;
    },
  ): Promise<User> =>
    safeInvoke('update_user', {
      token: getTokenOrThrow(),
      id,
      email: data.email,
      name: data.name,
      role: data.role,
      isActive: data.isActive,
      is_active: data.isActive,
    }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_user', { token: getTokenOrThrow(), id }),

  listMyAddresses: (): Promise<UserAddress[]> =>
    safeInvoke('list_my_addresses', { token: getTokenOrThrow() }),

  createMyAddress: (data: {
    label?: string;
    recipientName?: string;
    phone?: string;
    line1: string;
    line2?: string;
    city?: string;
    state?: string;
    postalCode?: string;
    countryCode?: string;
    isDefaultShipping?: boolean;
    isDefaultBilling?: boolean;
  }): Promise<UserAddress> =>
    safeInvoke('create_my_address', {
      token: getTokenOrThrow(),
      label: data.label,
      recipientName: data.recipientName,
      recipient_name: data.recipientName,
      phone: data.phone,
      line1: data.line1,
      line2: data.line2,
      city: data.city,
      state: data.state,
      postalCode: data.postalCode,
      postal_code: data.postalCode,
      countryCode: data.countryCode,
      country_code: data.countryCode,
      isDefaultShipping: data.isDefaultShipping,
      is_default_shipping: data.isDefaultShipping,
      isDefaultBilling: data.isDefaultBilling,
      is_default_billing: data.isDefaultBilling,
    }),

  updateMyAddress: (
    addressId: string,
    data: {
      label?: string;
      recipientName?: string;
      phone?: string;
      line1?: string;
      line2?: string;
      city?: string;
      state?: string;
      postalCode?: string;
      countryCode?: string;
      isDefaultShipping?: boolean;
      isDefaultBilling?: boolean;
    },
  ): Promise<UserAddress> =>
    safeInvoke('update_my_address', {
      token: getTokenOrThrow(),
      addressId,
      address_id: addressId,
      label: data.label,
      recipientName: data.recipientName,
      recipient_name: data.recipientName,
      phone: data.phone,
      line1: data.line1,
      line2: data.line2,
      city: data.city,
      state: data.state,
      postalCode: data.postalCode,
      postal_code: data.postalCode,
      countryCode: data.countryCode,
      country_code: data.countryCode,
      isDefaultShipping: data.isDefaultShipping,
      is_default_shipping: data.isDefaultShipping,
      isDefaultBilling: data.isDefaultBilling,
      is_default_billing: data.isDefaultBilling,
    }),

  deleteMyAddress: (addressId: string): Promise<void> =>
    safeInvoke('delete_my_address', {
      token: getTokenOrThrow(),
      addressId,
      address_id: addressId,
    }),
};
