import type { SupportTicketStats, TenantSubscriptionDetails } from '$lib/api/types';
import { canonicalTenantPath } from './tenantRouting';

export type AdminDashboardAudience = 'admin' | 'operations' | 'support' | 'noc' | 'hybrid';

export type AdminDashboardCapabilities = {
  teamRead?: boolean;
  rolesRead?: boolean;
  settingsRead?: boolean;
  customersRead?: boolean;
  billingRead?: boolean;
  workOrdersRead?: boolean;
  pppoeRead?: boolean;
  supportReadAll?: boolean;
  networkNocRead?: boolean;
  networkAlertsRead?: boolean;
  networkIncidentsRead?: boolean;
  routerInventoryRead?: boolean;
  auditLogsRead?: boolean;
  emailOutboxRead?: boolean;
};

export type AdminDashboardRequirements = {
  team: boolean;
  settings: boolean;
  subscription: boolean;
  customers: boolean;
  lifecycle: boolean;
  invoices: boolean;
  workOrders: boolean;
  pppoe: boolean;
  alerts: boolean;
  incidents: boolean;
  routers: boolean;
  support: boolean;
};

export type MetricChip = {
  value: string | number;
  labelKey: string;
  fallbackLabel: string;
};

export type AdminDashboardCard = {
  id: string;
  titleKey: string;
  fallbackTitle: string;
  value: string | number;
  href: string;
  icon: string;
  tone: 'emerald' | 'amber' | 'cyan' | 'indigo' | 'rose' | 'slate';
  meta?: MetricChip;
  badge?: string;
};

export type AdminDashboardFocusCard = {
  id: string;
  titleKey: string;
  fallbackTitle: string;
  descriptionKey: string;
  fallbackDescription: string;
  value: string | number;
  href: string;
  icon: string;
  tone: 'emerald' | 'amber' | 'cyan' | 'indigo' | 'rose' | 'slate';
};

export type AdminDashboardAction = {
  id: string;
  titleKey: string;
  fallbackTitle: string;
  descriptionKey: string;
  fallbackDescription: string;
  href: string;
  icon: string;
  tone: 'emerald' | 'amber' | 'cyan' | 'indigo' | 'rose' | 'slate';
};

export type AdminDashboardTrend = {
  id: string;
  titleKey: string;
  fallbackTitle: string;
  href: string;
  items: Array<{
    id: string;
    value: number;
    labelKey: string;
    fallbackLabel: string;
    tone: 'emerald' | 'amber' | 'cyan' | 'indigo' | 'rose' | 'slate';
  }>;
};

export type InvoiceSummary = {
  total: number;
  unpaid: number;
  overdue: number;
  paid: number;
  outstandingAmount: number;
};

export type WorkOrderSummary = {
  total: number;
  active: number;
  pending: number;
  inProgress: number;
  completed: number;
  cancelled: number;
};

export type PppoeSummary = {
  total: number;
  disabled: number;
  routerMissing: number;
  radiusMissing: number;
};

export type AlertSummary = {
  total: number;
  critical: number;
  warning: number;
};

export type IncidentSummary = {
  total: number;
  critical: number;
  warning: number;
};

export type AdminDashboardSummary = {
  teamMembers?: number;
  settingsCount?: number;
  customerTotal?: number;
  activationsWaiting?: number;
  subscription?: TenantSubscriptionDetails | null;
  invoice?: InvoiceSummary;
  workOrders?: WorkOrderSummary;
  pppoe?: PppoeSummary;
  alerts?: AlertSummary;
  incidents?: IncidentSummary;
  routersTotal?: number;
  support?: SupportTicketStats | null;
};

export function getAdminDashboardDataRequirements(
  capabilities: AdminDashboardCapabilities,
): AdminDashboardRequirements {
  const customerVisible = !!capabilities.customersRead;
  const billingVisible = !!capabilities.billingRead;
  const workOrdersVisible = !!capabilities.workOrdersRead;
  const pppoeVisible = !!capabilities.pppoeRead;
  const alertsVisible = !!capabilities.networkAlertsRead;
  const incidentsVisible = !!capabilities.networkIncidentsRead;
  const routersVisible = !!capabilities.routerInventoryRead || !!capabilities.networkNocRead;

  return {
    team: !!capabilities.teamRead,
    settings: false,
    subscription: billingVisible,
    customers: customerVisible,
    lifecycle: customerVisible || billingVisible || workOrdersVisible,
    invoices: billingVisible,
    workOrders: workOrdersVisible,
    pppoe: pppoeVisible,
    alerts: alertsVisible,
    incidents: incidentsVisible,
    routers: routersVisible,
    support: !!capabilities.supportReadAll,
  };
}

