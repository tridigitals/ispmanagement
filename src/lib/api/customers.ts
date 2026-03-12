import { getTokenOrThrow, safeInvoke } from './core';
import type {
  Customer,
  CustomerLocation,
  CustomerPortalCheckoutResponse,
  CustomerPortalInstallationTrackerResponse,
  CustomerPortalOrderRequestResponse,
  CustomerPortalSubscriptionStats,
  CustomerPortalUser,
  CustomerRegistrationInviteCreateResponse,
  CustomerRegistrationInvitePolicy,
  CustomerRegistrationInviteSummary,
  CustomerRegistrationInviteView,
  CustomerSubscription,
  CustomerSubscriptionView,
  IspPackage,
  PaginatedResponse,
} from './types';

export const customers = {
  list: (params?: {
    q?: string;
    page?: number;
    perPage?: number;
  }): Promise<PaginatedResponse<Customer>> =>
    safeInvoke('list_customers', {
      token: getTokenOrThrow(),
      q: params?.q,
      page: params?.page,
      per_page: params?.perPage,
    }),

  get: (customerId: string): Promise<Customer> =>
    safeInvoke('get_customer', {
      token: getTokenOrThrow(),
      customerId,
      customer_id: customerId,
    }),

  create: (dto: {
    name: string;
    email?: string | null;
    phone?: string | null;
    notes?: string | null;
    is_active?: boolean;
  }): Promise<Customer> =>
    safeInvoke('create_customer', {
      token: getTokenOrThrow(),
      ...dto,
    }),

  createWithPortal: (dto: {
    name: string;
    email?: string | null;
    phone?: string | null;
    notes?: string | null;
    is_active?: boolean;
    portal_email: string;
    portal_name?: string | null;
    portal_password: string;
  }): Promise<Customer> =>
    safeInvoke('create_customer_with_portal', {
      token: getTokenOrThrow(),
      ...dto,
    }),

  update: (
    customerId: string,
    dto: {
      name?: string;
      email?: string | null;
      phone?: string | null;
      notes?: string | null;
      is_active?: boolean;
    },
  ): Promise<Customer> =>
    safeInvoke('update_customer', {
      token: getTokenOrThrow(),
      customerId,
      customer_id: customerId,
      ...dto,
    }),

  delete: (customerId: string): Promise<void> =>
    safeInvoke('delete_customer', {
      token: getTokenOrThrow(),
      customerId,
      customer_id: customerId,
    }),

  invites: {
    list: (params?: {
      include_inactive?: boolean;
      limit?: number;
    }): Promise<CustomerRegistrationInviteView[]> =>
      safeInvoke('list_customer_registration_invites', {
        token: getTokenOrThrow(),
        include_inactive: params?.include_inactive,
        includeInactive: params?.include_inactive,
        limit: params?.limit,
      }),
    create: (dto?: {
      expires_in_hours?: number;
      max_uses?: number;
      note?: string | null;
    }): Promise<CustomerRegistrationInviteCreateResponse> =>
      safeInvoke('create_customer_registration_invite', {
        token: getTokenOrThrow(),
        expires_in_hours: dto?.expires_in_hours,
        max_uses: dto?.max_uses,
        note: dto?.note ?? undefined,
      }),
    revoke: (inviteId: string): Promise<void> =>
      safeInvoke('revoke_customer_registration_invite', {
        token: getTokenOrThrow(),
        inviteId,
        invite_id: inviteId,
      }),
    getPolicy: (): Promise<CustomerRegistrationInvitePolicy> =>
      safeInvoke('get_customer_registration_invite_policy', {
        token: getTokenOrThrow(),
      }),
    updatePolicy: (dto: {
      default_expires_in_hours?: number;
      default_max_uses?: number;
    }): Promise<CustomerRegistrationInvitePolicy> =>
      safeInvoke('update_customer_registration_invite_policy', {
        token: getTokenOrThrow(),
        default_expires_in_hours: dto.default_expires_in_hours,
        default_max_uses: dto.default_max_uses,
      }),
    summary: (): Promise<CustomerRegistrationInviteSummary> =>
      safeInvoke('get_customer_registration_invite_summary', {
        token: getTokenOrThrow(),
      }),
  },

  locations: {
    list: (customerId: string): Promise<CustomerLocation[]> =>
      safeInvoke('list_customer_locations', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
      }),
    create: (dto: {
      customer_id: string;
      label: string;
      address_line1?: string | null;
      address_line2?: string | null;
      city?: string | null;
      state?: string | null;
      postal_code?: string | null;
      country?: string | null;
      latitude?: number | null;
      longitude?: number | null;
      notes?: string | null;
    }): Promise<CustomerLocation> =>
      safeInvoke('create_customer_location', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    update: (
      locationId: string,
      dto: Partial<
        Pick<
          CustomerLocation,
          | 'label'
          | 'address_line1'
          | 'address_line2'
          | 'city'
          | 'state'
          | 'postal_code'
          | 'country'
          | 'latitude'
          | 'longitude'
          | 'notes'
        >
      >,
    ): Promise<CustomerLocation> =>
      safeInvoke('update_customer_location', {
        token: getTokenOrThrow(),
        locationId,
        location_id: locationId,
        ...dto,
      }),
    delete: (locationId: string): Promise<void> =>
      safeInvoke('delete_customer_location', {
        token: getTokenOrThrow(),
        locationId,
        location_id: locationId,
      }),
  },

  portalUsers: {
    list: (customerId: string): Promise<CustomerPortalUser[]> =>
      safeInvoke('list_customer_portal_users', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
      }),
    addExisting: (dto: { customer_id: string; user_id: string }): Promise<CustomerPortalUser> =>
      safeInvoke('add_customer_portal_user', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    createNew: (dto: {
      customer_id: string;
      email: string;
      name: string;
      password: string;
    }): Promise<CustomerPortalUser> =>
      safeInvoke('create_customer_portal_user', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    remove: (customerUserId: string): Promise<void> =>
      safeInvoke('remove_customer_portal_user', {
        token: getTokenOrThrow(),
        customerUserId,
        customer_user_id: customerUserId,
      }),
  },

  subscriptions: {
    list: (
      customerId: string,
      params?: { page?: number; per_page?: number },
    ): Promise<PaginatedResponse<CustomerSubscriptionView>> =>
      safeInvoke('list_customer_subscriptions', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
        page: params?.page,
        per_page: params?.per_page,
      }),
    create: (
      customerId: string,
      dto: {
        location_id: string;
        package_id: string;
        router_id?: string | null;
        billing_cycle: 'monthly' | 'yearly' | string;
        price: number;
        currency_code?: string | null;
        status?: 'active' | 'suspended' | 'cancelled' | string;
        starts_at?: string | null;
        ends_at?: string | null;
        notes?: string | null;
      },
    ): Promise<CustomerSubscription> =>
      safeInvoke('create_customer_subscription', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
        ...dto,
      }),
    update: (
      subscriptionId: string,
      dto: {
        location_id?: string;
        package_id?: string;
        router_id?: string | null;
        billing_cycle?: 'monthly' | 'yearly' | string;
        price?: number;
        currency_code?: string | null;
        status?: 'active' | 'suspended' | 'cancelled' | string;
        starts_at?: string | null;
        ends_at?: string | null;
        notes?: string | null;
      },
    ): Promise<CustomerSubscription> =>
      safeInvoke('update_customer_subscription', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
        ...dto,
      }),
    delete: (subscriptionId: string): Promise<void> =>
      safeInvoke('delete_customer_subscription', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
      }),
  },

  portal: {
    myLocations: (): Promise<CustomerLocation[]> =>
      safeInvoke('list_my_customer_locations', { token: getTokenOrThrow() }),
    createMyLocation: (dto: {
      label: string;
      address_line1?: string | null;
      address_line2?: string | null;
      city?: string | null;
      state?: string | null;
      postal_code?: string | null;
      country?: string | null;
      latitude?: number | null;
      longitude?: number | null;
      notes?: string | null;
    }): Promise<CustomerLocation> =>
      safeInvoke('create_my_customer_location', { token: getTokenOrThrow(), ...dto }),
    updateMyLocation: (
      locationId: string,
      dto: Partial<
        Pick<
          CustomerLocation,
          | 'label'
          | 'address_line1'
          | 'address_line2'
          | 'city'
          | 'state'
          | 'postal_code'
          | 'country'
          | 'latitude'
          | 'longitude'
          | 'notes'
        >
      >,
    ): Promise<CustomerLocation> =>
      safeInvoke('update_my_customer_location', {
        token: getTokenOrThrow(),
        locationId,
        location_id: locationId,
        ...dto,
      }),
    deleteMyLocation: (locationId: string): Promise<void> =>
      safeInvoke('delete_my_customer_location', {
        token: getTokenOrThrow(),
        locationId,
        location_id: locationId,
      }),
    myPackages: (): Promise<IspPackage[]> =>
      safeInvoke('list_my_customer_packages', { token: getTokenOrThrow() }),
    mySubscriptionStats: (): Promise<CustomerPortalSubscriptionStats> =>
      safeInvoke('get_my_customer_subscription_stats', { token: getTokenOrThrow() }),
    mySubscriptions: (params?: {
      page?: number;
      per_page?: number;
      status?: 'active' | 'pending_installation' | 'suspended' | 'cancelled' | 'needs_attention';
      sort_by?: 'updated_at' | 'price' | 'status' | 'package_name' | 'location_label';
      sort_dir?: 'asc' | 'desc';
    }): Promise<PaginatedResponse<CustomerSubscriptionView>> =>
      safeInvoke('list_my_customer_subscriptions', {
        token: getTokenOrThrow(),
        page: params?.page,
        per_page: params?.per_page,
        status: params?.status,
        sort_by: params?.sort_by,
        sort_dir: params?.sort_dir,
      }),
    checkout: (dto: {
      location_id: string;
      package_id: string;
      billing_cycle: 'monthly' | 'yearly' | string;
    }): Promise<CustomerPortalCheckoutResponse> =>
      safeInvoke('create_my_customer_subscription_invoice', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    orderRequest: (dto: {
      location_id: string;
      package_id: string;
      billing_cycle: 'monthly' | 'yearly' | string;
    }): Promise<CustomerPortalOrderRequestResponse> =>
      safeInvoke('create_my_customer_subscription_order_request', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    reopenOrderRequest: (
      subscriptionId: string,
      dto?: { notes?: string },
    ): Promise<CustomerPortalOrderRequestResponse> =>
      safeInvoke('reopen_my_customer_subscription_order_request', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
        notes: dto?.notes,
      }),
    installationTracker: (
      subscriptionId: string,
    ): Promise<CustomerPortalInstallationTrackerResponse> =>
      safeInvoke('get_my_customer_subscription_installation_tracker', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
      }),
    requestReschedule: (
      subscriptionId: string,
      dto: { scheduled_at: string; reason?: string },
    ): Promise<CustomerPortalOrderRequestResponse> =>
      safeInvoke('request_my_customer_subscription_reschedule', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
        ...dto,
      }),
  },
};
