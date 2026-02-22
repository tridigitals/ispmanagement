<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import {
    api,
    type AuditLog,
    type Customer,
    type CustomerLocation,
    type CustomerSubscriptionView,
    type Invoice,
    type IspPackageRouterMappingView,
  } from '$lib/api/client';
  import type { PppoeAccountPublic } from '$lib/api/client';
  import { timeAgo } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Table from '$lib/components/ui/Table.svelte';

  const customerDetailTabs = [
    'overview',
    'locations',
    'subscriptions',
    'billing',
    'timeline',
    'pppoe',
  ] as const;
  type CustomerDetailTab = (typeof customerDetailTabs)[number];

  const customerId = $derived(String($page.params.id || ''));

  let activeTab = $state<CustomerDetailTab>('overview');

  let customer = $state<Customer | null>(null);
  let loadingCustomer = $state(true);

  let locations = $state<CustomerLocation[]>([]);
  let loadingLocations = $state(false);

  // Subscriptions
  let subscriptions = $state<CustomerSubscriptionView[]>([]);
  let loadingSubscriptions = $state(false);
  let timelineLogs = $state<AuditLog[]>([]);
  let timelineType = $state<'all' | 'customer' | 'location' | 'subscription'>('all');
  let loadingTimeline = $state(false);
  let showAddSubscription = $state(false);
  let showEditSubscription = $state(false);
  let editingSubscription = $state<CustomerSubscriptionView | null>(null);
  let savingSubscription = $state(false);
  let deletingSubscription = $state<string | null>(null);
  let togglingSubscription = $state<string | null>(null);
  let subscriptionPackages = $state<any[]>([]);

  let subLocationId = $state('');
  let subPackageId = $state('');
  let subRouterId = $state('');
  let subBillingCycle = $state<'monthly' | 'yearly'>('monthly');
  let subPrice = $state('');
  let subCurrency = $state('');
  let subStatus = $state<'active' | 'suspended' | 'cancelled'>('active');
  let subStartsAt = $state('');
  let subEndsAt = $state('');
  let subNotes = $state('');
  let billingInvoices = $state<Invoice[]>([]);
  let loadingBilling = $state(false);
  let billingStatus = $state<'all' | 'pending' | 'verification_pending' | 'paid' | 'failed'>('all');
  let billingDateFrom = $state('');
  let billingDateTo = $state('');
  let billingQuickRange = $state<'' | 'today' | '7d' | '30d' | 'month'>('');
  let generatingInvoiceFor = $state<string | null>(null);

  // PPPoE
  let pppoeAccounts = $state<PppoeAccountPublic[]>([]);
  let loadingPppoe = $state(false);
  let pppoeQuery = $state('');
  let pppoeRouters = $state<any[]>([]);
  let loadingPppoeRouters = $state(false);
  let showAddPppoe = $state(false);
  let showEditPppoe = $state(false);
  let editingPppoe = $state<PppoeAccountPublic | null>(null);
  let savingPppoe = $state(false);

  let pppoeRouterId = $state('');
  let pppoeLocationId = $state('');
  let pppoeUsername = $state('');
  let pppoePassword = $state('');
  let pppoeRouterProfileName = $state('');
  let pppoeRemoteAddress = $state('');
  let pppoeAddressPool = $state('');
  let pppoeDisabled = $state(false);
  let pppoeComment = $state('');
  let pppoePackageId = $state('');
  let pppoePackageMappings = $state<IspPackageRouterMappingView[]>([]);
  const pppoePackageOptions = $derived.by(() => {
    const seen = new Set<string>();
    const out: Array<{ label: string; value: string }> = [];
    for (const m of pppoePackageMappings) {
      if (!m?.package_id || seen.has(m.package_id)) continue;
      seen.add(m.package_id);
      out.push({ label: m.package_name, value: m.package_id });
    }
    return out;
  });

  // Router-scoped inventory (used as suggestions for profile/pool fields)
  let pppoeProfiles = $state<any[]>([]);
  let pppoePools = $state<any[]>([]);
  let pppoeInventoryLoading = $state(false);
  let pppoeInventoryRouter = $state<string | null>(null);

  const pppoeProfileOptions = $derived.by(() => {
    const base = (pppoeProfiles || []).map((p: any) => ({ label: String(p.name), value: String(p.name) }));
    const cur = pppoeRouterProfileName?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const pppoePoolOptions = $derived.by(() => {
    const base = (pppoePools || []).map((p: any) => ({ label: String(p.name), value: String(p.name) }));
    const cur = pppoeAddressPool?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const pppoeColumns = $derived.by(() => [
    { key: 'username', label: $t('admin.customers.pppoe.columns.username') || 'Username' },
    { key: 'router', label: $t('admin.customers.pppoe.columns.router') || 'Router' },
    { key: 'location', label: $t('admin.customers.pppoe.columns.location') || 'Location' },
    { key: 'assignment', label: $t('admin.customers.pppoe.columns.assignment') || 'IP / Profile' },
    { key: 'sync', label: $t('admin.customers.pppoe.columns.sync') || 'Sync' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  // Overview form
  let name = $state('');
  let email = $state('');
  let phone = $state('');
  let notes = $state('');
  let isActive = $state(true);
  let saving = $state(false);
  let togglingCustomerStatus = $state(false);

  // Location modal
  let showAddLocation = $state(false);
  let showEditLocation = $state(false);
  let creatingLocation = $state(false);
  let updatingLocation = $state(false);
  let deletingLocation = $state(false);
  let editingLocation = $state<CustomerLocation | null>(null);
  let locationToDelete = $state<CustomerLocation | null>(null);
  let showDeleteLocation = $state(false);
  let locLabel = $state('');
  let locAddress1 = $state('');
  let locAddress2 = $state('');
  let locCity = $state('');
  let locState = $state('');
  let locPostal = $state('');
  let locCountry = $state('');
  let locNotes = $state('');

  // Deletes
  let showDeleteCustomer = $state(false);
  let deletingCustomer = $state(false);

  const locColumns = $derived.by(() => [
    { key: 'label', label: $t('admin.customers.locations.columns.label') || 'Label' },
    { key: 'address', label: $t('admin.customers.locations.columns.address') || 'Address' },
    { key: 'updated_at', label: $t('admin.customers.locations.columns.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  const subscriptionColumns = $derived.by(() => [
    { key: 'package', label: 'Package' },
    { key: 'billing', label: 'Billing' },
    { key: 'location', label: 'Location' },
    { key: 'router', label: 'Router' },
    { key: 'period', label: 'Period' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  const billingColumns = $derived.by(() => [
    { key: 'invoice_number', label: $t('admin.customers.billing.columns.invoice_number') || 'Invoice #' },
    { key: 'subscription', label: $t('admin.customers.billing.columns.subscription') || 'Subscription' },
    { key: 'amount', label: $t('admin.customers.billing.columns.amount') || 'Amount' },
    { key: 'status', label: $t('admin.customers.billing.columns.status') || 'Status' },
    { key: 'due_date', label: $t('admin.customers.billing.columns.due_date') || 'Due date' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  const billingCycleOptions = [
    { label: 'Monthly', value: 'monthly' },
    { label: 'Yearly', value: 'yearly' },
  ];

  const subscriptionStatusOptions = [
    { label: 'Active', value: 'active' },
    { label: 'Suspended', value: 'suspended' },
    { label: 'Cancelled', value: 'cancelled' },
  ];

  const subscriptionRouterOptions = $derived.by(() =>
    pppoeRouters.map((r) => ({ label: r.name, value: r.id })),
  );

  const subscriptionLocationOptions = $derived.by(() =>
    locations.map((l) => ({ label: l.label, value: l.id })),
  );

  const subscriptionPackageOptions = $derived.by(() =>
    subscriptionPackages
      .filter((p: any) => p?.is_active !== false)
      .map((p: any) => ({ label: p.name, value: p.id })),
  );
  const canReadAudit = $derived($can('read', 'audit_logs'));
  const timelineFilteredLogs = $derived.by(() => {
    if (timelineType === 'all') return timelineLogs;
    if (timelineType === 'customer') return timelineLogs.filter((l) => l.resource === 'customers');
    if (timelineType === 'location') return timelineLogs.filter((l) => l.resource === 'customer_locations');
    if (timelineType === 'subscription')
      return timelineLogs.filter((l) => l.resource === 'customer_subscriptions');
    return timelineLogs;
  });
  const subscriptionById = $derived.by(
    () => new Map(subscriptions.map((sub) => [sub.id, sub] as const)),
  );
  const billingRows = $derived.by(() => {
    const rows = billingInvoices.filter((inv) => {
      const sid = getSubscriptionIdFromInvoice(inv);
      if (!sid || !subscriptionById.has(sid)) return false;
      if (billingStatus !== 'all' && inv.status !== billingStatus) return false;
      const refDate = new Date(inv.created_at || inv.due_date);
      if (Number.isNaN(refDate.getTime())) return false;
      if (billingDateFrom) {
        const from = new Date(`${billingDateFrom}T00:00:00`);
        if (refDate < from) return false;
      }
      if (billingDateTo) {
        const to = new Date(`${billingDateTo}T23:59:59.999`);
        if (refDate > to) return false;
      }
      return true;
    });

    return rows.sort(
      (a, b) => new Date(b.created_at || b.due_date).getTime() - new Date(a.created_at || a.due_date).getTime(),
    );
  });
  const billingStats = $derived.by(() => {
    const now = Date.now();
    const overdue = billingRows.filter((inv) => inv.status !== 'paid' && new Date(inv.due_date).getTime() < now)
      .length;
    const unpaid = billingRows.filter((inv) => ['pending', 'verification_pending'].includes(inv.status)).length;
    const paid = billingRows.filter((inv) => inv.status === 'paid').length;
    return {
      total: billingRows.length,
      unpaid,
      paid,
      overdue,
    };
  });

  onMount(async () => {
    if (!$can('read', 'customers') && !$can('manage', 'customers')) {
      goto('/unauthorized');
      return;
    }
    const fromUrl = readActiveTabFromUrl();
    if (fromUrl) activeTab = fromUrl;
    await loadCustomer();
    await loadLocations();
  });

  $effect(() => {
    const fromUrl = readActiveTabFromUrl();
    if (fromUrl && fromUrl !== activeTab) {
      activeTab = fromUrl;
    }
  });

  $effect(() => {
    if (activeTab !== 'subscriptions') return;
    if (!$can('read', 'customers') && !$can('manage', 'customers')) return;
    void loadSubscriptions();
    if (subscriptionPackages.length === 0) {
      void loadSubscriptionPackages();
    }
  });

  $effect(() => {
    if (activeTab !== 'billing') return;
    if (!$can('read', 'customers') && !$can('manage', 'customers')) return;
    void loadSubscriptions();
    void loadBillingInvoices();
  });

  $effect(() => {
    if (activeTab !== 'timeline') return;
    if (!canReadAudit) return;
    void loadTimeline();
  });

  async function loadPppoeInventory(routerId: string, opts?: { silent?: boolean }) {
    if (pppoeInventoryLoading) return;
    if (!routerId) return;

    pppoeInventoryLoading = true;
    try {
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId) as any,
        api.mikrotik.routers.ipPools(routerId) as any,
      ]);
      pppoeProfiles = profiles || [];
      pppoePools = pools || [];
      pppoeInventoryRouter = routerId;
    } catch (e: any) {
      if (!opts?.silent) {
        toast.error(e?.message || e);
      }
    } finally {
      pppoeInventoryLoading = false;
    }
  }

  async function loadPppoePackages(routerId: string) {
    if (!routerId) {
      pppoePackageMappings = [];
      return;
    }
    try {
      pppoePackageMappings = await api.ispPackages.routerMappings.list({ router_id: routerId });
    } catch {
      pppoePackageMappings = [];
    }
  }

  function maybeAutoSelectPppoePackageFromProfile() {
    const profile = pppoeRouterProfileName?.trim();
    if (!pppoeRouterId || !profile) return;
    if (pppoePackageId) return;

    const matches = pppoePackageMappings.filter((m) => (m.router_profile_name || '') === profile);
    if (matches.length === 1) {
      pppoePackageId = matches[0].package_id;
      applyPppoePackage(pppoePackageId);
      return;
    }

    if (!pppoeAddressPool) {
      const withPool = matches.find((m) => m.address_pool);
      if (withPool?.address_pool) pppoeAddressPool = withPool.address_pool;
    }
  }

  function applyPppoePackage(pkgId: string) {
    if (!pkgId) return;
    const m = pppoePackageMappings.find((x) => x.package_id === pkgId);
    if (!m) return;
    pppoeRouterProfileName = m.router_profile_name || '';
    if (m.address_pool) {
      pppoeAddressPool = m.address_pool;
      pppoeRemoteAddress = '';
    }
  }

  $effect(() => {
    if (!showAddPppoe && !showEditPppoe) return;
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) return;

    const rid = pppoeRouterId;
    if (!rid) {
      pppoeProfiles = [];
      pppoePools = [];
      pppoeInventoryRouter = null;
      pppoePackageMappings = [];
      return;
    }

    if (pppoeInventoryRouter === rid) {
      // still ensure packages are loaded once per router selection
      if (pppoePackageMappings.length === 0) void loadPppoePackages(rid);
      return;
    }
    void loadPppoeInventory(rid, { silent: true });
    void loadPppoePackages(rid);
  });

  async function loadCustomer() {
    loadingCustomer = true;
    try {
      const c = await api.customers.get(customerId);
      customer = c;
      name = c.name || '';
      email = c.email || '';
      phone = c.phone || '';
      notes = c.notes || '';
      isActive = !!c.is_active;
    } catch (e: any) {
      toast.error(get(t)('admin.customers.toasts.load_failed') || 'Failed to load customer');
      goto('..');
    } finally {
      loadingCustomer = false;
    }
  }

  async function loadLocations() {
    if (!$can('read', 'customer_locations') && !$can('manage', 'customer_locations')) return;
    loadingLocations = true;
    try {
      locations = await api.customers.locations.list(customerId);
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.locations.toasts.load_failed') ||
          `Failed to load locations: ${e?.message || e}`,
      );
    } finally {
      loadingLocations = false;
    }
  }

  async function loadSubscriptionPackages() {
    try {
      const res = await api.ispPackages.packages.list({ page: 1, per_page: 500, q: '' });
      subscriptionPackages = res.data || [];
    } catch {
      subscriptionPackages = [];
    }
  }

  async function loadSubscriptions() {
    loadingSubscriptions = true;
    try {
      const res = await api.customers.subscriptions.list(customerId, { page: 1, per_page: 200 });
      subscriptions = res.data || [];
    } catch (e: any) {
      toast.error(`Failed to load subscriptions: ${e?.message || e}`);
    } finally {
      loadingSubscriptions = false;
    }
  }

  function readActiveTabFromUrl(): CustomerDetailTab | null {
    const tab = String($page.url.searchParams.get('tab') || '').toLowerCase();
    return customerDetailTabs.includes(tab as CustomerDetailTab) ? (tab as CustomerDetailTab) : null;
  }

  function getSubscriptionIdFromInvoice(inv: Invoice): string | null {
    const ext = inv.external_id || '';
    if (!ext.startsWith('pkgsub:')) return null;
    const raw = ext.slice('pkgsub:'.length);
    const idx = raw.indexOf(':');
    if (idx <= 0) return null;
    return raw.slice(0, idx);
  }

  function billingStatusLabel(status: string): string {
    const map: Record<string, string> = {
      pending: get(t)('admin.package_invoices.statuses.pending') || 'Pending',
      verification_pending:
        get(t)('admin.package_invoices.statuses.verification_pending') || 'Verification pending',
      paid: get(t)('admin.package_invoices.statuses.paid') || 'Paid',
      failed: get(t)('admin.package_invoices.statuses.failed') || 'Failed',
    };
    return map[status] || status;
  }

  async function loadBillingInvoices() {
    if (loadingBilling) return;
    loadingBilling = true;
    try {
      const invoices = await api.payment.listCustomerPackageInvoices();
      billingInvoices = invoices;
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.billing.toasts.load_failed', { values: { message: e?.message || e } }) ||
          `Failed to load billing invoices: ${e?.message || e}`,
      );
    } finally {
      loadingBilling = false;
    }
  }

  async function generateInvoiceForSubscription(subscriptionId: string) {
    if (!subscriptionId || generatingInvoiceFor) return;
    generatingInvoiceFor = subscriptionId;
    try {
      await api.payment.createInvoiceForCustomerSubscription(subscriptionId);
      toast.success(
        get(t)('admin.customers.billing.toasts.generated') || 'Invoice generated successfully',
      );
      activeTab = 'billing';
      await loadBillingInvoices();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.billing.toasts.generate_failed', { values: { message: e?.message || e } }) ||
          `Failed to generate invoice: ${e?.message || e}`,
      );
    } finally {
      generatingInvoiceFor = null;
    }
  }

  function openInvoiceDetail(id: string) {
    const base = $page.url.pathname.replace(/\/admin\/customers\/[^/]+\/?$/, '/admin');
    void goto(`${base}/invoices/${id}`);
  }

  function clearBillingFilters() {
    billingStatus = 'all';
    billingDateFrom = '';
    billingDateTo = '';
    billingQuickRange = '';
  }

  function formatDateInputValue(d: Date): string {
    const local = new Date(d.getTime() - d.getTimezoneOffset() * 60_000);
    return local.toISOString().slice(0, 10);
  }

  function applyBillingQuickRange(range: 'today' | '7d' | '30d' | 'month') {
    const end = new Date();
    const start = new Date(end);
    if (range === '7d') start.setDate(start.getDate() - 6);
    if (range === '30d') start.setDate(start.getDate() - 29);
    if (range === 'month') start.setDate(1);
    billingDateFrom = formatDateInputValue(start);
    billingDateTo = formatDateInputValue(end);
    billingQuickRange = range;
  }

  function onBillingDateChange() {
    billingQuickRange = '';
  }

  async function loadTimeline() {
    if (!canReadAudit) return;
    loadingTimeline = true;
    try {
      const [res, locRows, subRes] = await Promise.all([
        api.audit.listTenant(1, 100, { customer_id: customerId }),
        api.customers.locations.list(customerId).catch(() => [] as CustomerLocation[]),
        api.customers.subscriptions
          .list(customerId, { page: 1, per_page: 500 })
          .catch(() => ({ data: [] as CustomerSubscriptionView[] } as any)),
      ]);

      const allowedLocationIds = new Set((locRows || []).map((l) => l.id));
      const allowedSubscriptionIds = new Set(((subRes?.data as CustomerSubscriptionView[]) || []).map((s) => s.id));

      timelineLogs = (res.data || []).filter((log) => {
        if (log.resource === 'customers') {
          return log.resource_id === customerId;
        }
        if (log.resource === 'customer_locations') {
          return !!log.resource_id && allowedLocationIds.has(log.resource_id);
        }
        if (log.resource === 'customer_subscriptions') {
          return !!log.resource_id && allowedSubscriptionIds.has(log.resource_id);
        }
        return false;
      });
      timelineType = 'all';
    } catch (e: any) {
      toast.error(`Failed to load timeline: ${e?.message || e}`);
    } finally {
      loadingTimeline = false;
    }
  }

  function timelineActionLabel(action: string): string {
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
    return map[action] || action.replaceAll('_', ' ').toLowerCase().replace(/^./, (m) => m.toUpperCase());
  }

  function timelineResourceLabel(resource: string): string {
    const map: Record<string, string> = {
      customers: 'Customer',
      customer_locations: 'Location',
      customer_subscriptions: 'Subscription',
      customer_users: 'Portal user',
    };
    return map[resource] || resource;
  }

  function timelineActorLabel(log: AuditLog): string {
    return log.user_name || log.user_email || 'System';
  }

  async function refreshCurrent() {
    await Promise.all([
      loadCustomer(),
      loadLocations(),
      activeTab === 'subscriptions' ? loadSubscriptions() : Promise.resolve(),
      activeTab === 'billing' ? loadBillingInvoices() : Promise.resolve(),
      activeTab === 'pppoe' ? loadPppoeAccounts() : Promise.resolve(),
      activeTab === 'timeline' && canReadAudit ? loadTimeline() : Promise.resolve(),
    ]);
  }

  function resetSubscriptionForm() {
    subLocationId = locations[0]?.id || '';
    subPackageId = '';
    subRouterId = '';
    subBillingCycle = 'monthly';
    subPrice = '';
    subCurrency = '';
    subStatus = 'active';
    subStartsAt = '';
    subEndsAt = '';
    subNotes = '';
  }

  function openCreateSubscription() {
    resetSubscriptionForm();
    subCurrency = subCurrency || 'IDR';
    showAddSubscription = true;
  }

  function openEditSubscription(row: CustomerSubscriptionView) {
    editingSubscription = row;
    subLocationId = row.location_id;
    subPackageId = row.package_id;
    subRouterId = row.router_id || '';
    subBillingCycle = (row.billing_cycle === 'yearly' ? 'yearly' : 'monthly') as 'monthly' | 'yearly';
    subPrice = String(row.price ?? '');
    subCurrency = row.currency_code || '';
    subStatus = (['active', 'suspended', 'cancelled'].includes(row.status)
      ? row.status
      : 'active') as 'active' | 'suspended' | 'cancelled';
    subStartsAt = row.starts_at ? row.starts_at.slice(0, 10) : '';
    subEndsAt = row.ends_at ? row.ends_at.slice(0, 10) : '';
    subNotes = row.notes || '';
    showEditSubscription = true;
  }

  async function submitCreateSubscription() {
    const price = Number(subPrice);
    if (!subLocationId || !subPackageId || !Number.isFinite(price) || price < 0) return;
    savingSubscription = true;
    try {
      await api.customers.subscriptions.create(customerId, {
        location_id: subLocationId,
        package_id: subPackageId,
        router_id: subRouterId || null,
        billing_cycle: subBillingCycle,
        price,
        currency_code: subCurrency || null,
        status: subStatus,
        starts_at: subStartsAt || null,
        ends_at: subEndsAt || null,
        notes: subNotes.trim() || null,
      });
      toast.success('Subscription created');
      showAddSubscription = false;
      await loadSubscriptions();
    } catch (e: any) {
      toast.error(`Failed to create subscription: ${e?.message || e}`);
    } finally {
      savingSubscription = false;
    }
  }

  async function submitUpdateSubscription() {
    if (!editingSubscription) return;
    const price = Number(subPrice);
    if (!subLocationId || !subPackageId || !Number.isFinite(price) || price < 0) return;
    savingSubscription = true;
    try {
      await api.customers.subscriptions.update(editingSubscription.id, {
        location_id: subLocationId,
        package_id: subPackageId,
        router_id: subRouterId || null,
        billing_cycle: subBillingCycle,
        price,
        currency_code: subCurrency || null,
        status: subStatus,
        starts_at: subStartsAt || null,
        ends_at: subEndsAt || null,
        notes: subNotes.trim() || null,
      });
      toast.success('Subscription updated');
      showEditSubscription = false;
      editingSubscription = null;
      await loadSubscriptions();
    } catch (e: any) {
      toast.error(`Failed to update subscription: ${e?.message || e}`);
    } finally {
      savingSubscription = false;
    }
  }

  async function deleteSubscription(id: string) {
    if (!confirm('Delete this subscription?')) return;
    deletingSubscription = id;
    try {
      await api.customers.subscriptions.delete(id);
      toast.success('Subscription deleted');
      await loadSubscriptions();
    } catch (e: any) {
      toast.error(`Failed to delete subscription: ${e?.message || e}`);
    } finally {
      deletingSubscription = null;
    }
  }

  async function setSubscriptionStatus(
    row: CustomerSubscriptionView,
    nextStatus: 'active' | 'suspended',
  ) {
    togglingSubscription = row.id;
    try {
      await api.customers.subscriptions.update(row.id, { status: nextStatus });
      toast.success(nextStatus === 'suspended' ? 'Subscription suspended' : 'Subscription resumed');
      await loadSubscriptions();
    } catch (e: any) {
      toast.error(`Failed to update status: ${e?.message || e}`);
    } finally {
      togglingSubscription = null;
    }
  }

  async function loadPppoeRouters() {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) return;
    loadingPppoeRouters = true;
    try {
      pppoeRouters = await api.mikrotik.routers.list();
    } catch (e: any) {
      toast.error(get(t)('admin.customers.pppoe.toasts.routers_failed') || 'Failed to load routers');
    } finally {
      loadingPppoeRouters = false;
    }
  }

  async function loadPppoeAccounts() {
    loadingPppoe = true;
    try {
      const res = await api.pppoe.accounts.list({
        customer_id: customerId,
        q: pppoeQuery.trim() || undefined,
        page: 1,
        per_page: 200,
      });
      pppoeAccounts = res.data || [];
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.load_failed', { values: { message: e?.message || e } }) ||
          `Failed to load PPPoE accounts: ${e?.message || e}`,
      );
    } finally {
      loadingPppoe = false;
    }
  }

  function resetPppoeForm() {
    pppoeRouterId = '';
    pppoeLocationId = '';
    pppoeUsername = '';
    pppoePassword = '';
    pppoePackageId = '';
    pppoeRouterProfileName = '';
    pppoeRemoteAddress = '';
    pppoeAddressPool = '';
    pppoeDisabled = false;
    pppoeComment = '';
    pppoePackageMappings = [];
  }

  function openCreatePppoe() {
    resetPppoeForm();
    showAddPppoe = true;
  }

  function openEditPppoe(row: PppoeAccountPublic) {
    editingPppoe = row;
    pppoeRouterId = row.router_id;
    pppoeLocationId = row.location_id;
    pppoeUsername = row.username;
    pppoePassword = '';
    pppoePackageId = row.package_id || '';
    pppoeRouterProfileName = row.router_profile_name || '';
    pppoeRemoteAddress = row.remote_address || '';
    pppoeAddressPool = row.address_pool || '';
    pppoeDisabled = !!row.disabled;
    pppoeComment = row.comment || '';
    showEditPppoe = true;
  }

  async function submitCreatePppoe() {
    if (!pppoeRouterId || !pppoeLocationId || !pppoeUsername.trim() || !pppoePassword) return;
    savingPppoe = true;
    try {
      await api.pppoe.accounts.create({
        router_id: pppoeRouterId,
        customer_id: customerId,
        location_id: pppoeLocationId,
        username: pppoeUsername.trim(),
        password: pppoePassword,
        package_id: pppoePackageId || null,
        router_profile_name: pppoeRouterProfileName.trim() || null,
        remote_address: pppoeRemoteAddress.trim() || null,
        address_pool: pppoeAddressPool.trim() || null,
        disabled: pppoeDisabled,
        comment: pppoeComment.trim() || null,
      });
      toast.success(get(t)('admin.customers.pppoe.toasts.created') || 'PPPoE account created');
      showAddPppoe = false;
      resetPppoeForm();
      await loadPppoeAccounts();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.create_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    } finally {
      savingPppoe = false;
    }
  }

  async function submitUpdatePppoe() {
    if (!editingPppoe) return;
    savingPppoe = true;
    try {
      await api.pppoe.accounts.update(editingPppoe.id, {
        username: pppoeUsername.trim() || undefined,
        password: pppoePassword || undefined,
        package_id: pppoePackageId || null,
        router_profile_name: pppoeRouterProfileName.trim() || null,
        remote_address: pppoeRemoteAddress.trim() || null,
        address_pool: pppoeAddressPool.trim() || null,
        disabled: pppoeDisabled,
        comment: pppoeComment.trim() || null,
      });
      toast.success(get(t)('admin.customers.pppoe.toasts.updated') || 'PPPoE account updated');
      showEditPppoe = false;
      editingPppoe = null;
      resetPppoeForm();
      await loadPppoeAccounts();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.update_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    } finally {
      savingPppoe = false;
    }
  }

  async function applyPppoe(row: PppoeAccountPublic) {
    try {
      await api.pppoe.accounts.apply(row.id);
      toast.success(get(t)('admin.customers.pppoe.toasts.applied') || 'Applied to router');
      await loadPppoeAccounts();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.apply_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }

  async function reconcilePppoeRouters() {
    const routerIds = Array.from(new Set(pppoeAccounts.map((a) => a.router_id).filter(Boolean)));
    if (routerIds.length === 0) return;
    try {
      for (const rid of routerIds) {
        await api.pppoe.accounts.reconcileRouter(rid);
      }
      toast.success(get(t)('admin.customers.pppoe.toasts.reconciled') || 'Reconciled router state');
      await loadPppoeAccounts();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.reconcile_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }

  async function deletePppoe(row: PppoeAccountPublic) {
    if (!confirm(get(t)('admin.customers.pppoe.confirm_delete') || 'Delete this PPPoE account?')) return;
    try {
      await api.pppoe.accounts.delete(row.id);
      toast.success(get(t)('admin.customers.pppoe.toasts.deleted') || 'Deleted');
      await loadPppoeAccounts();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.delete_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }

  async function saveOverview() {
    if (!customer) return;
    saving = true;
    try {
      const updated = await api.customers.update(customer.id, {
        name: name.trim(),
        email: email.trim(),
        phone: phone.trim(),
        notes: notes.trim(),
        is_active: isActive,
      });
      customer = updated;
      toast.success(get(t)('admin.customers.toasts.updated') || 'Customer updated');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.toasts.update_failed', { values: { message: e?.message || e } }) ||
          `Failed to update: ${e?.message || e}`,
      );
    } finally {
      saving = false;
    }
  }

  async function addLocation() {
    if (!locLabel.trim()) return;
    creatingLocation = true;
    try {
      await api.customers.locations.create({
        customer_id: customerId,
        label: locLabel.trim(),
        address_line1: locAddress1.trim() || null,
        address_line2: locAddress2.trim() || null,
        city: locCity.trim() || null,
        state: locState.trim() || null,
        postal_code: locPostal.trim() || null,
        country: locCountry.trim() || null,
        notes: locNotes.trim() || null,
      });
      showAddLocation = false;
      locLabel = '';
      locAddress1 = '';
      locAddress2 = '';
      locCity = '';
      locState = '';
      locPostal = '';
      locCountry = '';
      locNotes = '';
      await loadLocations();
      toast.success(get(t)('admin.customers.locations.toasts.created') || 'Location added');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.locations.toasts.create_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    } finally {
      creatingLocation = false;
    }
  }

  async function setCustomerActive(next: boolean) {
    if (!customer) return;
    togglingCustomerStatus = true;
    try {
      const updated = await api.customers.update(customer.id, { is_active: next });
      customer = updated;
      isActive = !!updated.is_active;
      toast.success(next ? 'Customer activated' : 'Customer suspended');
    } catch (e: any) {
      toast.error(`Failed to update status: ${e?.message || e}`);
    } finally {
      togglingCustomerStatus = false;
    }
  }

  function resetLocationForm(row?: CustomerLocation) {
    locLabel = row?.label || '';
    locAddress1 = row?.address_line1 || '';
    locAddress2 = row?.address_line2 || '';
    locCity = row?.city || '';
    locState = row?.state || '';
    locPostal = row?.postal_code || '';
    locCountry = row?.country || '';
    locNotes = row?.notes || '';
  }

  function openCreateLocation() {
    editingLocation = null;
    resetLocationForm();
    showAddLocation = true;
  }

  function openEditLocation(row: CustomerLocation) {
    editingLocation = row;
    resetLocationForm(row);
    showEditLocation = true;
  }

  async function submitUpdateLocation() {
    if (!editingLocation || !locLabel.trim()) return;
    updatingLocation = true;
    try {
      await api.customers.locations.update(editingLocation.id, {
        label: locLabel.trim(),
        address_line1: locAddress1.trim() || null,
        address_line2: locAddress2.trim() || null,
        city: locCity.trim() || null,
        state: locState.trim() || null,
        postal_code: locPostal.trim() || null,
        country: locCountry.trim() || null,
        notes: locNotes.trim() || null,
      });
      showEditLocation = false;
      editingLocation = null;
      toast.success('Location updated');
      await loadLocations();
    } catch (e: any) {
      toast.error(`Failed to update location: ${e?.message || e}`);
    } finally {
      updatingLocation = false;
    }
  }

  function confirmDeleteLocation(row: CustomerLocation) {
    locationToDelete = row;
    showDeleteLocation = true;
  }

  async function doDeleteLocation() {
    const row = locationToDelete;
    if (!row) return;
    deletingLocation = true;
    try {
      await api.customers.locations.delete(row.id);
      showDeleteLocation = false;
      locationToDelete = null;
      toast.success('Location deleted');
      await loadLocations();
    } catch (e: any) {
      toast.error(`Failed to delete location: ${e?.message || e}`);
    } finally {
      deletingLocation = false;
    }
  }

  async function doDeleteCustomer() {
    if (!customer) return;
    deletingCustomer = true;
    try {
      await api.customers.delete(customer.id);
      toast.success(get(t)('admin.customers.toasts.deleted') || 'Customer deleted');
      goto('..');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.toasts.delete_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    } finally {
      deletingCustomer = false;
      showDeleteCustomer = false;
    }
  }
</script>

<div class="page-content fade-in">
  <div class="customer-hero card">
    <div class="hero-top">
      <button class="btn btn-secondary" onclick={() => goto('..')}>
        <Icon name="arrow-left" size={16} />
        {$t('common.back') || 'Back'}
      </button>
      <div class="header-actions">
        {#if $can('manage', 'customers') && customer}
          {#if customer.is_active}
            <button
              class="btn btn-warning"
              onclick={() => setCustomerActive(false)}
              disabled={togglingCustomerStatus}
            >
              <Icon name="pause" size={16} />
              Suspend
            </button>
          {:else}
            <button
              class="btn btn-primary"
              onclick={() => setCustomerActive(true)}
              disabled={togglingCustomerStatus}
            >
              <Icon name="play" size={16} />
              Activate
            </button>
          {/if}
        {/if}
        <button
          class="btn btn-secondary"
          onclick={refreshCurrent}
        >
          <Icon name="refresh-cw" size={16} />
          {$t('common.refresh') || 'Refresh'}
        </button>
        {#if $can('manage', 'customers')}
          <button class="btn btn-danger" onclick={() => (showDeleteCustomer = true)}>
            <Icon name="trash-2" size={16} />
            {$t('common.delete') || 'Delete'}
          </button>
        {/if}
      </div>
    </div>

    <div class="hero-main">
      <div class="avatar">
        {(customer?.name || '?')
          .split(' ')
          .filter(Boolean)
          .slice(0, 2)
          .map((s) => s[0]?.toUpperCase() || '')
          .join('')}
      </div>
      <div class="meta">
        <h1>{customer?.name || $t('admin.customers.detail.title') || 'Customer'}</h1>
        <p class="subtitle">
          {customer?.email || customer?.phone || ($t('admin.customers.detail.subtitle') || 'Customer details')}
        </p>
        <div class="hero-badges">
          <span class={`status-pill ${customer?.is_active ? 'is-active' : 'is-inactive'}`}>
            <span class="dot"></span>
            {customer?.is_active ? ($t('common.active') || 'Active') : ($t('common.inactive') || 'Inactive')}
          </span>
          <span class="meta-pill">
            <Icon name="clock" size={14} />
            {customer?.updated_at ? `Updated ${timeAgo(customer.updated_at)}` : '-'}
          </span>
        </div>
      </div>
    </div>

  </div>

  <div class="tabs">
    <button class:active={activeTab === 'overview'} onclick={() => (activeTab = 'overview')}>
      {$t('admin.customers.tabs.overview') || 'Overview'}
    </button>
    <button class:active={activeTab === 'locations'} onclick={() => (activeTab = 'locations')}>
      {$t('admin.customers.tabs.locations') || 'Locations'}
    </button>
    <button class:active={activeTab === 'subscriptions'} onclick={() => (activeTab = 'subscriptions')}>
      {$t('admin.customers.tabs.subscriptions') || 'Subscriptions'}
    </button>
    <button class:active={activeTab === 'billing'} onclick={() => (activeTab = 'billing')}>
      {$t('admin.customers.tabs.billing') || 'Billing'}
    </button>
    {#if $can('read', 'pppoe') || $can('manage', 'pppoe')}
      <button class:active={activeTab === 'pppoe'} onclick={() => (activeTab = 'pppoe')}>
        {$t('admin.customers.tabs.pppoe') || 'PPPoE'}
      </button>
    {/if}
    {#if canReadAudit}
      <button class:active={activeTab === 'timeline'} onclick={() => (activeTab = 'timeline')}>
        Timeline
      </button>
    {/if}
  </div>

  {#if loadingCustomer}
    <div class="card loading-card">
      <div class="spinner"></div>
      <p>{$t('common.loading') || 'Loading...'}</p>
    </div>
  {:else if customer}
    {#if activeTab === 'overview'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.overview.title') || 'Customer profile'}</h3>
            <p class="subtitle">Primary identity and contact data used for billing and support.</p>
          </div>
          {#if $can('manage', 'customers')}
            <button class="btn btn-primary" onclick={saveOverview} disabled={saving || !name.trim()}>
              <Icon name="check-circle" size={16} />
              {$t('common.save') || 'Save'}
            </button>
          {/if}
        </div>

        <div class="overview-grid">
          <div class="form">
            <label>
              <span>{$t('admin.customers.fields.name') || 'Name'}</span>
              <input class="input" bind:value={name} disabled={!$can('manage', 'customers')} />
            </label>
            <div class="grid2">
              <label>
                <span>{$t('admin.customers.fields.email') || 'Email'}</span>
                <input class="input" bind:value={email} disabled={!$can('manage', 'customers')} />
              </label>
              <label>
                <span>{$t('admin.customers.fields.phone') || 'Phone'}</span>
                <input class="input" bind:value={phone} disabled={!$can('manage', 'customers')} />
              </label>
            </div>
            <label>
              <span>{$t('admin.customers.fields.notes') || 'Notes'}</span>
              <textarea
                class="input"
                rows="5"
                bind:value={notes}
                disabled={!$can('manage', 'customers')}
              ></textarea>
            </label>
          </div>
          <aside class="overview-side">
            <div class="side-title">Profile quality</div>
            <div class="side-item">
              <span>Name</span>
              <strong>{name.trim() ? 'Complete' : 'Missing'}</strong>
            </div>
            <div class="side-item">
              <span>Email</span>
              <strong>{email.trim() ? 'Complete' : 'Missing'}</strong>
            </div>
            <div class="side-item">
              <span>Phone</span>
              <strong>{phone.trim() ? 'Complete' : 'Missing'}</strong>
            </div>
            <div class="side-item">
              <span>Status</span>
              <strong>{isActive ? 'Active' : 'Inactive'}</strong>
            </div>
            <div class="side-divider"></div>
            <p class="side-note">
              Keep customer identity and contacts accurate to avoid billing and support issues.
            </p>
          </aside>
        </div>
      </div>
    {:else if activeTab === 'locations'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.locations.title') || 'Locations'}</h3>
            <p class="subtitle">
              {$t('admin.customers.locations.subtitle') || 'Service locations under this customer.'}
            </p>
          </div>
          {#if $can('manage', 'customer_locations')}
            <button class="btn btn-primary" onclick={openCreateLocation}>
              <Icon name="plus" size={16} />
              {$t('admin.customers.locations.actions.add') || 'Add location'}
            </button>
          {/if}
        </div>

        <Table
          columns={locColumns}
          data={locations}
          loading={loadingLocations}
          emptyText={$t('admin.customers.locations.empty') || 'No locations yet.'}
          pagination
        >
          {#snippet cell({ item, key })}
            {@const loc = item as CustomerLocation}
            {#if key === 'label'}
              <div class="name">{loc.label}</div>
              <div class="sub">{loc.city || ''}</div>
            {:else if key === 'address'}
              <div>{loc.address_line1 || '-'}</div>
              <div class="sub">
                {[loc.city, loc.state, loc.postal_code, loc.country].filter(Boolean).join(', ') || '-'}
              </div>
            {:else if key === 'updated_at'}
              <span class="mono">{new Date(loc.updated_at).toLocaleString()}</span>
            {:else if key === 'actions'}
              <div class="row-actions">
                <button class="btn-icon" title={$t('common.refresh') || 'Refresh'} onclick={loadLocations}>
                  <Icon name="refresh-cw" size={16} />
                </button>
                {#if $can('manage', 'customer_locations')}
                  <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEditLocation(loc)}>
                    <Icon name="edit-3" size={16} />
                  </button>
                  <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => confirmDeleteLocation(loc)}>
                    <Icon name="trash-2" size={16} />
                  </button>
                {/if}
              </div>
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>
    {:else if activeTab === 'subscriptions'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.subscriptions.title') || 'Subscriptions'}</h3>
            <p class="subtitle">
              {$t('admin.customers.subscriptions.subtitle') ||
                'Customer package subscriptions for billing and service assignment.'}
            </p>
          </div>
          <div class="header-actions">
            <button class="btn btn-secondary" onclick={loadSubscriptions} disabled={loadingSubscriptions}>
              <Icon name="refresh-cw" size={16} />
              {$t('common.refresh') || 'Refresh'}
            </button>
            {#if $can('manage', 'customers')}
              <button class="btn btn-primary" onclick={openCreateSubscription}>
                <Icon name="plus" size={16} />
                {$t('common.add') || 'Add'}
              </button>
            {/if}
          </div>
        </div>

        <Table
          columns={subscriptionColumns}
          data={subscriptions}
          loading={loadingSubscriptions}
          emptyText={$t('admin.customers.subscriptions.empty') || 'No subscriptions yet.'}
          pagination
        >
          {#snippet cell({ item, key })}
            {@const row = item as CustomerSubscriptionView}
            {#if key === 'package'}
              <div class="name">{row.package_name || row.package_id}</div>
              <div class="sub">{row.status}</div>
            {:else if key === 'billing'}
              <div class="name">{row.billing_cycle}</div>
              <div class="sub mono">{row.currency_code} {Number(row.price || 0).toLocaleString()}</div>
            {:else if key === 'location'}
              <div>{row.location_label || '-'}</div>
            {:else if key === 'router'}
              <div>{row.router_name || '-'}</div>
            {:else if key === 'period'}
              <div class="sub">{row.starts_at ? new Date(row.starts_at).toLocaleDateString() : '-'}</div>
              <div class="sub">{row.ends_at ? new Date(row.ends_at).toLocaleDateString() : '-'}</div>
            {:else if key === 'actions'}
              <div class="row-actions">
                {#if $can('manage', 'customers')}
                  <button
                    class="btn-icon"
                    title={$t('admin.customers.billing.actions.generate_from_subscription') || 'Generate invoice'}
                    onclick={() => generateInvoiceForSubscription(row.id)}
                    disabled={generatingInvoiceFor === row.id || deletingSubscription === row.id}
                  >
                    <Icon name="file-text" size={16} />
                  </button>
                  {#if row.status === 'active'}
                    <button
                      class="btn-icon"
                      title="Suspend"
                      onclick={() => setSubscriptionStatus(row, 'suspended')}
                      disabled={togglingSubscription === row.id || deletingSubscription === row.id}
                    >
                      <Icon name="pause" size={16} />
                    </button>
                  {:else if row.status === 'suspended'}
                    <button
                      class="btn-icon"
                      title="Resume"
                      onclick={() => setSubscriptionStatus(row, 'active')}
                      disabled={togglingSubscription === row.id || deletingSubscription === row.id}
                    >
                      <Icon name="play" size={16} />
                    </button>
                  {/if}
                  <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEditSubscription(row)}>
                    <Icon name="edit-3" size={16} />
                  </button>
                  <button
                    class="btn-icon danger"
                    title={$t('common.delete') || 'Delete'}
                    onclick={() => deleteSubscription(row.id)}
                    disabled={deletingSubscription === row.id}
                  >
                    <Icon name="trash-2" size={16} />
                  </button>
                {/if}
              </div>
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>
    {:else if activeTab === 'billing'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.billing.title') || 'Billing'}</h3>
            <p class="subtitle">
              {$t('admin.customers.billing.subtitle') ||
                'Invoice history generated from this customer subscriptions.'}
            </p>
          </div>
          <div class="header-actions">
            <label class="inline-filter">
              <span>{$t('admin.customers.billing.filters.status') || 'Status'}</span>
              <select class="input" bind:value={billingStatus}>
                <option value="all">{$t('admin.customers.billing.filters.all') || 'All'}</option>
                <option value="pending">{$t('admin.package_invoices.statuses.pending') || 'Pending'}</option>
                <option value="verification_pending">
                  {$t('admin.package_invoices.statuses.verification_pending') || 'Verification pending'}
                </option>
                <option value="paid">{$t('admin.package_invoices.statuses.paid') || 'Paid'}</option>
                <option value="failed">{$t('admin.package_invoices.statuses.failed') || 'Failed'}</option>
              </select>
            </label>
            <div class="quick-ranges">
              <button
                class="btn btn-secondary btn-quick"
                class:active={billingQuickRange === 'today'}
                onclick={() => applyBillingQuickRange('today')}
              >
                {$t('admin.customers.billing.filters.today') || 'Today'}
              </button>
              <button
                class="btn btn-secondary btn-quick"
                class:active={billingQuickRange === '7d'}
                onclick={() => applyBillingQuickRange('7d')}
              >
                {$t('admin.customers.billing.filters.last_7d') || '7D'}
              </button>
              <button
                class="btn btn-secondary btn-quick"
                class:active={billingQuickRange === '30d'}
                onclick={() => applyBillingQuickRange('30d')}
              >
                {$t('admin.customers.billing.filters.last_30d') || '30D'}
              </button>
              <button
                class="btn btn-secondary btn-quick"
                class:active={billingQuickRange === 'month'}
                onclick={() => applyBillingQuickRange('month')}
              >
                {$t('admin.customers.billing.filters.this_month') || 'This Month'}
              </button>
            </div>
            <label class="inline-filter">
              <span>{$t('admin.customers.billing.filters.from') || 'From'}</span>
              <input class="input" type="date" bind:value={billingDateFrom} oninput={onBillingDateChange} />
            </label>
            <label class="inline-filter">
              <span>{$t('admin.customers.billing.filters.to') || 'To'}</span>
              <input class="input" type="date" bind:value={billingDateTo} oninput={onBillingDateChange} />
            </label>
            <button
              class="btn btn-secondary"
              onclick={clearBillingFilters}
              disabled={billingStatus === 'all' && !billingDateFrom && !billingDateTo}
            >
              <Icon name="eraser" size={16} />
              {$t('admin.customers.billing.filters.clear') || 'Clear'}
            </button>
            <button class="btn btn-secondary" onclick={loadBillingInvoices} disabled={loadingBilling}>
              <Icon name="refresh-cw" size={16} />
              {$t('common.refresh') || 'Refresh'}
            </button>
          </div>
        </div>

        <div class="billing-stats">
          <div class="billing-stat">
            <div class="billing-stat-label">{$t('admin.customers.billing.stats.total') || 'Total invoices'}</div>
            <div class="billing-stat-value">{billingStats.total}</div>
          </div>
          <div class="billing-stat">
            <div class="billing-stat-label">{$t('admin.customers.billing.stats.unpaid') || 'Unpaid'}</div>
            <div class="billing-stat-value">{billingStats.unpaid}</div>
          </div>
          <div class="billing-stat">
            <div class="billing-stat-label">{$t('admin.customers.billing.stats.paid') || 'Paid'}</div>
            <div class="billing-stat-value">{billingStats.paid}</div>
          </div>
          <div class="billing-stat">
            <div class="billing-stat-label">{$t('admin.customers.billing.stats.overdue') || 'Overdue'}</div>
            <div class="billing-stat-value">{billingStats.overdue}</div>
          </div>
        </div>

        <Table
          columns={billingColumns}
          data={billingRows}
          loading={loadingBilling}
          emptyText={$t('admin.customers.billing.empty') || 'No invoices for this customer yet.'}
          pagination
        >
          {#snippet cell({ item, key })}
            {@const row = item as Invoice}
            {@const subscriptionId = getSubscriptionIdFromInvoice(row)}
            {@const subscription = subscriptionId ? subscriptionById.get(subscriptionId) : null}
            {#if key === 'invoice_number'}
              <div class="name">#{row.invoice_number}</div>
              <div class="sub mono">{row.created_at ? new Date(row.created_at).toLocaleString() : '-'}</div>
            {:else if key === 'subscription'}
              <div class="name">{subscription?.package_name || subscription?.package_id || '-'}</div>
              <div class="sub">{subscription?.billing_cycle || '-'}</div>
            {:else if key === 'amount'}
              <div class="name">{formatMoney(row.amount, { currency: row.currency_code || undefined })}</div>
            {:else if key === 'status'}
              <span class={`badge ${row.status === 'paid' ? 'ok' : row.status === 'failed' ? 'danger' : 'warn'}`}>
                {billingStatusLabel(row.status)}
              </span>
            {:else if key === 'due_date'}
              <div class="name">{new Date(row.due_date).toLocaleDateString()}</div>
              <div class="sub mono">{new Date(row.due_date).toLocaleTimeString()}</div>
            {:else if key === 'actions'}
              <div class="row-actions">
                <button
                  class="btn-icon"
                  title={$t('admin.package_invoices.list.actions.view_details') || 'View details'}
                  onclick={() => openInvoiceDetail(row.id)}
                >
                  <Icon name="eye" size={16} />
                </button>
              </div>
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>
    {:else if activeTab === 'pppoe'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.pppoe.title') || 'PPPoE accounts'}</h3>
            <p class="subtitle">
              {$t('admin.customers.pppoe.subtitle') ||
                'Manage PPPoE secrets for this customer (per-router). The database is the source of truth.'}
            </p>
          </div>
          <div class="header-actions">
            <div class="search">
              <Icon name="search" size={16} />
              <input
                class="input"
                bind:value={pppoeQuery}
                placeholder={$t('admin.customers.pppoe.search') || 'Search username...'}
                oninput={() => void loadPppoeAccounts()}
              />
            </div>
            <button class="btn btn-secondary" onclick={loadPppoeAccounts} disabled={loadingPppoe}>
              <Icon name="refresh-cw" size={16} />
              {$t('common.refresh') || 'Refresh'}
            </button>
            {#if $can('manage', 'pppoe')}
              <button
                class="btn btn-secondary"
                onclick={reconcilePppoeRouters}
                disabled={loadingPppoe || pppoeAccounts.length === 0}
                title={$t('admin.customers.pppoe.actions.reconcile_hint') || 'Mark which accounts exist on the router'}
              >
                <Icon name="refresh-cw" size={16} />
                {$t('admin.customers.pppoe.actions.reconcile') || 'Reconcile'}
              </button>
            {/if}
            {#if $can('manage', 'pppoe')}
              <button class="btn btn-primary" onclick={openCreatePppoe} disabled={loadingPppoeRouters}>
                <Icon name="plus" size={16} />
                {$t('admin.customers.pppoe.actions.add') || 'Add PPPoE'}
              </button>
            {/if}
          </div>
        </div>

        <Table
          columns={pppoeColumns}
          data={pppoeAccounts}
          loading={loadingPppoe}
          emptyText={$t('admin.customers.pppoe.empty') || 'No PPPoE accounts yet.'}
          pagination
        >
          {#snippet cell({ item, key })}
            {@const row = item as PppoeAccountPublic}
            {@const routerName = pppoeRouters.find((r) => r.id === row.router_id)?.name || '-'}
            {@const locName = locations.find((l) => l.id === row.location_id)?.label || '-'}
            {#if key === 'username'}
              <div class="name">{row.username}</div>
              <div class="sub mono">{row.disabled ? ($t('common.disabled') || 'Disabled') : ($t('common.active') || 'Active')}</div>
            {:else if key === 'router'}
              <div class="name">{routerName}</div>
              <div class="sub mono">{row.router_id}</div>
            {:else if key === 'location'}
              <div class="name">{locName}</div>
              <div class="sub mono">{row.location_id}</div>
            {:else if key === 'assignment'}
              <div class="sub">
                <span class="pill">{$t('admin.customers.pppoe.fields.profile') || 'Profile'}: {row.router_profile_name || '-'}</span>
                <span class="pill">{$t('admin.customers.pppoe.fields.remote_address') || 'Remote'}: {row.remote_address || row.address_pool || '-'}</span>
              </div>
            {:else if key === 'sync'}
              <div class="sub">
                {#if row.router_present}
                  <span class="badge ok">{$t('admin.customers.pppoe.sync.present') || 'On router'}</span>
                {:else}
                  <span class="badge warn">{$t('admin.customers.pppoe.sync.missing') || 'Missing'}</span>
                {/if}
                <span class="mono">{row.last_sync_at ? timeAgo(row.last_sync_at) : '-'}</span>
              </div>
              {#if row.last_error}
                <div class="sub error">{row.last_error}</div>
              {/if}
            {:else if key === 'actions'}
              <div class="row-actions">
                {#if $can('manage', 'pppoe')}
                  <button class="btn-icon" title={$t('admin.customers.pppoe.actions.apply') || 'Apply to router'} onclick={() => applyPppoe(row)}>
                    <Icon name="send" size={16} />
                  </button>
                  <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEditPppoe(row)}>
                    <Icon name="edit" size={16} />
                  </button>
                  <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => deletePppoe(row)}>
                    <Icon name="trash-2" size={16} />
                  </button>
                {/if}
              </div>
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>
    {:else if activeTab === 'timeline'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>Timeline</h3>
            <p class="subtitle">Recent customer activity and audit history.</p>
          </div>
          <button class="btn btn-secondary" onclick={loadTimeline} disabled={loadingTimeline}>
            <Icon name="refresh-cw" size={16} />
            {$t('common.refresh') || 'Refresh'}
          </button>
        </div>
        <div class="timeline-filters">
          <button class:active={timelineType === 'all'} onclick={() => (timelineType = 'all')}>All</button>
          <button class:active={timelineType === 'customer'} onclick={() => (timelineType = 'customer')}>Profile</button>
          <button class:active={timelineType === 'location'} onclick={() => (timelineType = 'location')}>Location</button>
          <button class:active={timelineType === 'subscription'} onclick={() => (timelineType = 'subscription')}>Subscription</button>
        </div>
        {#if loadingTimeline}
          <div class="loading-card">
            <div class="spinner"></div>
            <p>{$t('common.loading') || 'Loading...'}</p>
          </div>
        {:else if timelineFilteredLogs.length === 0}
          <div class="sub">No timeline yet.</div>
        {:else}
          <div class="timeline-list">
            {#each timelineFilteredLogs as log (log.id)}
              <div class="timeline-item">
                <div class="timeline-main">
                  <div class="name">{timelineActionLabel(log.action)}</div>
                  <div class="sub">
                    {timelineActorLabel(log)} · {new Date(log.created_at).toLocaleString()}
                  </div>
                </div>
                <div class="timeline-meta">
                  <span class="pill">{timelineResourceLabel(log.resource)}</span>
                </div>
                {#if log.details}
                  <div class="timeline-details">{log.details}</div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<Modal
  show={showAddPppoe}
  title={$t('admin.customers.pppoe.new.title') || 'Add PPPoE account'}
  onclose={() => (showAddPppoe = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
        <Select2
          bind:value={pppoeRouterId}
          options={pppoeRouters.map((r) => ({ label: r.name, value: r.id }))}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={loadingPppoeRouters}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => {
            pppoePackageId = '';
            pppoeRouterProfileName = '';
            pppoeRemoteAddress = '';
            pppoeAddressPool = '';
          }}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.location') || 'Location'}</span>
        <Select2
          bind:value={pppoeLocationId}
          options={locations.map((l) => ({ label: l.label, value: l.id }))}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.package') || 'Package'}</span>
      <Select2
        bind:value={pppoePackageId}
        options={pppoePackageOptions}
        placeholder={($t('common.select') || 'Select') + '...'}
        width="100%"
        disabled={!pppoeRouterId || pppoePackageOptions.length === 0}
        maxItems={5000}
        searchPlaceholder={$t('common.search') || 'Search'}
        noResultsText={$t('common.no_results') || 'No results'}
        onchange={() => applyPppoePackage(pppoePackageId)}
      />
      <div class="field-hint">
        {$t('admin.network.pppoe.form.package_hint') ||
          'If you select a package, profile/pool will be prefilled for the selected router (you can still override).'}
      </div>
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
        <input class="input" bind:value={pppoeUsername} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input class="input" type="password" bind:value={pppoePassword} />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.profile') || 'Profile'}</span>
        <Select2
          bind:value={pppoeRouterProfileName}
          options={pppoeProfileOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!pppoeRouterId || pppoeProfileOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => maybeAutoSelectPppoePackageFromProfile()}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.remote_address') || 'Remote IP'}</span>
        <input
          class="input mono"
          bind:value={pppoeRemoteAddress}
          placeholder="10.10.10.10"
          disabled={!pppoeRouterId}
        />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.pool') || 'Address pool'}</span>
        <Select2
          bind:value={pppoeAddressPool}
          options={pppoePoolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!pppoeRouterId || pppoePoolOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      <div></div>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.pool') || 'Address pool'}</span>
        <Select2
          bind:value={pppoeAddressPool}
          options={pppoePoolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!pppoeRouterId || pppoePoolOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      <div></div>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={pppoeComment} />
    </label>

    <div class="toggle-row">
      <div class="toggle-text">
        <div class="toggle-title">{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</div>
        <div class="toggle-sub">
          {$t('admin.network.pppoe.form.disabled_hint') ||
            'Disable this PPPoE account (will be applied to router when you click Apply).'}
        </div>
      </div>
      <Toggle bind:checked={pppoeDisabled} ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'} />
    </div>

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showAddPppoe = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={submitCreatePppoe}
        disabled={savingPppoe || !pppoeRouterId || !pppoeLocationId || !pppoeUsername.trim() || !pppoePassword}
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showEditPppoe}
  title={$t('admin.customers.pppoe.edit.title') || 'Edit PPPoE account'}
  onclose={() => (showEditPppoe = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
        <Select2
          bind:value={pppoeRouterId}
          options={pppoeRouters.map((r) => ({ label: r.name, value: r.id }))}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={loadingPppoeRouters}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => {
            pppoePackageId = '';
            pppoeRouterProfileName = '';
            pppoeRemoteAddress = '';
            pppoeAddressPool = '';
          }}
        />
      </label>
      <div></div>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.package') || 'Package'}</span>
      <Select2
        bind:value={pppoePackageId}
        options={pppoePackageOptions}
        placeholder={($t('common.select') || 'Select') + '...'}
        width="100%"
        disabled={!pppoeRouterId || pppoePackageOptions.length === 0}
        maxItems={5000}
        searchPlaceholder={$t('common.search') || 'Search'}
        noResultsText={$t('common.no_results') || 'No results'}
        onchange={() => applyPppoePackage(pppoePackageId)}
      />
      <div class="field-hint">
        {$t('admin.network.pppoe.form.package_hint') ||
          'If you select a package, profile/pool will be prefilled for the selected router (you can still override).'}
      </div>
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
        <input class="input" bind:value={pppoeUsername} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input class="input" type="password" bind:value={pppoePassword} placeholder={$t('admin.customers.pppoe.edit.password_hint') || 'Leave blank to keep'} />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.profile') || 'Profile'}</span>
        <Select2
          bind:value={pppoeRouterProfileName}
          options={pppoeProfileOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!pppoeRouterId || pppoeProfileOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => maybeAutoSelectPppoePackageFromProfile()}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.remote_address') || 'Remote IP'}</span>
        <input class="input mono" bind:value={pppoeRemoteAddress} placeholder="10.10.10.10" />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.pool') || 'Address pool'}</span>
        <Select2
          bind:value={pppoeAddressPool}
          options={pppoePoolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!pppoeRouterId || pppoePoolOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      <div></div>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={pppoeComment} />
    </label>

    <div class="toggle-row">
      <div class="toggle-text">
        <div class="toggle-title">{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</div>
        <div class="toggle-sub">
          {$t('admin.network.pppoe.form.disabled_hint') ||
            'Disable this PPPoE account (will be applied to router when you click Apply).'}
        </div>
      </div>
      <Toggle bind:checked={pppoeDisabled} ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'} />
    </div>

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showEditPppoe = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" onclick={submitUpdatePppoe} disabled={savingPppoe || !pppoeUsername.trim()}>
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showAddSubscription}
  title={$t('admin.customers.subscriptions.new.title') || 'Add subscription'}
  onclose={() => (showAddSubscription = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.location') || 'Location'}</span>
        <Select2
          bind:value={subLocationId}
          options={subscriptionLocationOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.package') || 'Package'}</span>
        <Select2
          bind:value={subPackageId}
          options={subscriptionPackageOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.router') || 'Router (optional)'}</span>
        <Select2
          bind:value={subRouterId}
          options={subscriptionRouterOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.billing_cycle') || 'Billing cycle'}</span>
        <Select2 bind:value={subBillingCycle} options={billingCycleOptions} width="100%" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.price') || 'Price'}</span>
        <input class="input" type="number" min="0" step="0.01" bind:value={subPrice} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.currency') || 'Currency'}</span>
        <input class="input" bind:value={subCurrency} placeholder="IDR" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.status') || 'Status'}</span>
        <Select2 bind:value={subStatus} options={subscriptionStatusOptions} width="100%" />
      </label>
      <div></div>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.starts_at') || 'Starts at'}</span>
        <input class="input" type="date" bind:value={subStartsAt} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.ends_at') || 'Ends at'}</span>
        <input class="input" type="date" bind:value={subEndsAt} />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.subscriptions.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={subNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showAddSubscription = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={submitCreateSubscription}
        disabled={savingSubscription || !subLocationId || !subPackageId || !subPrice}
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showEditSubscription}
  title={$t('admin.customers.subscriptions.edit.title') || 'Edit subscription'}
  onclose={() => {
    showEditSubscription = false;
    editingSubscription = null;
  }}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.location') || 'Location'}</span>
        <Select2
          bind:value={subLocationId}
          options={subscriptionLocationOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.package') || 'Package'}</span>
        <Select2
          bind:value={subPackageId}
          options={subscriptionPackageOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.router') || 'Router (optional)'}</span>
        <Select2
          bind:value={subRouterId}
          options={subscriptionRouterOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.billing_cycle') || 'Billing cycle'}</span>
        <Select2 bind:value={subBillingCycle} options={billingCycleOptions} width="100%" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.price') || 'Price'}</span>
        <input class="input" type="number" min="0" step="0.01" bind:value={subPrice} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.currency') || 'Currency'}</span>
        <input class="input" bind:value={subCurrency} placeholder="IDR" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.status') || 'Status'}</span>
        <Select2 bind:value={subStatus} options={subscriptionStatusOptions} width="100%" />
      </label>
      <div></div>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.starts_at') || 'Starts at'}</span>
        <input class="input" type="date" bind:value={subStartsAt} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.ends_at') || 'Ends at'}</span>
        <input class="input" type="date" bind:value={subEndsAt} />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.subscriptions.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={subNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showEditSubscription = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={submitUpdateSubscription}
        disabled={savingSubscription || !subLocationId || !subPackageId || !subPrice}
      >
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showAddLocation}
  title={$t('admin.customers.locations.new.title') || 'Add location'}
  onclose={() => (showAddLocation = false)}
>
  <div class="form">
    <label>
      <span>{$t('admin.customers.locations.fields.label') || 'Label'}</span>
      <input class="input" bind:value={locLabel} placeholder="Site A / Rumah / Kantor" />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address1') || 'Address line 1'}</span>
      <input class="input" bind:value={locAddress1} />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address2') || 'Address line 2'}</span>
      <input class="input" bind:value={locAddress2} />
    </label>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.city') || 'City'}</span>
        <input class="input" bind:value={locCity} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.state') || 'State'}</span>
        <input class="input" bind:value={locState} />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.postal') || 'Postal code'}</span>
        <input class="input" bind:value={locPostal} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.country') || 'Country'}</span>
        <input class="input" bind:value={locCountry} />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.locations.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={locNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showAddLocation = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={addLocation}
        disabled={creatingLocation || !locLabel.trim()}
      >
        <Icon name="plus" size={16} />
        {$t('common.add') || 'Add'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showEditLocation}
  title={$t('admin.customers.locations.edit.title') || 'Edit location'}
  onclose={() => (showEditLocation = false)}
>
  <div class="form">
    <label>
      <span>{$t('admin.customers.locations.fields.label') || 'Label'}</span>
      <input class="input" bind:value={locLabel} placeholder="Site A / Rumah / Kantor" />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address1') || 'Address line 1'}</span>
      <input class="input" bind:value={locAddress1} />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address2') || 'Address line 2'}</span>
      <input class="input" bind:value={locAddress2} />
    </label>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.city') || 'City'}</span>
        <input class="input" bind:value={locCity} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.state') || 'State'}</span>
        <input class="input" bind:value={locState} />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.postal') || 'Postal code'}</span>
        <input class="input" bind:value={locPostal} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.country') || 'Country'}</span>
        <input class="input" bind:value={locCountry} />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.locations.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={locNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showEditLocation = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={submitUpdateLocation}
        disabled={updatingLocation || !locLabel.trim()}
      >
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<ConfirmDialog
  show={showDeleteCustomer}
  title={$t('admin.customers.delete.title') || 'Delete customer'}
  message={$t('admin.customers.delete.message') || 'This will remove the customer and all related data.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingCustomer}
  onconfirm={doDeleteCustomer}
  oncancel={() => (showDeleteCustomer = false)}
/>

<ConfirmDialog
  show={showDeleteLocation}
  title={$t('admin.customers.locations.delete.title') || 'Delete location'}
  message={$t('admin.customers.locations.delete.message') || 'This location will be removed.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingLocation}
  onconfirm={doDeleteLocation}
  oncancel={() => (showDeleteLocation = false)}
/>

<style>
  .page-content {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  .customer-hero {
    margin-bottom: 1rem;
    padding: 1rem 1.05rem;
    background: var(--bg-surface);
  }

  .hero-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.9rem;
  }

  .hero-main {
    display: flex;
    align-items: center;
    gap: 0.95rem;
  }

  .avatar {
    width: 52px;
    height: 52px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    font-weight: 800;
    letter-spacing: 0.4px;
    color: #e0e7ff;
    background:
      linear-gradient(145deg, rgba(79, 70, 229, 0.95), rgba(99, 102, 241, 0.6)),
      rgba(79, 70, 229, 0.5);
    border: 1px solid rgba(129, 140, 248, 0.45);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22);
  }

  .meta h1 {
    margin: 0;
    font-size: 1.65rem;
    letter-spacing: -0.02em;
  }

  .hero-badges {
    margin-top: 0.5rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .status-pill,
  .meta-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border-radius: 999px;
    padding: 0.28rem 0.62rem;
    font-size: 0.8rem;
    font-weight: 700;
    border: none;
    background: color-mix(in srgb, var(--bg-surface), transparent 12%);
    color: var(--text-secondary);
  }

  .status-pill.is-active {
    border-color: rgba(16, 185, 129, 0.35);
    color: rgb(52, 211, 153);
    background: rgba(16, 185, 129, 0.1);
  }

  .status-pill.is-inactive {
    border-color: rgba(251, 191, 36, 0.35);
    color: rgb(252, 211, 77);
    background: rgba(234, 179, 8, 0.1);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .title {
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.55rem 0.9rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
    transition: background 0.15s ease, border-color 0.15s ease, transform 0.02s ease;
    user-select: none;
  }

  .btn:hover {
    background: var(--bg-hover);
  }

  .btn:active {
    transform: translateY(1px);
  }

  .btn:disabled {
    opacity: 0.7;
    cursor: wait;
  }

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-primary:hover {
    background: rgba(99, 102, 241, 1);
  }

  .btn-secondary {
    background: var(--bg-surface);
  }

  .btn-danger {
    border-color: rgba(239, 68, 68, 0.35);
    background: rgba(239, 68, 68, 0.1);
    color: rgb(239, 68, 68);
  }

  .btn-danger:hover {
    background: rgba(239, 68, 68, 0.14);
  }

  .btn-warning {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.14);
    color: rgb(251, 191, 36);
  }

  .btn-warning:hover {
    background: rgba(245, 158, 11, 0.2);
  }

  .tabs {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
  }

  .tabs button {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.45rem 0.85rem;
    cursor: pointer;
    font-weight: 650;
    font-size: 0.9rem;
  }

  .tabs button.active {
    border-color: rgba(99, 102, 241, 0.55);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.12);
  }

  .section {
    padding: 1.1rem;
    background: var(--bg-surface);
  }

  .section-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .form {
    display: grid;
    gap: 0.9rem;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 0.9rem;
  }

  .overview-side {
    border-radius: 14px;
    padding: 0.88rem 0.9rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 10%);
    height: fit-content;
  }

  .side-title {
    font-weight: 760;
    margin-bottom: 0.65rem;
  }

  .side-item {
    display: flex;
    justify-content: space-between;
    gap: 0.7rem;
    margin-bottom: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .side-item strong {
    color: var(--text-primary);
    font-size: 0.86rem;
  }

  .side-divider {
    border-top: 1px solid color-mix(in srgb, var(--border-color), transparent 35%);
    margin: 0.75rem 0;
  }

  .side-note {
    margin: 0;
    font-size: 0.84rem;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  label > span {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.65rem 0.75rem;
    outline: none;
  }

  textarea.input {
    resize: vertical;
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.9rem;
    padding: 0.85rem 0.95rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .toggle-text {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }

  .toggle-title {
    color: var(--text-primary);
    font-weight: 800;
  }

  .toggle-sub {
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 0.9rem;
    line-height: 1.35;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .inline-filter {
    display: grid;
    gap: 0.3rem;
    min-width: 180px;
  }

  .quick-ranges {
    display: flex;
    align-items: flex-end;
    gap: 0.45rem;
  }

  .btn-quick {
    min-height: 40px;
    padding-inline: 0.7rem;
    border-radius: 10px;
  }

  .btn-quick.active {
    border-color: rgba(99, 102, 241, 0.5);
    background: rgba(99, 102, 241, 0.14);
    color: #e0e7ff;
  }

  .inline-filter span {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .billing-stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.65rem;
    margin-bottom: 0.85rem;
  }

  .billing-stat {
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface), transparent 9%);
    padding: 0.65rem 0.75rem;
  }

  .billing-stat-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin-bottom: 0.2rem;
  }

  .billing-stat-value {
    font-weight: 800;
    font-size: 1.1rem;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-icon {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.4rem 0.45rem;
    cursor: pointer;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  .badge.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
    background: rgba(239, 68, 68, 0.1);
  }

  .name {
    font-weight: 650;
  }

  .sub {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-top: 0.15rem;
  }

  .field-hint {
    margin-top: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .loading-card {
    padding: 1.25rem;
    display: grid;
    place-items: center;
    gap: 0.5rem;
  }

  .spinner {
    width: 26px;
    height: 26px;
    border-radius: 999px;
    border: 3px solid rgba(148, 163, 184, 0.3);
    border-top-color: rgba(99, 102, 241, 0.9);
    animation: spin 0.9s linear infinite;
  }

  .callout {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    padding: 0.75rem 0.9rem;
    border: 1px solid rgba(148, 163, 184, 0.35);
    border-radius: 12px;
    background: rgba(148, 163, 184, 0.08);
    color: var(--text-primary);
  }

  .timeline-list {
    display: grid;
    gap: 0.7rem;
  }

  .timeline-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    margin-bottom: 0.75rem;
  }

  .timeline-filters button {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.28rem 0.65rem;
    font-size: 0.82rem;
    font-weight: 650;
    cursor: pointer;
  }

  .timeline-filters button.active {
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.45);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.1);
  }

  .timeline-item {
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .timeline-main {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.6rem;
  }

  .timeline-meta {
    margin-top: 0.3rem;
  }

  .timeline-details {
    margin-top: 0.45rem;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  @keyframes spin {
    from {
      transform: rotate(0);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 1rem;
    }
    .hero-top {
      align-items: stretch;
      flex-direction: column;
    }
    .hero-main {
      align-items: flex-start;
    }
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
    .overview-grid {
      grid-template-columns: 1fr;
    }
    .grid2 {
      grid-template-columns: 1fr;
    }
    .billing-stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .quick-ranges {
      width: 100%;
      justify-content: flex-start;
      flex-wrap: wrap;
    }
  }
</style>