export function getAdminDashboardAudience(
  capabilities: AdminDashboardCapabilities,
): AdminDashboardAudience {
  const adminCluster = !!(
    capabilities.teamRead ||
    capabilities.rolesRead ||
    capabilities.settingsRead ||
    capabilities.auditLogsRead ||
    capabilities.emailOutboxRead
  );
  const networkCluster = !!(
    capabilities.networkNocRead ||
    capabilities.networkAlertsRead ||
    capabilities.networkIncidentsRead ||
    capabilities.routerInventoryRead
  );
  const opsCluster = !!(capabilities.workOrdersRead || capabilities.pppoeRead);
  const supportCluster = !!(
    capabilities.customersRead ||
    capabilities.billingRead ||
    capabilities.supportReadAll
  );

  if (adminCluster) return 'admin';
  if (networkCluster && !supportCluster && !opsCluster) return 'noc';
  if (opsCluster && !adminCluster) return 'operations';
  if (supportCluster && !networkCluster && !opsCluster) return 'support';
  return 'hybrid';
}

export function summarizeInvoices(
  invoices: Array<{ status?: string | null; amount?: number | null; due_date?: string | null }>,
): InvoiceSummary {
  const now = Date.now();
  let unpaid = 0;
  let overdue = 0;
  let paid = 0;
  let outstandingAmount = 0;

  for (const invoice of invoices) {
    const status = String(invoice.status || '').toLowerCase();
    const amount = Number(invoice.amount || 0);
    const dueDate = invoice.due_date ? Date.parse(invoice.due_date) : Number.NaN;

    if (status === 'paid') {
      paid += 1;
      continue;
    }

    if (status === 'overdue' || (!Number.isNaN(dueDate) && dueDate < now)) {
      overdue += 1;
    } else {
      unpaid += 1;
    }

    outstandingAmount += amount;
  }

  return {
    total: invoices.length,
    unpaid,
    overdue,
    paid,
    outstandingAmount,
  };
}

export function summarizeWorkOrders(
  workOrders: Array<{ status?: string | null }>,
): WorkOrderSummary {
  const summary: WorkOrderSummary = {
    total: workOrders.length,
    active: 0,
    pending: 0,
    inProgress: 0,
    completed: 0,
    cancelled: 0,
  };

  for (const workOrder of workOrders) {
    const status = String(workOrder.status || '').toLowerCase();
    if (status === 'pending') {
      summary.pending += 1;
      summary.active += 1;
    } else if (status === 'in_progress') {
      summary.inProgress += 1;
      summary.active += 1;
    } else if (status === 'completed') {
      summary.completed += 1;
    } else if (status === 'cancelled') {
      summary.cancelled += 1;
    }
  }

  return summary;
}

export function summarizePppoeAccounts(
  accounts: Array<{
    disabled?: boolean | null;
    router_present?: boolean | null;
    is_provisioned?: boolean | null;
  }>,
): PppoeSummary {
  return {
    total: accounts.length,
    disabled: accounts.filter((account) => !!account.disabled).length,
    routerMissing: accounts.filter((account) => account.router_present === false).length,
    radiusMissing: accounts.filter((account) => account.is_provisioned === false).length,
  };
}

export function summarizeAlerts(alerts: Array<{ severity?: string | null }>): AlertSummary {
  return {
    total: alerts.length,
    critical: alerts.filter((item) => String(item.severity || '').toLowerCase() === 'critical')
      .length,
    warning: alerts.filter((item) => String(item.severity || '').toLowerCase() === 'warning')
      .length,
  };
}

export function summarizeIncidents(
  incidents: Array<{ severity?: string | null }>,
): IncidentSummary {
  return {
    total: incidents.length,
    critical: incidents.filter((item) => String(item.severity || '').toLowerCase() === 'critical')
      .length,
    warning: incidents.filter((item) => String(item.severity || '').toLowerCase() === 'warning')
      .length,
  };
}

type BuildDashboardInput = {
  tenantPrefix: string;
  capabilities: AdminDashboardCapabilities;
  summary: AdminDashboardSummary;
};

