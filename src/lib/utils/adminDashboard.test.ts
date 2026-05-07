import { describe, expect, it } from 'vitest';

import {
  buildAdminDashboardModel,
  getAdminDashboardAudience,
  getAdminDashboardDataRequirements,
  summarizeAlerts,
  summarizeIncidents,
  summarizeInvoices,
  summarizePppoeAccounts,
  summarizeWorkOrders,
} from './adminDashboard';

describe('admin dashboard helpers', () => {
  it('fetches only the domains needed for an operations-focused technician dashboard', () => {
    expect(
      getAdminDashboardDataRequirements({
        customersRead: true,
        workOrdersRead: true,
        pppoeRead: true,
      }),
    ).toEqual({
      team: false,
      settings: false,
      subscription: false,
      customers: true,
      lifecycle: true,
      invoices: false,
      workOrders: true,
      pppoe: true,
      alerts: false,
      incidents: false,
      routers: false,
      support: false,
    });
  });

  it('classifies owner/admin dashboards separately from technician and noc flows', () => {
    expect(
      getAdminDashboardAudience({
        teamRead: true,
        settingsRead: true,
        customersRead: true,
        billingRead: true,
      }),
    ).toBe('admin');

    expect(
      getAdminDashboardAudience({
        customersRead: true,
        workOrdersRead: true,
        pppoeRead: true,
      }),
    ).toBe('operations');

    expect(
      getAdminDashboardAudience({
        networkNocRead: true,
        networkAlertsRead: true,
        networkIncidentsRead: true,
        routerInventoryRead: true,
      }),
    ).toBe('noc');
  });

  it('keeps technician primary stats operational and avoids admin-only emphasis', () => {
    const model = buildAdminDashboardModel({
      tenantPrefix: '/demo',
      capabilities: {
        customersRead: true,
        workOrdersRead: true,
        pppoeRead: true,
      },
      summary: {
        customerTotal: 48,
        workOrders: summarizeWorkOrders([
          { status: 'pending' },
          { status: 'in_progress' },
          { status: 'completed' },
        ]),
        pppoe: summarizePppoeAccounts([
          { disabled: false, router_present: true, is_provisioned: true },
          { disabled: true, router_present: false, is_provisioned: true },
        ]),
      },
    });

    expect(model.primaryStats.map((card) => card.id)).toEqual([
      'work_orders_active',
      'pppoe_accounts',
      'customers',
    ]);
    expect(model.primaryStats.map((card) => card.id)).not.toContain('team_members');
    expect(model.quickActions.map((item) => item.id)).toEqual([
      'installations',
      'pppoe',
      'customers',
    ]);
    expect(model.quickActions.map((item) => item.href)).toEqual([
      '/admin/network/installations',
      '/admin/network/pppoe',
      '/admin/customers',
    ]);
  });

  it('treats unprovisioned native radius accounts as pending provisioning, not external sync drift', () => {
    expect(
      summarizePppoeAccounts([
        { disabled: false, router_present: true, is_provisioned: true },
        { disabled: false, router_present: true, is_provisioned: false },
      ]),
    ).toMatchObject({
      total: 2,
      radiusMissing: 1,
    });
  });

  it('gives admin users a broad business snapshot and billing trend', () => {
    const model = buildAdminDashboardModel({
      tenantPrefix: '/demo',
      capabilities: {
        teamRead: true,
        rolesRead: true,
        settingsRead: true,
        customersRead: true,
        billingRead: true,
      },
      summary: {
        teamMembers: 12,
        settingsCount: 37,
        customerTotal: 240,
        invoice: summarizeInvoices([
          { status: 'paid', amount: 100_000, due_date: '2026-04-10' },
          { status: 'pending', amount: 150_000, due_date: '2026-04-01' },
          { status: 'overdue', amount: 200_000, due_date: '2026-03-28' },
        ]),
        subscription: {
          plan_name: 'Scale',
          plan_slug: 'scale',
          status: 'active',
          current_period_end: null,
          storage_usage: 5_000,
          storage_limit: 10_000,
          member_usage: 12,
          member_limit: 25,
        },
      },
    });

    expect(model.primaryStats.map((card) => card.id)).toEqual([
      'customers',
      'invoices_due',
      'team_members',
      'subscription_plan',
    ]);
    expect(model.quickActions.map((item) => item.id)).toContain('roles');
    expect(model.quickActions.find((item) => item.id === 'roles')?.href).toBe('/admin/roles');
    expect(model.trendCards.map((item) => item.id)).toContain('invoice_status');
  });

  it('shows network-first cards and trend strips for noc capability sets', () => {
    const model = buildAdminDashboardModel({
      tenantPrefix: '/demo',
      capabilities: {
        networkNocRead: true,
        networkAlertsRead: true,
        networkIncidentsRead: true,
        routerInventoryRead: true,
      },
      summary: {
        routersTotal: 18,
        alerts: summarizeAlerts([
          { severity: 'warning' },
          { severity: 'critical' },
          { severity: 'critical' },
        ]),
        incidents: summarizeIncidents([
          { severity: 'warning' },
          { severity: 'critical' },
          { severity: 'critical' },
        ]),
      },
    });

    expect(model.primaryStats.map((card) => card.id)).toEqual([
      'incidents_open',
      'alerts_active',
      'routers_monitored',
    ]);
    expect(model.quickActions.map((item) => item.id)).toEqual([
      'noc',
      'alerts',
      'incidents',
      'routers',
    ]);
    expect(model.quickActions.map((item) => item.href)).toEqual([
      '/admin/network/noc',
      '/admin/network/alerts',
      '/admin/network/incidents',
      '/admin/network/routers',
    ]);
    expect(model.trendCards.map((item) => item.id)).toEqual([
      'incident_severity',
      'alert_severity',
    ]);
  });
});
