import { describe, expect, it } from 'vitest';

import type { InstallationWorkOrderView } from '$lib/api/types';

import {
  buildInstallationStats,
  filterAndSortInstallationRows,
  type InstallationAssignmentFilter,
  type InstallationSortKey,
} from './installationTableState';

function row(overrides: Partial<InstallationWorkOrderView>): InstallationWorkOrderView {
  return {
    id: 'wo-1',
    tenant_id: 't-1',
    subscription_id: 'sub-1',
    invoice_id: null,
    customer_id: 'cust-1',
    location_id: 'loc-1',
    package_id: 'pkg-1',
    router_id: null,
    status: 'pending',
    assigned_to: null,
    scheduled_at: null,
    completed_at: null,
    notes: null,
    created_at: '2026-05-08T01:00:00Z',
    updated_at: '2026-05-08T02:00:00Z',
    customer_name: 'Alpha',
    location_label: 'Site A',
    package_name: 'Package A',
    package_provisioning_type: 'pppoe',
    router_name: null,
    assigned_to_name: null,
    assigned_to_email: null,
    assignment_id: null,
    assignment_status: null,
    subscription_status: 'pending_installation',
    subscription_starts_at: null,
    subscription_grace_until: null,
    has_customer_package_invoice: false,
    selected_zone_id: null,
    selected_zone_name: null,
    selected_node_id: null,
    selected_node_name: null,
    selected_node_score: null,
    path_node_ids: null,
    path_link_ids: null,
    ...overrides,
  };
}

describe('installationTableState', () => {
  const rows = [
    row({ id: 'wo-a', customer_name: 'Bravo', status: 'completed', assigned_to_name: 'Tech B', assigned_to: 'u2', updated_at: '2026-05-08T05:00:00Z' }),
    row({ id: 'wo-b', customer_name: 'Alpha', status: 'pending', assigned_to_name: null, assigned_to: null, updated_at: '2026-05-08T03:00:00Z' }),
    row({ id: 'wo-c', customer_name: 'Charlie', status: 'in_progress', assigned_to_name: 'Tech A', assigned_to: 'u1', updated_at: '2026-05-08T04:00:00Z', scheduled_at: '2026-05-09T01:00:00Z' }),
    row({ id: 'wo-d', customer_name: 'Delta', status: 'cancelled', assigned_to_name: 'Tech C', assigned_to: 'u3', updated_at: '2026-05-08T01:00:00Z' }),
  ];

  it('builds status stats including cancelled', () => {
    expect(buildInstallationStats(rows)).toEqual({
      total: 4,
      pending: 1,
      inProgress: 1,
      completed: 1,
      cancelled: 1,
    });
  });

  it('filters by search, status and assignment', () => {
    const filtered = filterAndSortInstallationRows(rows, {
      search: 'char',
      statusFilter: 'in_progress',
      assignmentFilter: 'assigned',
      assigneeUserId: '',
      sortKey: 'updated_at',
      sortDirection: 'desc',
    });

    expect(filtered.map((item) => item.id)).toEqual(['wo-c']);
  });

  it('sorts by customer name ascending', () => {
    const sortKey: InstallationSortKey = 'customer_name';
    const assignmentFilter: InstallationAssignmentFilter = 'all';
    const filtered = filterAndSortInstallationRows(rows, {
      search: '',
      statusFilter: 'all',
      assignmentFilter,
      assigneeUserId: '',
      sortKey,
      sortDirection: 'asc',
    });

    expect(filtered.map((item) => item.customer_name)).toEqual(['Alpha', 'Bravo', 'Charlie', 'Delta']);
  });

  it('sorts unassigned rows last when sorting assignee ascending', () => {
    const filtered = filterAndSortInstallationRows(rows, {
      search: '',
      statusFilter: 'all',
      assignmentFilter: 'all',
      assigneeUserId: '',
      sortKey: 'assigned_to_name',
      sortDirection: 'asc',
    });

    expect(filtered.map((item) => item.id)).toEqual(['wo-c', 'wo-a', 'wo-d', 'wo-b']);
  });

  it('filters by specific assignee user id', () => {
    const filtered = filterAndSortInstallationRows(rows, {
      search: '',
      statusFilter: 'all',
      assignmentFilter: 'all',
      assigneeUserId: 'u2',
      sortKey: 'updated_at',
      sortDirection: 'desc',
    });

    expect(filtered.map((item) => item.id)).toEqual(['wo-a']);
  });
});