export function buildAdminDashboardModel({
  tenantPrefix: _tenantPrefix,
  capabilities,
  summary,
}: BuildDashboardInput): {
  audience: AdminDashboardAudience;
  primaryStats: AdminDashboardCard[];
  focusCards: AdminDashboardFocusCard[];
  quickActions: AdminDashboardAction[];
  trendCards: AdminDashboardTrend[];
} {
  const audience = getAdminDashboardAudience(capabilities);
  const routes = {
    team: canonicalTenantPath('/admin/team'),
    roles: canonicalTenantPath('/admin/roles'),
    settings: canonicalTenantPath('/admin/settings'),
    customers: canonicalTenantPath('/admin/customers'),
    invoices: canonicalTenantPath('/admin/invoices'),
    subscription: canonicalTenantPath('/admin/subscription'),
    installations: canonicalTenantPath('/admin/network/installations'),
    pppoe: canonicalTenantPath('/admin/network/pppoe'),
    noc: canonicalTenantPath('/admin/network/noc'),
    alerts: canonicalTenantPath('/admin/network/alerts'),
    incidents: canonicalTenantPath('/admin/network/incidents'),
    routers: canonicalTenantPath('/admin/network/routers'),
    support: canonicalTenantPath('/admin/support'),
    audit: canonicalTenantPath('/admin/audit-logs'),
    emailOutbox: canonicalTenantPath('/admin/email-outbox'),
  };

  const registry: Record<string, AdminDashboardCard> = {
    customers: {
      id: 'customers',
      titleKey: 'admin.dashboard.widgets.customers.title',
      fallbackTitle: 'Customers',
      value: summary.customerTotal ?? 0,
      href: routes.customers,
      icon: 'users',
      tone: 'emerald',
      meta: {
        value: summary.activationsWaiting ?? 0,
        labelKey: 'admin.dashboard.widgets.customers.meta',
        fallbackLabel: 'awaiting activation',
      },
    },
    invoices_due: {
      id: 'invoices_due',
      titleKey: 'admin.dashboard.widgets.invoices_due.title',
      fallbackTitle: 'Invoices needing follow-up',
      value: (summary.invoice?.unpaid ?? 0) + (summary.invoice?.overdue ?? 0),
      href: routes.invoices,
      icon: 'credit-card',
      tone: 'indigo',
      meta: {
        value: summary.invoice?.overdue ?? 0,
        labelKey: 'admin.dashboard.widgets.invoices_due.meta',
        fallbackLabel: 'overdue',
      },
    },
    team_members: {
      id: 'team_members',
      titleKey: 'admin.dashboard.widgets.team_members.title',
      fallbackTitle: 'Team members',
      value: summary.teamMembers ?? 0,
      href: routes.team,
      icon: 'users',
      tone: 'cyan',
    },
    subscription_plan: {
      id: 'subscription_plan',
      titleKey: 'admin.dashboard.widgets.subscription_plan.title',
      fallbackTitle: 'Subscription plan',
      value: summary.subscription?.plan_name || 'Free',
      href: routes.subscription,
      icon: 'credit-card',
      tone: 'slate',
      badge: summary.subscription?.status || 'inactive',
      meta:
        summary.subscription?.member_limit != null
          ? {
              value: `${summary.subscription.member_usage}/${summary.subscription.member_limit}`,
              labelKey: 'admin.dashboard.widgets.subscription_plan.meta',
              fallbackLabel: 'members used',
            }
          : undefined,
    },
    work_orders_active: {
      id: 'work_orders_active',
      titleKey: 'admin.dashboard.widgets.work_orders_active.title',
      fallbackTitle: 'Active installations',
      value: summary.workOrders?.active ?? 0,
      href: routes.installations,
      icon: 'activity',
      tone: 'emerald',
      meta: {
        value: summary.workOrders?.pending ?? 0,
        labelKey: 'admin.dashboard.widgets.work_orders_active.meta',
        fallbackLabel: 'waiting to start',
      },
    },
    pppoe_accounts: {
      id: 'pppoe_accounts',
      titleKey: 'admin.dashboard.widgets.pppoe_accounts.title',
      fallbackTitle: 'PPPoE accounts',
      value: summary.pppoe?.total ?? 0,
      href: routes.pppoe,
      icon: 'router',
      tone: 'cyan',
      meta: {
        value: summary.pppoe?.routerMissing ?? 0,
        labelKey: 'admin.dashboard.widgets.pppoe_accounts.meta',
        fallbackLabel: 'need router sync',
      },
    },
    incidents_open: {
      id: 'incidents_open',
      titleKey: 'admin.dashboard.widgets.incidents_open.title',
      fallbackTitle: 'Open incidents',
      value: summary.incidents?.total ?? 0,
      href: routes.incidents,
      icon: 'shield',
      tone: 'rose',
      meta: {
        value: summary.incidents?.critical ?? 0,
        labelKey: 'admin.dashboard.widgets.incidents_open.meta',
        fallbackLabel: 'critical',
      },
    },
    alerts_active: {
      id: 'alerts_active',
      titleKey: 'admin.dashboard.widgets.alerts_active.title',
      fallbackTitle: 'Active alerts',
      value: summary.alerts?.total ?? 0,
      href: routes.alerts,
      icon: 'alert-triangle',
      tone: 'amber',
      meta: {
        value: summary.alerts?.critical ?? 0,
        labelKey: 'admin.dashboard.widgets.alerts_active.meta',
        fallbackLabel: 'critical',
      },
    },
    routers_monitored: {
      id: 'routers_monitored',
      titleKey: 'admin.dashboard.widgets.routers_monitored.title',
      fallbackTitle: 'Monitored routers',
      value: summary.routersTotal ?? 0,
      href: routes.routers,
      icon: 'server',
      tone: 'cyan',
    },
    support_open: {
      id: 'support_open',
      titleKey: 'admin.dashboard.widgets.support_open.title',
      fallbackTitle: 'Open support tickets',
      value: summary.support?.open ?? 0,
      href: routes.support,
      icon: 'message-circle',
      tone: 'amber',
      meta: {
        value: summary.support?.pending ?? 0,
        labelKey: 'admin.dashboard.widgets.support_open.meta',
        fallbackLabel: 'pending reply',
      },
    },
  };

  const focusRegistry: Record<string, AdminDashboardFocusCard> = {
    work_orders_attention: {
      id: 'work_orders_attention',
      titleKey: 'admin.dashboard.focus.work_orders_attention.title',
      fallbackTitle: 'Installations waiting for action',
      descriptionKey: 'admin.dashboard.focus.work_orders_attention.description',
      fallbackDescription: 'Prioritize pending and in-progress work orders for technicians.',
      value: summary.workOrders?.active ?? 0,
      href: routes.installations,
      icon: 'activity',
      tone: 'emerald',
    },
    pppoe_sync_issues: {
      id: 'pppoe_sync_issues',
      titleKey: 'admin.dashboard.focus.pppoe_sync_issues.title',
      fallbackTitle: 'PPPoE sync issues',
      descriptionKey: 'admin.dashboard.focus.pppoe_sync_issues.description',
      fallbackDescription: 'Check accounts missing on router or disabled unexpectedly.',
      value: (summary.pppoe?.routerMissing ?? 0) + (summary.pppoe?.disabled ?? 0),
      href: routes.pppoe,
      icon: 'router',
      tone: 'cyan',
    },
    billing_followup: {
      id: 'billing_followup',
      titleKey: 'admin.dashboard.focus.billing_followup.title',
      fallbackTitle: 'Billing follow-up',
      descriptionKey: 'admin.dashboard.focus.billing_followup.description',
      fallbackDescription: 'Overdue and pending invoices that still need action today.',
      value: (summary.invoice?.unpaid ?? 0) + (summary.invoice?.overdue ?? 0),
      href: routes.invoices,
      icon: 'credit-card',
      tone: 'indigo',
    },
    customer_activation: {
      id: 'customer_activation',
      titleKey: 'admin.dashboard.focus.customer_activation.title',
      fallbackTitle: 'Customers awaiting activation',
      descriptionKey: 'admin.dashboard.focus.customer_activation.description',
      fallbackDescription: 'Customers who are close to service activation or payment handoff.',
      value: summary.activationsWaiting ?? 0,
      href: routes.customers,
      icon: 'users',
      tone: 'amber',
    },
    support_queue: {
      id: 'support_queue',
      titleKey: 'admin.dashboard.focus.support_queue.title',
      fallbackTitle: 'Support queue',
      descriptionKey: 'admin.dashboard.focus.support_queue.description',
      fallbackDescription: 'Open and pending tickets that need a coordinated response.',
      value: (summary.support?.open ?? 0) + (summary.support?.pending ?? 0),
      href: routes.support,
      icon: 'message-circle',
      tone: 'amber',
    },
    network_watch: {
      id: 'network_watch',
      titleKey: 'admin.dashboard.focus.network_watch.title',
      fallbackTitle: 'Network watchlist',
      descriptionKey: 'admin.dashboard.focus.network_watch.description',
      fallbackDescription: 'Alerts and incidents that need NOC review before they escalate.',
      value: (summary.alerts?.total ?? 0) + (summary.incidents?.total ?? 0),
      href: routes.noc,
      icon: 'shield',
      tone: 'rose',
    },
  };

  const actionRegistry: Record<string, AdminDashboardAction> = {
    team: {
      id: 'team',
      titleKey: 'admin.dashboard.actions.team.title',
      fallbackTitle: 'Team',
      descriptionKey: 'admin.dashboard.actions.team.description',
      fallbackDescription: 'Review members, invitations, and responsibilities.',
      href: routes.team,
      icon: 'users',
      tone: 'cyan',
    },
    roles: {
      id: 'roles',
      titleKey: 'admin.dashboard.actions.roles.title',
      fallbackTitle: 'Roles & permissions',
      descriptionKey: 'admin.dashboard.actions.roles.description',
      fallbackDescription: 'Adjust granular access without opening unrelated modules.',
      href: routes.roles,
      icon: 'lock',
      tone: 'amber',
    },
    settings: {
      id: 'settings',
      titleKey: 'admin.dashboard.actions.settings.title',
      fallbackTitle: 'Settings',
      descriptionKey: 'admin.dashboard.actions.settings.description',
      fallbackDescription: 'Update tenant-level policies, defaults, and network behavior.',
      href: routes.settings,
      icon: 'settings',
      tone: 'slate',
    },
    customers: {
      id: 'customers',
      titleKey: 'admin.dashboard.actions.customers.title',
      fallbackTitle: 'Customers',
      descriptionKey: 'admin.dashboard.actions.customers.description',
      fallbackDescription: 'Open customer records, service history, and read-only field context.',
      href: routes.customers,
      icon: 'users',
      tone: 'emerald',
    },
    invoices: {
      id: 'invoices',
      titleKey: 'admin.dashboard.actions.invoices.title',
      fallbackTitle: 'Invoices',
      descriptionKey: 'admin.dashboard.actions.invoices.description',
      fallbackDescription: 'Review outstanding invoices and retry collections when needed.',
      href: routes.invoices,
      icon: 'credit-card',
      tone: 'indigo',
    },
    installations: {
      id: 'installations',
      titleKey: 'admin.dashboard.actions.installations.title',
      fallbackTitle: 'Installations',
      descriptionKey: 'admin.dashboard.actions.installations.description',
      fallbackDescription: 'Claim work orders, track progress, and complete field jobs.',
      href: routes.installations,
      icon: 'activity',
      tone: 'emerald',
    },
    pppoe: {
      id: 'pppoe',
      titleKey: 'admin.dashboard.actions.pppoe.title',
      fallbackTitle: 'PPPoE',
      descriptionKey: 'admin.dashboard.actions.pppoe.description',
      fallbackDescription: 'Manage accounts and resolve router or RADIUS sync issues.',
      href: routes.pppoe,
      icon: 'router',
      tone: 'cyan',
    },
    noc: {
      id: 'noc',
      titleKey: 'admin.dashboard.actions.noc.title',
      fallbackTitle: 'NOC wallboard',
      descriptionKey: 'admin.dashboard.actions.noc.description',
      fallbackDescription: 'Open the live wallboard for the current network situation.',
      href: routes.noc,
      icon: 'shield',
      tone: 'rose',
    },
    alerts: {
      id: 'alerts',
      titleKey: 'admin.dashboard.actions.alerts.title',
      fallbackTitle: 'Alerts',
      descriptionKey: 'admin.dashboard.actions.alerts.description',
      fallbackDescription: 'Triage active router alerts and acknowledge noise quickly.',
      href: routes.alerts,
      icon: 'alert-triangle',
      tone: 'amber',
    },
    incidents: {
      id: 'incidents',
      titleKey: 'admin.dashboard.actions.incidents.title',
      fallbackTitle: 'Incidents',
      descriptionKey: 'admin.dashboard.actions.incidents.description',
      fallbackDescription: 'Review active incidents, owners, and severity escalations.',
      href: routes.incidents,
      icon: 'shield',
      tone: 'rose',
    },
    routers: {
      id: 'routers',
      titleKey: 'admin.dashboard.actions.routers.title',
      fallbackTitle: 'Routers',
      descriptionKey: 'admin.dashboard.actions.routers.description',
      fallbackDescription: 'Inspect router inventory, health, and per-device details.',
      href: routes.routers,
      icon: 'server',
      tone: 'cyan',
    },
    support: {
      id: 'support',
      titleKey: 'admin.dashboard.actions.support.title',
      fallbackTitle: 'Support',
      descriptionKey: 'admin.dashboard.actions.support.description',
      fallbackDescription: 'Open the shared ticket queue and continue customer replies.',
      href: routes.support,
      icon: 'message-circle',
      tone: 'amber',
    },
    audit: {
      id: 'audit',
      titleKey: 'admin.dashboard.actions.audit.title',
      fallbackTitle: 'Audit logs',
      descriptionKey: 'admin.dashboard.actions.audit.description',
      fallbackDescription: 'Review sensitive changes and permission-level activity.',
      href: routes.audit,
      icon: 'lock',
      tone: 'slate',
    },
    email_outbox: {
      id: 'email_outbox',
      titleKey: 'admin.dashboard.actions.email_outbox.title',
      fallbackTitle: 'Email outbox',
      descriptionKey: 'admin.dashboard.actions.email_outbox.description',
      fallbackDescription: 'Check failed deliveries and retry critical outbound messages.',
      href: routes.emailOutbox,
      icon: 'mail',
      tone: 'slate',
    },
  };

  const trendCards: AdminDashboardTrend[] = [];
  if (capabilities.billingRead && summary.invoice) {
    trendCards.push({
      id: 'invoice_status',
      titleKey: 'admin.dashboard.trends.invoice_status.title',
      fallbackTitle: 'Invoice status distribution',
      href: routes.invoices,
      items: [
        {
          id: 'overdue',
          value: summary.invoice.overdue,
          labelKey: 'admin.dashboard.trends.invoice_status.overdue',
          fallbackLabel: 'Overdue',
          tone: 'rose',
        },
        {
          id: 'pending',
          value: summary.invoice.unpaid,
          labelKey: 'admin.dashboard.trends.invoice_status.pending',
          fallbackLabel: 'Pending',
          tone: 'amber',
        },
        {
          id: 'paid',
          value: summary.invoice.paid,
          labelKey: 'admin.dashboard.trends.invoice_status.paid',
          fallbackLabel: 'Paid',
          tone: 'emerald',
        },
      ],
    });
  }
  if (capabilities.networkIncidentsRead && summary.incidents) {
    trendCards.push({
      id: 'incident_severity',
      titleKey: 'admin.dashboard.trends.incident_severity.title',
      fallbackTitle: 'Incident severity',
      href: routes.incidents,
      items: [
        {
          id: 'critical',
          value: summary.incidents.critical,
          labelKey: 'admin.dashboard.trends.incident_severity.critical',
          fallbackLabel: 'Critical',
          tone: 'rose',
        },
        {
          id: 'warning',
          value: summary.incidents.warning,
          labelKey: 'admin.dashboard.trends.incident_severity.warning',
          fallbackLabel: 'Warning',
          tone: 'amber',
        },
      ],
    });
  }
  if (capabilities.networkAlertsRead && summary.alerts) {
    trendCards.push({
      id: 'alert_severity',
      titleKey: 'admin.dashboard.trends.alert_severity.title',
      fallbackTitle: 'Alert severity',
      href: routes.alerts,
      items: [
        {
          id: 'critical',
          value: summary.alerts.critical,
          labelKey: 'admin.dashboard.trends.alert_severity.critical',
          fallbackLabel: 'Critical',
          tone: 'rose',
        },
        {
          id: 'warning',
          value: summary.alerts.warning,
          labelKey: 'admin.dashboard.trends.alert_severity.warning',
          fallbackLabel: 'Warning',
          tone: 'amber',
        },
      ],
    });
  }

  let primaryOrder: string[] = [];
  let focusOrder: string[] = [];
  let actionOrder: string[] = [];

  if (audience === 'admin') {
    primaryOrder = ['customers', 'invoices_due', 'team_members', 'subscription_plan'];
    focusOrder = ['billing_followup', 'customer_activation', 'network_watch', 'support_queue'];
    actionOrder = ['customers', 'invoices', 'team', 'roles', 'settings', 'support'];
  } else if (audience === 'operations') {
    primaryOrder = ['work_orders_active', 'pppoe_accounts', 'customers'];
    focusOrder = ['work_orders_attention', 'pppoe_sync_issues', 'customer_activation'];
    actionOrder = ['installations', 'pppoe', 'customers'];
  } else if (audience === 'support') {
    primaryOrder = ['customers', 'invoices_due', 'support_open'];
    focusOrder = ['billing_followup', 'customer_activation', 'support_queue'];
    actionOrder = ['customers', 'invoices', 'support'];
  } else if (audience === 'noc') {
    primaryOrder = ['incidents_open', 'alerts_active', 'routers_monitored'];
    focusOrder = ['network_watch'];
    actionOrder = ['noc', 'alerts', 'incidents', 'routers'];
  } else {
    primaryOrder = ['customers', 'work_orders_active', 'invoices_due', 'incidents_open'];
    focusOrder = ['work_orders_attention', 'billing_followup', 'network_watch', 'support_queue'];
    actionOrder = ['customers', 'installations', 'pppoe', 'invoices', 'noc', 'support'];
  }

  const visibleCards = new Set<string>();
  if (capabilities.customersRead) visibleCards.add('customers');
  if (capabilities.billingRead) {
    visibleCards.add('invoices_due');
    visibleCards.add('subscription_plan');
  }
  if (capabilities.teamRead) visibleCards.add('team_members');
  if (capabilities.workOrdersRead) visibleCards.add('work_orders_active');
  if (capabilities.pppoeRead) visibleCards.add('pppoe_accounts');
  if (capabilities.networkIncidentsRead) visibleCards.add('incidents_open');
  if (capabilities.networkAlertsRead) visibleCards.add('alerts_active');
  if (capabilities.routerInventoryRead || capabilities.networkNocRead)
    visibleCards.add('routers_monitored');
  if (capabilities.supportReadAll) visibleCards.add('support_open');

  const primaryStats = primaryOrder
    .filter((id) => visibleCards.has(id))
    .map((id) => registry[id])
    .filter(Boolean)
    .slice(0, 4);

  const focusCards = focusOrder
    .filter((id) => {
      if (id === 'work_orders_attention') return capabilities.workOrdersRead;
      if (id === 'pppoe_sync_issues') return capabilities.pppoeRead;
      if (id === 'billing_followup') return capabilities.billingRead;
      if (id === 'customer_activation') return capabilities.customersRead;
      if (id === 'support_queue') return capabilities.supportReadAll;
      if (id === 'network_watch')
        return (
          capabilities.networkAlertsRead ||
          capabilities.networkIncidentsRead ||
          capabilities.networkNocRead
        );
      return false;
    })
    .map((id) => focusRegistry[id])
    .filter(Boolean)
    .slice(0, 4);

  const quickActions = actionOrder
    .filter((id) => {
      if (id === 'team') return capabilities.teamRead;
      if (id === 'roles') return capabilities.rolesRead;
      if (id === 'settings') return capabilities.settingsRead;
      if (id === 'customers') return capabilities.customersRead;
      if (id === 'invoices') return capabilities.billingRead;
      if (id === 'installations') return capabilities.workOrdersRead;
      if (id === 'pppoe') return capabilities.pppoeRead;
      if (id === 'noc') return capabilities.networkNocRead;
      if (id === 'alerts') return capabilities.networkAlertsRead;
      if (id === 'incidents') return capabilities.networkIncidentsRead;
      if (id === 'routers') return capabilities.routerInventoryRead;
      if (id === 'support') return capabilities.supportReadAll;
      if (id === 'audit') return capabilities.auditLogsRead;
      if (id === 'email_outbox') return capabilities.emailOutboxRead;
      return false;
    })
    .map((id) => actionRegistry[id])
    .filter(Boolean)
    .slice(0, 6);

  return {
    audience,
    primaryStats,
    focusCards,
    quickActions,
    trendCards: trendCards.slice(0, 3),
  };
}
