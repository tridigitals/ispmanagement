import { customers } from '$lib/api/customers';
import { mikrotik } from '$lib/api/mikrotik';
import { payment } from '$lib/api/payment';
import { superadmin } from '$lib/api/superadmin';
import { support } from '$lib/api/support';
import { team } from '$lib/api/team';
import type { Invoice, TeamMember } from '$lib/api/types';
import type {
  GlobalSearchProvider,
  GlobalSearchProviderContext,
  GlobalSearchResult,
} from './globalSearchModel';

const PROVIDER_CACHE_TTL_MS = 30_000;
const providerCache = new Map<string, { expiresAt: number; value: unknown }>();

async function getCachedProviderValue<T>(key: string, loader: () => Promise<T>): Promise<T> {
  const now = Date.now();
  const cached = providerCache.get(key);
  if (cached && cached.expiresAt > now) {
    return cached.value as T;
  }

  const value = await loader();
  providerCache.set(key, {
    value,
    expiresAt: now + PROVIDER_CACHE_TTL_MS,
  });
  return value;
}

function includesQuery(values: Array<string | null | undefined>, query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return values.some((value) => String(value || '').toLowerCase().includes(needle));
}

function tenantAdminBasePath(context: GlobalSearchProviderContext, path: string) {
  return `${context.tenantPrefix}${path}`;
}

function mapInvoiceResult(
  invoice: Invoice,
  context: GlobalSearchProviderContext,
  href: string,
): GlobalSearchResult {
  return {
    id: invoice.id,
    kind: 'invoice',
    title: invoice.invoice_number,
    subtitle: `${invoice.status} • ${invoice.description || invoice.due_date}`,
    href,
    groupKey: context.isSuperAdmin ? 'superadmin-invoices' : 'invoices',
    groupLabel: context.isSuperAdmin ? 'Superadmin invoices' : 'Invoices',
  };
}

function mapTeamMemberResult(member: TeamMember, context: GlobalSearchProviderContext): GlobalSearchResult {
  return {
    id: member.id,
    kind: 'team-member',
    title: member.name,
    subtitle: `${member.email} • ${member.role_name || member.role}`,
    href: tenantAdminBasePath(context, `/admin/team`),
    groupKey: 'team-members',
    groupLabel: 'Team members',
  };
}

