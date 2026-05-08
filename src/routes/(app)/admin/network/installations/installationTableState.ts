import type { InstallationWorkOrderView } from '$lib/api/types';

export type InstallationAssignmentFilter = 'all' | 'assigned' | 'unassigned';
export type InstallationSortKey = 'updated_at' | 'scheduled_at' | 'customer_name' | 'assigned_to_name' | 'status';

type FilterParams = {
  search: string;
  statusFilter: string;
  assignmentFilter: InstallationAssignmentFilter;
  assigneeUserId: string;
  sortKey: InstallationSortKey;
  sortDirection: 'asc' | 'desc';
};

export function buildInstallationStats(rows: InstallationWorkOrderView[]) {
  return {
    total: rows.length,
    pending: rows.filter((r) => r.status === 'pending').length,
    inProgress: rows.filter((r) => r.status === 'in_progress').length,
    completed: rows.filter((r) => r.status === 'completed').length,
    cancelled: rows.filter((r) => r.status === 'cancelled').length,
  };
}

function compareNullableStrings(a?: string | null, b?: string | null) {
  const left = `${a || ''}`.trim().toLowerCase();
  const right = `${b || ''}`.trim().toLowerCase();
  if (!left && !right) return 0;
  if (!left) return 1;
  if (!right) return -1;
  return left.localeCompare(right);
}

function compareNullableDates(a?: string | null, b?: string | null) {
  const left = a ? Date.parse(a) : Number.NEGATIVE_INFINITY;
  const right = b ? Date.parse(b) : Number.NEGATIVE_INFINITY;
  return left - right;
}

export function filterAndSortInstallationRows(
  rows: InstallationWorkOrderView[],
  params: FilterParams,
): InstallationWorkOrderView[] {
  const q = params.search.trim().toLowerCase();

  const filtered = rows.filter((row) => {
    if (params.statusFilter !== 'all' && row.status !== params.statusFilter) return false;
    if (params.assignmentFilter === 'assigned' && !row.assigned_to) return false;
    if (params.assignmentFilter === 'unassigned' && row.assigned_to) return false;
    if (params.assigneeUserId && row.assigned_to !== params.assigneeUserId) return false;
    if (!q) return true;

    const hay = [
      row.customer_name,
      row.customer_id,
      row.location_label,
      row.package_name,
      row.assigned_to_name,
      row.assigned_to_email,
      row.router_name,
      row.selected_zone_name,
      row.selected_node_name,
      row.status,
      row.subscription_status,
    ]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();

    return hay.includes(q);
  });

  const sorted = [...filtered].sort((left, right) => {
    let result = 0;

    switch (params.sortKey) {
      case 'customer_name':
        result = compareNullableStrings(left.customer_name, right.customer_name);
        break;
      case 'assigned_to_name':
        result = compareNullableStrings(left.assigned_to_name, right.assigned_to_name);
        break;
      case 'scheduled_at':
        result = compareNullableDates(left.scheduled_at, right.scheduled_at);
        break;
      case 'status':
        result = compareNullableStrings(left.status, right.status);
        break;
      case 'updated_at':
      default:
        result = compareNullableDates(left.updated_at, right.updated_at);
        break;
    }

    if (result === 0) {
      result = compareNullableDates(left.updated_at, right.updated_at);
    }

    return params.sortDirection === 'asc' ? result : -result;
  });

  return sorted;
}
