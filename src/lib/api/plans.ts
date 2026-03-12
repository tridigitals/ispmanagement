import { getTokenOrThrow, safeInvoke } from './core';
import type { TenantSubscriptionDetails } from './types';

export const plans = {
  list: (): Promise<any[]> => safeInvoke('list_plans', { token: getTokenOrThrow() }),

  get: (planId: string): Promise<any> =>
    safeInvoke('get_plan', { token: getTokenOrThrow(), planId }),

  create: (
    name: string,
    slug: string,
    description?: string,
    price_monthly?: number,
    price_yearly?: number,
    is_active?: boolean,
    is_default?: boolean,
    sort_order?: number,
  ): Promise<any> =>
    safeInvoke('create_plan', {
      token: getTokenOrThrow(),
      name,
      slug,
      description,
      priceMonthly: price_monthly,
      priceYearly: price_yearly,
      isActive: is_active,
      isDefault: is_default,
      sortOrder: sort_order,
      price_monthly,
      price_yearly,
      is_active,
      is_default,
      sort_order,
    }),

  update: (
    planId: string,
    name?: string,
    slug?: string,
    description?: string,
    price_monthly?: number,
    price_yearly?: number,
    is_active?: boolean,
    is_default?: boolean,
    sort_order?: number,
  ): Promise<any> =>
    safeInvoke('update_plan', {
      token: getTokenOrThrow(),
      planId,
      name,
      slug,
      description,
      priceMonthly: price_monthly,
      priceYearly: price_yearly,
      isActive: is_active,
      isDefault: is_default,
      sortOrder: sort_order,
      price_monthly,
      price_yearly,
      is_active,
      is_default,
      sort_order,
    }),

  delete: (planId: string): Promise<void> =>
    safeInvoke('delete_plan', { token: getTokenOrThrow(), planId }),

  listFeatures: (): Promise<any[]> => safeInvoke('list_features', { token: getTokenOrThrow() }),

  createFeature: (
    code: string,
    name: string,
    description?: string,
    value_type?: string,
    category?: string,
    default_value?: string,
    sort_order?: number,
  ): Promise<any> =>
    safeInvoke('create_feature', {
      token: getTokenOrThrow(),
      code,
      name,
      description,
      value_type,
      category,
      default_value,
      sort_order,
    }),

  deleteFeature: (featureId: string): Promise<void> =>
    safeInvoke('delete_feature', { token: getTokenOrThrow(), featureId }),

  setPlanFeature: (planId: string, featureId: string, value: string): Promise<void> =>
    safeInvoke('set_plan_feature', { token: getTokenOrThrow(), planId, featureId, value }),

  getSubscription: (tenantId: string): Promise<any> =>
    safeInvoke('get_tenant_subscription', { token: getTokenOrThrow(), tenantId }),

  getSubscriptionDetails: (tenantId?: string): Promise<TenantSubscriptionDetails> =>
    safeInvoke('get_tenant_subscription_details', { token: getTokenOrThrow(), tenantId }),

  assignPlan: (tenantId: string, planId: string): Promise<any> =>
    safeInvoke('assign_plan_to_tenant', { token: getTokenOrThrow(), tenantId, planId }),

  checkAccess: (tenantId: string, featureCode: string): Promise<any> =>
    safeInvoke('check_feature_access', { token: getTokenOrThrow(), tenantId, featureCode }),
};