export function getGlobalSearchProviders(): GlobalSearchProvider[] {
  return [
    {
      key: 'customers',
      label: 'Customers',
      isEnabled: (context) =>
        context.shellScope === 'admin' &&
        (context.can('read', 'customers') || context.can('manage', 'customers')),
      minQueryLength: 1,
      search: async (query, context) => {
        const response = await customers.list({ q: query, page: 1, perPage: 5 });
        return response.data.map((customer) => ({
          id: customer.id,
          kind: 'customer',
          title: customer.name,
          subtitle: `${customer.email || customer.phone || 'No contact'} • ${customer.service_status}`,
          href: tenantAdminBasePath(context, `/v2/admin/customers/${customer.id}`),
          groupKey: 'customers',
          groupLabel: 'Customers',
        }));
      },
    },
    {
      key: 'routers',
      label: 'Routers',
      isEnabled: (context) =>
        context.shellScope === 'admin' &&
        (context.can('read', 'router_inventory') || context.can('manage', 'router_inventory')),
      minQueryLength: 2,
      search: async (query, context) => {
        const rows = await getCachedProviderValue('routers:list', () => mikrotik.routers.list());
        return rows
          .filter((row: any) => includesQuery([row.name, row.host, row.identity], query))
          .slice(0, 5)
          .map((row: any) => ({
            id: row.id,
            kind: 'router',
            title: row.name,
            subtitle: `${row.host}:${row.port} • ${row.is_online ? 'online' : 'offline'}`,
            href: tenantAdminBasePath(context, `/v2/admin/network/routers/${row.id}`),
            groupKey: 'routers',
            groupLabel: 'Routers',
          }));
      },
    },
    {
      key: 'invoices',
      label: 'Invoices',
      isEnabled: (context) =>
        context.shellScope === 'admin' &&
        (context.can('read', 'billing') || context.can('manage', 'billing')),
      minQueryLength: 2,
      search: async (query, context) => {
        const rows = await getCachedProviderValue('invoices:list', () => payment.listInvoices());
        return rows
          .filter((invoice) =>
            includesQuery([invoice.invoice_number, invoice.description, invoice.status], query),
          )
          .slice(0, 5)
          .map((invoice) =>
            mapInvoiceResult(
              invoice,
              context,
              tenantAdminBasePath(context, `/admin/invoices/${invoice.id}`),
            ),
          );
      },
    },
    {
      key: 'team-members',
      label: 'Team members',
      isEnabled: (context) =>
        context.shellScope === 'admin' &&
        (context.can('read', 'team') ||
          context.can('create', 'team') ||
          context.can('update', 'team') ||
          context.can('delete', 'team')),
      minQueryLength: 2,
      search: async (query, context) => {
        const rows = await getCachedProviderValue('team:list', () => team.list());
        return rows
          .filter((member) =>
            includesQuery(
              [member.name, member.email, member.role_name, member.role],
              query,
            ),
          )
          .slice(0, 5)
          .map((member) => mapTeamMemberResult(member, context));
      },
    },
    {
      key: 'support-tickets',
      label: 'Support tickets',
      isEnabled: (context) =>
        context.shellScope === 'admin' &&
        (context.can('read_all', 'support') ||
          context.can('read', 'support') ||
          context.can('create', 'support')),
      minQueryLength: 1,
      search: async (query, context) => {
        const response = await support.list({ search: query, page: 1, perPage: 5 });
        const basePath = context.can('read_all', 'support')
          ? tenantAdminBasePath(context, '/admin/support')
          : `${context.tenantPrefix}/support`;
        return response.data.map((ticket) => ({
          id: ticket.id,
          kind: 'support-ticket',
          title: ticket.subject,
          subtitle: `${ticket.status} • ${ticket.priority}`,
          href: `${basePath}/${ticket.id}`,
          groupKey: 'support-tickets',
          groupLabel: 'Support tickets',
        }));
      },
    },
    {
      key: 'tenants',
      label: 'Tenants',
      isEnabled: (context) => context.shellScope === 'superadmin' && context.isSuperAdmin,
      minQueryLength: 1,
      search: async (query) => {
        const response = await superadmin.listTenants();
        return response.data
          .filter((tenant) => includesQuery([tenant.name, tenant.slug, tenant.custom_domain], query))
          .slice(0, 5)
          .map((tenant) => ({
            id: tenant.id,
            kind: 'tenant' as const,
            title: tenant.name,
            subtitle: `${tenant.slug}${tenant.custom_domain ? ` • ${tenant.custom_domain}` : ''}`,
            href: '/superadmin/tenants',
            groupKey: 'tenants',
            groupLabel: 'Tenants',
          }));
      },
    },
    {
      key: 'superadmin-invoices',
      label: 'Superadmin invoices',
      isEnabled: (context) => context.shellScope === 'superadmin' && context.isSuperAdmin,
      minQueryLength: 2,
      search: async (query, context) => {
        const rows = await getCachedProviderValue('superadmin-invoices:list', () =>
          payment.listAllInvoices(),
        );
        return rows
          .filter((invoice) =>
            includesQuery([invoice.invoice_number, invoice.description, invoice.status], query),
          )
          .slice(0, 5)
          .map((invoice) => mapInvoiceResult(invoice, context, `/superadmin/invoices/${invoice.id}`));
      },
    },
  ];
}

export function getEnabledGlobalSearchProviderKeys(context: GlobalSearchProviderContext): string[] {
  return getGlobalSearchProviders()
    .filter((provider) => provider.isEnabled(context))
    .map((provider) => provider.key);
}

export function resetGlobalSearchProviderCaches() {
  providerCache.clear();
}
