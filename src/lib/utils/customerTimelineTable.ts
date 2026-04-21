import type { AuditLog } from '$lib/api/client';

export type CustomerTimelineRow = {
  id: string;
  created_at: string;
  action: string;
  resource: string;
  actor: string;
  details: string;
};

export function getCustomerTimelineActionLabel(action: string): string {
  const map: Record<string, string> = {
    CUSTOMER_CREATE: 'Customer created',
    CUSTOMER_UPDATE: 'Customer updated',
    CUSTOMER_DELETE: 'Customer deleted',
    CUSTOMER_LOCATION_CREATE: 'Location added',
    CUSTOMER_LOCATION_UPDATE: 'Location updated',
    CUSTOMER_LOCATION_DELETE: 'Location deleted',
    CUSTOMER_SUBSCRIPTION_CREATE: 'Subscription created',
    CUSTOMER_SUBSCRIPTION_UPDATE: 'Subscription updated',
    CUSTOMER_SUBSCRIPTION_DELETE: 'Subscription deleted',
    CUSTOMER_PORTAL_USER_CREATE: 'Portal user created',
    CUSTOMER_PORTAL_USER_ADD: 'Portal user linked',
    CUSTOMER_PORTAL_USER_REMOVE: 'Portal user removed',
  };
  return (
    map[action] ||
    action
      .replaceAll('_', ' ')
      .toLowerCase()
      .replace(/^./, (m) => m.toUpperCase())
  );
}

export function getCustomerTimelineResourceLabel(resource: string): string {
  const map: Record<string, string> = {
    customers: 'Customer',
    customer_locations: 'Location',
    customer_subscriptions: 'Subscription',
    customer_users: 'Portal user',
  };
  return map[resource] || resource;
}

export function getCustomerTimelineActorLabel(log: AuditLog): string {
  return log.user_name || log.user_email || 'System';
}

export function buildCustomerTimelineRows(logs: AuditLog[]): CustomerTimelineRow[] {
  return logs.map((log) => ({
    id: log.id,
    created_at: log.created_at,
    action: getCustomerTimelineActionLabel(log.action),
    resource: getCustomerTimelineResourceLabel(log.resource),
    actor: getCustomerTimelineActorLabel(log),
    details: log.details || '',
  }));
}
