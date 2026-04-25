export type PermissionChecker = (action: string, resource: string) => boolean;

export function canAccessNetworkMap(can: PermissionChecker): boolean {
  return (
    can('read', 'network_topology') ||
    can('manage', 'network_topology') ||
    can('read', 'router_inventory') ||
    can('manage', 'router_inventory') ||
    can('read', 'work_orders') ||
    can('manage', 'work_orders')
  );
}
