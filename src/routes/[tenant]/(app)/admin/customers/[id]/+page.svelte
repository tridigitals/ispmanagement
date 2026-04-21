<script lang="ts">
  import { onMount, untrack } from 'svelte';
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
    type CustomerLifecycleObservability,
    type CustomerLocation,
    type CustomerSubscriptionView,
    type Invoice,
    type IspPackageRouterMappingView,
  } from '$lib/api/client';
  import type { PppoeAccountPublic } from '$lib/api/client';
  import { getPppoeAssignmentPayload } from '$lib/utils/pppoePackageAssignment';
  import {
    getCustomerDetailAutoLoadKey,
    normalizeCustomerDetailTab,
    readCustomerDetailTabFromUrlValue,
    getVisibleCustomerDetailTabs,
    shouldAutoLoadCustomerDetailTab,
    type CustomerDetailTab,
  } from '$lib/utils/customerDetailAccess';
  import {
    getPppoeApplyActionFallback,
    getPppoeProvisioningTargetFallback,
    getPppoeSyncDisplay,
  } from '$lib/utils/pppoeSource';
  import { getCustomerPppoeToolbarConfig } from '$lib/utils/customerPppoeToolbar';
  import { buildCustomerTimelineRows } from '$lib/utils/customerTimelineTable';
  import { timeAgo } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import { getAdminCustomerNavigation } from '$lib/utils/adminCustomerNavigation';
  import {
    formatLocationCoordinates,
    validateOptionalCoordinates,
  } from '$lib/utils/customerLocationCoordinates';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Table from '$lib/components/ui/Table.svelte';

  const customerId = $derived(String($page.params.id || ''));
  const customerNav = $derived.by(() =>
    getAdminCustomerNavigation({
      hostname: $page.url.hostname,
      tenantSlug: $page.data?.tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const customersPath = $derived(customerNav.customersPath);

  let activeTab = $state<CustomerDetailTab>('overview');

  let customer = $state<Customer | null>(null);
  let loadingCustomer = $state(true);

  let locations = $state<CustomerLocation[]>([]);
  let loadingLocations = $state(false);

  // Subscriptions
  let subscriptions = $state<CustomerSubscriptionView[]>([]);
  let loadingSubscriptions = $state(false);
  let lifecycleObservability = $state<CustomerLifecycleObservability | null>(null);
  let loadingLifecycleObservability = $state(false);
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
  let billingLoadInFlight = false;
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
  let showEditPppoe = $state(false);
  let editingPppoe = $state<PppoeAccountPublic | null>(null);
  let savingPppoe = $state(false);

  let pppoeRouterId = $state('');
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

  const pppoePackageSelectionHasMissingMapping = $derived.by(
    () =>
      Boolean(pppoePackageId) &&
      !getPppoeAssignmentPayload({
        packageId: pppoePackageId,
        mappings: pppoePackageMappings,
        current: {
          router_profile_name: pppoeRouterProfileName,
          remote_address: pppoeRemoteAddress,
          address_pool: pppoeAddressPool,
        },
      }).hasPackageMapping,
  );

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
  let locLatitude = $state('');
  let locLongitude = $state('');
  let locNotes = $state('');

  // Deletes
  let showDeleteCustomer = $state(false);
  let deletingCustomer = $state(false);

  const locColumns = $derived.by(() => [
    { key: 'label', label: $t('admin.customers.locations.columns.label') || 'Label' },
    { key: 'address', label: $t('admin.customers.locations.columns.address') || 'Address' },
    {
      key: 'coordinates',
      label: $t('admin.customers.locations.columns.coordinates') || 'Coordinates',
    },
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
    {
      key: 'invoice_number',
      label: $t('admin.customers.billing.columns.invoice_number') || 'Invoice #',
    },
    {
      key: 'subscription',
      label: $t('admin.customers.billing.columns.subscription') || 'Subscription',
    },
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
  const canReadCustomers = $derived($can('read', 'customers') || $can('manage', 'customers'));
  const canManageCustomers = $derived($can('manage', 'customers'));
  const canReadCustomerLocations = $derived(
    $can('read', 'customer_locations') || $can('manage', 'customer_locations'),
  );
  const canManageCustomerLocations = $derived($can('manage', 'customer_locations'));
  const canReadBilling = $derived($can('read', 'billing') || $can('manage', 'billing'));
  const canReadAudit = $derived($can('read', 'audit_logs'));
  const canReadPppoe = $derived($can('read', 'pppoe') || $can('manage', 'pppoe'));
  const pppoeToolbar = $derived(getCustomerPppoeToolbarConfig());
  const visibleTabs = $derived.by(() =>
    getVisibleCustomerDetailTabs({
      canReadCustomerLocations,
      canReadBilling,
      canReadPppoe,
      canReadAudit,
    }),
  );
  const customerDetailAccess = $derived.by(() => ({
    canReadCustomerLocations,
    canReadBilling,
    canReadPppoe,
    canReadAudit,
  }));
  const timelineFilteredLogs = $derived.by(() => {
    if (timelineType === 'all') return timelineLogs;
    if (timelineType === 'customer') return timelineLogs.filter((l) => l.resource === 'customers');
    if (timelineType === 'location')
      return timelineLogs.filter((l) => l.resource === 'customer_locations');
    if (timelineType === 'subscription')
      return timelineLogs.filter((l) => l.resource === 'customer_subscriptions');
    return timelineLogs;
  });
  const subscriptionById = $derived.by(
    () => new Map(subscriptions.map((sub) => [sub.id, sub] as const)),
  );
  const timelineColumns = $derived.by(() => [
    { key: 'created_at', label: 'Waktu' },
    { key: 'action', label: 'Aksi' },
    { key: 'resource', label: 'Resource' },
    { key: 'actor', label: 'Actor' },
    { key: 'details', label: 'Detail' },
  ]);
  const timelineRows = $derived.by(() => buildCustomerTimelineRows(timelineFilteredLogs));
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
      (a, b) =>
        new Date(b.created_at || b.due_date).getTime() -
        new Date(a.created_at || a.due_date).getTime(),
    );
  });
  const billingStats = $derived.by(() => {
    const now = Date.now();
    const overdue = billingRows.filter(
      (inv) => inv.status !== 'paid' && new Date(inv.due_date).getTime() < now,
    ).length;
    const unpaid = billingRows.filter((inv) =>
      ['pending', 'verification_pending'].includes(inv.status),
    ).length;
    const paid = billingRows.filter((inv) => inv.status === 'paid').length;
    return {
      total: billingRows.length,
      unpaid,
      paid,
      overdue,
    };
  });

  onMount(async () => {
    if (!canReadCustomers) {
      goto('/unauthorized');
      return;
    }
    const fromUrl = readActiveTabFromUrl();
    if (fromUrl) activeTab = fromUrl;
    await loadCustomer();
    if (canReadCustomerLocations) {
      await loadLocations();
    }
  });

  $effect(() => {
    const fromUrl = readActiveTabFromUrl();
    if (fromUrl && fromUrl !== activeTab) {
      activeTab = fromUrl;
    }
    if (!visibleTabs.includes(activeTab)) {
      activeTab = 'overview';
    }
  });

  $effect(() => {
    const autoLoadKey = getCustomerDetailAutoLoadKey(activeTab, customerId, customerDetailAccess);
    if (!autoLoadKey) return;
    if (activeTab !== 'subscriptions') return;
    if (!canReadCustomers) return;
    untrack(() => {
      void loadSubscriptions();
      if (subscriptionPackages.length === 0) {
        void loadSubscriptionPackages();
      }
    });
  });

  $effect(() => {
    const autoLoadKey = getCustomerDetailAutoLoadKey(activeTab, customerId, customerDetailAccess);
    if (!autoLoadKey) return;
    if (activeTab !== 'billing') return;
    if (!canReadBilling) return;
    untrack(() => {
      void loadSubscriptions();
      void loadBillingInvoices();
    });
  });

  $effect(() => {
    const autoLoadKey = getCustomerDetailAutoLoadKey(activeTab, customerId, customerDetailAccess);
    if (!autoLoadKey) return;
    if (activeTab !== 'timeline') return;
    if (!canReadAudit) return;
    untrack(() => {
      void loadTimeline();
    });
  });

  $effect(() => {
    const autoLoadKey = getCustomerDetailAutoLoadKey(activeTab, customerId, customerDetailAccess);
    if (!autoLoadKey) return;
    if (activeTab !== 'pppoe') return;
    if (!canReadPppoe) return;
    untrack(() => {
      void loadPppoeAccounts();
    });
  });

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

  function applyPppoePackage(pkgId: string) {
    const resolved = getPppoeAssignmentPayload({
      packageId: pkgId,
      mappings: pppoePackageMappings,
      current: {
        router_profile_name: pppoeRouterProfileName,
        remote_address: pppoeRemoteAddress,
        address_pool: pppoeAddressPool,
      },
    });
    pppoeRouterProfileName = resolved.router_profile_name || '';
    pppoeRemoteAddress = resolved.remote_address || '';
    pppoeAddressPool = resolved.address_pool || '';
  }

  $effect(() => {
    if (!showEditPppoe) return;
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) return;

    const rid = pppoeRouterId;
    if (!rid) {
      pppoePackageMappings = [];
      return;
    }
    if (pppoePackageMappings.length === 0) void loadPppoePackages(rid);
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
      goto(customersPath);
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
    loadingLifecycleObservability = true;
    try {
      const [res, metrics] = await Promise.all([
        api.customers.subscriptions.list(customerId, { page: 1, per_page: 200 }),
        api.customers.observability.lifecycle(customerId),
      ]);
      subscriptions = res.data || [];
      lifecycleObservability = metrics;
    } catch (e: any) {
      toast.error(`Failed to load subscriptions: ${e?.message || e}`);
    } finally {
      loadingSubscriptions = false;
      loadingLifecycleObservability = false;
    }
  }

  function subscriptionStatusLabel(status: string): string {
    const map: Record<string, string> = {
      active: get(t)('common.active') || 'Active',
      grace_active: 'Aktif sementara',
      pending_installation: 'Menunggu instalasi',
      installation_done_awaiting_payment: 'Instalasi selesai, menunggu pembayaran',
      suspended: get(t)('common.suspended') || 'Suspended',
      cancelled: get(t)('common.cancelled') || 'Cancelled',
    };
    return map[status] || status;
  }

  function metricCount(stage: string, source: 'lifecycle' | 'work_order' = 'lifecycle'): number {
    const items =
      source === 'lifecycle'
        ? lifecycleObservability?.lifecycle_funnel || []
        : lifecycleObservability?.work_order_funnel || [];
    return items.find((item) => item.stage === stage)?.count || 0;
  }

  function agingBucketCount(bucket: string): number {
    return lifecycleObservability?.aging_buckets.find((item) => item.bucket === bucket)?.count || 0;
  }

  function readActiveTabFromUrl(): CustomerDetailTab | null {
    return readCustomerDetailTabFromUrlValue($page.url.searchParams.get('tab'), {
      canReadCustomerLocations,
      canReadBilling,
      canReadPppoe,
      canReadAudit,
    });
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
    if (billingLoadInFlight) return;
    billingLoadInFlight = true;
    loadingBilling = true;
    try {
      const invoices = await api.payment.listCustomerPackageInvoices();
      billingInvoices = invoices;
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.billing.toasts.load_failed', {
          values: { message: e?.message || e },
        }) || `Failed to load billing invoices: ${e?.message || e}`,
      );
    } finally {
      loadingBilling = false;
      billingLoadInFlight = false;
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
        get(t)('admin.customers.billing.toasts.generate_failed', {
          values: { message: e?.message || e },
        }) || `Failed to generate invoice: ${e?.message || e}`,
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
          .catch(() => ({ data: [] as CustomerSubscriptionView[] }) as any),
      ]);

      const allowedLocationIds = new Set((locRows || []).map((l) => l.id));
      const allowedSubscriptionIds = new Set(
        ((subRes?.data as CustomerSubscriptionView[]) || []).map((s) => s.id),
      );

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
    subBillingCycle = (row.billing_cycle === 'yearly' ? 'yearly' : 'monthly') as
      | 'monthly'
      | 'yearly';
    subPrice = String(row.price ?? '');
    subCurrency = row.currency_code || '';
    subStatus = (
      ['active', 'suspended', 'cancelled'].includes(row.status) ? row.status : 'active'
    ) as 'active' | 'suspended' | 'cancelled';
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
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) return;
    loadingPppoeRouters = true;
    try {
      pppoeRouters = await api.mikrotik.routers.list();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.routers_failed') || 'Failed to load routers',
      );
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
        get(t)('admin.customers.pppoe.toasts.load_failed', {
          values: { message: e?.message || e },
        }) || `Failed to load PPPoE accounts: ${e?.message || e}`,
      );
    } finally {
      loadingPppoe = false;
    }
  }

  function resetPppoeForm() {
    pppoeRouterId = '';
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

  function openEditPppoe(row: PppoeAccountPublic) {
    editingPppoe = row;
    pppoeRouterId = row.router_id;
    pppoeUsername = row.username;
    pppoePassword = '';
    pppoePackageId = row.package_id || '';
    pppoeRouterProfileName = row.router_profile_name || '';
    pppoeRemoteAddress = row.remote_address || '';
    pppoeAddressPool = row.address_pool || '';
    pppoeDisabled = !!row.disabled;
    pppoeComment = row.comment || '';
    showEditPppoe = true;
    if (pppoeRouters.length === 0) {
      void loadPppoeRouters();
    }
  }

  async function submitUpdatePppoe() {
    if (!editingPppoe) return;
    if (pppoePackageSelectionHasMissingMapping) {
      toast.error(
        get(t)('admin.network.pppoe.form.package_mapping_missing') ||
          'This package does not have a router mapping yet. Existing account values will be kept until a mapping is added.',
      );
      return;
    }
    savingPppoe = true;
    try {
      const assignmentPayload = getPppoeAssignmentPayload({
        packageId: pppoePackageId,
        mappings: pppoePackageMappings,
        current: {
          router_profile_name: pppoeRouterProfileName,
          remote_address: pppoeRemoteAddress,
          address_pool: pppoeAddressPool,
        },
      });
      await api.pppoe.accounts.update(editingPppoe.id, {
        username: pppoeUsername.trim() || undefined,
        password: pppoePassword || undefined,
        package_id: pppoePackageId || null,
        router_profile_name: assignmentPayload.router_profile_name,
        remote_address: assignmentPayload.remote_address,
        address_pool: assignmentPayload.address_pool,
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
        get(t)('admin.customers.pppoe.toasts.update_failed', {
          values: { message: e?.message || e },
        }) || `Failed: ${e?.message || e}`,
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
        get(t)('admin.customers.pppoe.toasts.apply_failed', {
          values: { message: e?.message || e },
        }) || `Failed: ${e?.message || e}`,
      );
    }
  }

  async function deletePppoe(row: PppoeAccountPublic) {
    if (!confirm(get(t)('admin.customers.pppoe.confirm_delete') || 'Delete this PPPoE account?'))
      return;
    try {
      await api.pppoe.accounts.delete(row.id);
      toast.success(get(t)('admin.customers.pppoe.toasts.deleted') || 'Deleted');
      await loadPppoeAccounts();
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.delete_failed', {
          values: { message: e?.message || e },
        }) || `Failed: ${e?.message || e}`,
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
    const parsedCoordinates = validateOptionalCoordinates(locLatitude, locLongitude);
    if (parsedCoordinates.error) {
      if (parsedCoordinates.error === 'both_required') {
        toast.error('Latitude dan longitude harus diisi berpasangan');
      } else if (parsedCoordinates.error === 'invalid_number') {
        toast.error('Koordinat lokasi tidak valid');
      } else if (parsedCoordinates.error === 'latitude_range') {
        toast.error('Latitude harus di antara -90 hingga 90');
      } else if (parsedCoordinates.error === 'longitude_range') {
        toast.error('Longitude harus di antara -180 hingga 180');
      }
      return;
    }
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
        latitude: parsedCoordinates.latitude,
        longitude: parsedCoordinates.longitude,
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
      locLatitude = '';
      locLongitude = '';
      locNotes = '';
      await loadLocations();
      toast.success(get(t)('admin.customers.locations.toasts.created') || 'Location added');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.locations.toasts.create_failed', {
          values: { message: e?.message || e },
        }) || `Failed: ${e?.message || e}`,
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
    locLatitude = row?.latitude != null ? String(row.latitude) : '';
    locLongitude = row?.longitude != null ? String(row.longitude) : '';
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
    const parsedCoordinates = validateOptionalCoordinates(locLatitude, locLongitude);
    if (parsedCoordinates.error) {
      if (parsedCoordinates.error === 'both_required') {
        toast.error('Latitude dan longitude harus diisi berpasangan');
      } else if (parsedCoordinates.error === 'invalid_number') {
        toast.error('Koordinat lokasi tidak valid');
      } else if (parsedCoordinates.error === 'latitude_range') {
        toast.error('Latitude harus di antara -90 hingga 90');
      } else if (parsedCoordinates.error === 'longitude_range') {
        toast.error('Longitude harus di antara -180 hingga 180');
      }
      return;
    }
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
        latitude: parsedCoordinates.latitude,
        longitude: parsedCoordinates.longitude,
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
      goto(customersPath);
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
      <button class="btn btn-secondary" onclick={() => goto(customersPath)}>
        <Icon name="arrow-left" size={16} />
        {$t('common.back') || 'Back'}
      </button>
      <div class="header-actions">
        {#if canManageCustomers && customer}
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
        <button class="btn btn-secondary" onclick={refreshCurrent}>
          <Icon name="refresh-cw" size={16} />
          {$t('common.refresh') || 'Refresh'}
        </button>
        {#if canManageCustomers}
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
          {customer?.email ||
            customer?.phone ||
            $t('admin.customers.detail.subtitle') ||
            'Customer details'}
        </p>
        <div class="hero-badges">
          <span class={`status-pill ${customer?.is_active ? 'is-active' : 'is-inactive'}`}>
            <span class="dot"></span>
            {customer?.is_active
              ? $t('common.active') || 'Active'
              : $t('common.inactive') || 'Inactive'}
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
    {#if visibleTabs.includes('locations')}
      <button class:active={activeTab === 'locations'} onclick={() => (activeTab = 'locations')}>
        {$t('admin.customers.tabs.locations') || 'Locations'}
      </button>
    {/if}
    {#if visibleTabs.includes('subscriptions')}
      <button
        class:active={activeTab === 'subscriptions'}
        onclick={() => (activeTab = 'subscriptions')}
      >
        {$t('admin.customers.tabs.subscriptions') || 'Subscriptions'}
      </button>
    {/if}
    {#if visibleTabs.includes('billing')}
      <button class:active={activeTab === 'billing'} onclick={() => (activeTab = 'billing')}>
        {$t('admin.customers.tabs.billing') || 'Billing'}
      </button>
    {/if}
    {#if visibleTabs.includes('pppoe')}
      <button class:active={activeTab === 'pppoe'} onclick={() => (activeTab = 'pppoe')}>
        {$t('admin.customers.tabs.pppoe') || 'PPPoE'}
      </button>
    {/if}
    {#if visibleTabs.includes('timeline')}
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
          {#if canManageCustomers}
            <button
              class="btn btn-primary"
              onclick={saveOverview}
              disabled={saving || !name.trim()}
            >
              <Icon name="check-circle" size={16} />
              {$t('common.save') || 'Save'}
            </button>
          {/if}
        </div>

        <div class="overview-grid">
          <div class="form">
            <label>
              <span>{$t('admin.customers.fields.name') || 'Name'}</span>
              <input class="input" bind:value={name} disabled={!canManageCustomers} />
            </label>
            <div class="grid2">
              <label>
                <span>{$t('admin.customers.fields.email') || 'Email'}</span>
                <input class="input" bind:value={email} disabled={!canManageCustomers} />
              </label>
              <label>
                <span>{$t('admin.customers.fields.phone') || 'Phone'}</span>
                <input class="input" bind:value={phone} disabled={!canManageCustomers} />
              </label>
            </div>
            <label>
              <span>{$t('admin.customers.fields.notes') || 'Notes'}</span>
              <textarea class="input" rows="5" bind:value={notes} disabled={!canManageCustomers}
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
    {:else if activeTab === 'locations' && canReadCustomerLocations}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.locations.title') || 'Locations'}</h3>
            <p class="subtitle">
              {$t('admin.customers.locations.subtitle') || 'Service locations under this customer.'}
            </p>
          </div>
          {#if canManageCustomerLocations}
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
                {[loc.city, loc.state, loc.postal_code, loc.country].filter(Boolean).join(', ') ||
                  '-'}
              </div>
            {:else if key === 'coordinates'}
              <div class="mono">{formatLocationCoordinates(loc.latitude, loc.longitude) || '-'}</div>
            {:else if key === 'updated_at'}
              <span class="mono">{new Date(loc.updated_at).toLocaleString()}</span>
            {:else if key === 'actions'}
              <div class="row-actions">
                <button
                  class="btn-icon"
                  title={$t('common.refresh') || 'Refresh'}
                  onclick={loadLocations}
                >
                  <Icon name="refresh-cw" size={16} />
                </button>
                {#if canManageCustomerLocations}
                  <button
                    class="btn-icon"
                    title={$t('common.edit') || 'Edit'}
                    onclick={() => openEditLocation(loc)}
                  >
                    <Icon name="edit-3" size={16} />
                  </button>
                  <button
                    class="btn-icon danger"
                    title={$t('common.delete') || 'Delete'}
                    onclick={() => confirmDeleteLocation(loc)}
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
    {:else if activeTab === 'subscriptions'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.subscriptions.title') || 'Subscriptions'}</h3>
            <p class="subtitle">
              {$t('admin.customers.subscriptions.subtitle') ||
                'Customer service subscriptions for billing and service assignment.'}
            </p>
          </div>
          <div class="header-actions">
            <button
              class="btn btn-secondary"
              onclick={loadSubscriptions}
              disabled={loadingSubscriptions}
            >
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

        <div class="lifecycle-observability card">
          <div class="observability-head">
            <div>
              <h4>Lifecycle observability</h4>
              <p class="subtitle">
                Operational funnel and aging snapshot for this customer's activations.
              </p>
            </div>
            <span class="meta-pill">
              <Icon name="activity" size={14} />
              {#if loadingLifecycleObservability}
                Loading...
              {:else if lifecycleObservability?.generated_at}
                {`Updated ${timeAgo(lifecycleObservability.generated_at)}`}
              {:else}
                Waiting for data
              {/if}
            </span>
          </div>

          <div class="observability-grid">
            <div class="metric-tile">
              <span class="metric-label">Pending installation</span>
              <strong>{metricCount('pending_installation')}</strong>
            </div>
            <div class="metric-tile emphasis">
              <span class="metric-label">Grace active</span>
              <strong
                >{metricCount('grace_active') ||
                  metricCount('installation_done_awaiting_payment')}</strong
              >
            </div>
            <div class="metric-tile">
              <span class="metric-label">Active</span>
              <strong>{metricCount('active')}</strong>
            </div>
            <div class="metric-tile">
              <span class="metric-label">Cancelled</span>
              <strong>{metricCount('cancelled')}</strong>
            </div>
            <div class="metric-tile">
              <span class="metric-label">WO pending</span>
              <strong>{metricCount('pending', 'work_order')}</strong>
            </div>
            <div class="metric-tile">
              <span class="metric-label">WO in progress</span>
              <strong>{metricCount('in_progress', 'work_order')}</strong>
            </div>
            <div class="metric-tile">
              <span class="metric-label">WO completed</span>
              <strong>{metricCount('completed', 'work_order')}</strong>
            </div>
          </div>

          <div class="aging-row">
            <span class="aging-pill">0-1d: {agingBucketCount('0-1d')}</span>
            <span class="aging-pill">2-3d: {agingBucketCount('2-3d')}</span>
            <span class="aging-pill">4-7d: {agingBucketCount('4-7d')}</span>
            <span class="aging-pill">>7d: {agingBucketCount('>7d')}</span>
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
              <div class="sub">{subscriptionStatusLabel(row.status)}</div>
            {:else if key === 'billing'}
              <div class="name">{row.billing_cycle}</div>
              <div class="sub mono">
                {row.currency_code}
                {Number(row.price || 0).toLocaleString()}
              </div>
            {:else if key === 'location'}
              <div>{row.location_label || '-'}</div>
            {:else if key === 'router'}
              <div>{row.router_name || '-'}</div>
            {:else if key === 'period'}
              <div class="sub">
                {row.starts_at ? new Date(row.starts_at).toLocaleDateString() : '-'}
              </div>
              <div class="sub">
                {row.ends_at ? new Date(row.ends_at).toLocaleDateString() : '-'}
              </div>
            {:else if key === 'actions'}
              <div class="row-actions">
                {#if $can('manage', 'customers')}
                  <button
                    class="btn-icon"
                    title={$t('admin.customers.billing.actions.generate_from_subscription') ||
                      'Generate invoice'}
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
                  <button
                    class="btn-icon"
                    title={$t('common.edit') || 'Edit'}
                    onclick={() => openEditSubscription(row)}
                  >
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
                <option value="pending"
                  >{$t('admin.package_invoices.statuses.pending') || 'Pending'}</option
                >
                <option value="verification_pending">
                  {$t('admin.package_invoices.statuses.verification_pending') ||
                    'Verification pending'}
                </option>
                <option value="paid">{$t('admin.package_invoices.statuses.paid') || 'Paid'}</option>
                <option value="failed"
                  >{$t('admin.package_invoices.statuses.failed') || 'Failed'}</option
                >
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
              <input
                class="input"
                type="date"
                bind:value={billingDateFrom}
                oninput={onBillingDateChange}
              />
            </label>
            <label class="inline-filter">
              <span>{$t('admin.customers.billing.filters.to') || 'To'}</span>
              <input
                class="input"
                type="date"
                bind:value={billingDateTo}
                oninput={onBillingDateChange}
              />
            </label>
            <button
              class="btn btn-secondary"
              onclick={clearBillingFilters}
              disabled={billingStatus === 'all' && !billingDateFrom && !billingDateTo}
            >
              <Icon name="eraser" size={16} />
              {$t('admin.customers.billing.filters.clear') || 'Clear'}
            </button>
            <button
              class="btn btn-secondary"
              onclick={loadBillingInvoices}
              disabled={loadingBilling}
            >
              <Icon name="refresh-cw" size={16} />
              {$t('common.refresh') || 'Refresh'}
            </button>
          </div>
        </div>

        <div class="billing-stats">
          <div class="billing-stat">
            <div class="billing-stat-label">
              {$t('admin.customers.billing.stats.total') || 'Total invoices'}
            </div>
            <div class="billing-stat-value">{billingStats.total}</div>
          </div>
          <div class="billing-stat">
            <div class="billing-stat-label">
              {$t('admin.customers.billing.stats.unpaid') || 'Unpaid'}
            </div>
            <div class="billing-stat-value">{billingStats.unpaid}</div>
          </div>
          <div class="billing-stat">
            <div class="billing-stat-label">
              {$t('admin.customers.billing.stats.paid') || 'Paid'}
            </div>
            <div class="billing-stat-value">{billingStats.paid}</div>
          </div>
          <div class="billing-stat">
            <div class="billing-stat-label">
              {$t('admin.customers.billing.stats.overdue') || 'Overdue'}
            </div>
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
              <div class="sub mono">
                {row.created_at ? new Date(row.created_at).toLocaleString() : '-'}
              </div>
            {:else if key === 'subscription'}
              <div class="name">
                {subscription?.package_name || subscription?.package_id || '-'}
              </div>
              <div class="sub">{subscription?.billing_cycle || '-'}</div>
            {:else if key === 'amount'}
              <div class="name">
                {formatMoney(row.amount, { currency: row.currency_code || undefined })}
              </div>
            {:else if key === 'status'}
              <span
                class={`badge ${row.status === 'paid' ? 'ok' : row.status === 'failed' ? 'danger' : 'warn'}`}
              >
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
          <div class="pppoe-toolbar">
            {#if pppoeToolbar.showSearch}
              <label class="pppoe-search" for="customer-pppoe-search">
                <Icon name="search" size={16} />
                <span class="sr-only">{$t('common.search') || 'Search'}</span>
                <input
                  id="customer-pppoe-search"
                  class="pppoe-search-input"
                  bind:value={pppoeQuery}
                  placeholder={$t('admin.customers.pppoe.search') || 'Search username...'}
                  oninput={() => void loadPppoeAccounts()}
                />
              </label>
            {/if}
            {#if pppoeToolbar.showRefresh}
              <button class="btn btn-secondary" onclick={loadPppoeAccounts} disabled={loadingPppoe}>
                <Icon name="refresh-cw" size={16} />
                {$t('common.refresh') || 'Refresh'}
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
            {@const syncMeta = getPppoeSyncDisplay(row)}
            {#if key === 'username'}
              <div class="name">{row.username}</div>
              <div class="sub mono">
                {row.disabled
                  ? $t('common.disabled') || 'Disabled'
                  : $t('common.active') || 'Active'}
              </div>
              <div class="sub mono">
                {getPppoeProvisioningTargetFallback(row.account_source)}
              </div>
            {:else if key === 'router'}
              <div class="name">{routerName}</div>
              <div class="sub mono">{row.router_id}</div>
            {:else if key === 'location'}
              <div class="name">{locName}</div>
              <div class="sub mono">{row.location_id}</div>
            {:else if key === 'assignment'}
              <div class="sub">
                <span class="pill"
                  >{$t('admin.customers.pppoe.fields.profile') || 'Profile'}: {row.router_profile_name ||
                    '-'}</span
                >
                <span class="pill"
                  >{$t('admin.customers.pppoe.fields.remote_address') || 'Remote'}: {row.remote_address ||
                    row.address_pool ||
                    '-'}</span
                >
              </div>
            {:else if key === 'sync'}
              <div class="sub">
                <span class={`badge ${syncMeta.tone === 'ok' ? 'ok' : 'warn'}`}>
                  {syncMeta.label}
                </span>
                <span class="mono">{syncMeta.syncedAt ? timeAgo(syncMeta.syncedAt) : '-'}</span>
              </div>
              {#if syncMeta.error}
                <div class="sub error">{syncMeta.error}</div>
              {/if}
              {#if row.account_source === 'managed_radius' && row.radius_identity}
                <div class="sub mono">Identity: {row.radius_identity}</div>
              {/if}
            {:else if key === 'actions'}
              <div class="row-actions">
                {#if $can('manage', 'pppoe')}
                  <button
                    class="btn-icon"
                    title={$t('admin.customers.pppoe.actions.apply') ||
                      getPppoeApplyActionFallback(row.account_source)}
                    onclick={() => applyPppoe(row)}
                  >
                    <Icon name="send" size={16} />
                  </button>
                  <button
                    class="btn-icon"
                    title={$t('common.edit') || 'Edit'}
                    onclick={() => openEditPppoe(row)}
                  >
                    <Icon name="edit" size={16} />
                  </button>
                  <button
                    class="btn-icon danger"
                    title={$t('common.delete') || 'Delete'}
                    onclick={() => deletePppoe(row)}
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
          <button class:active={timelineType === 'all'} onclick={() => (timelineType = 'all')}
            >All</button
          >
          <button
            class:active={timelineType === 'customer'}
            onclick={() => (timelineType = 'customer')}>Profile</button
          >
          <button
            class:active={timelineType === 'location'}
            onclick={() => (timelineType = 'location')}>Location</button
          >
          <button
            class:active={timelineType === 'subscription'}
            onclick={() => (timelineType = 'subscription')}>Subscription</button
          >
        </div>
        <Table
          columns={timelineColumns}
          data={timelineRows}
          loading={loadingTimeline}
          emptyText="No timeline yet."
          pagination
          searchable
          searchPlaceholder="Search timeline..."
          mobileView="scroll"
        >
          {#snippet cell({ item, key })}
            {#if key === 'created_at'}
              <div class="timeline-table-time">
                <div>{new Date(item.created_at).toLocaleString()}</div>
                <div class="sub">{timeAgo(item.created_at)}</div>
              </div>
            {:else if key === 'action'}
              <div class="timeline-table-action">{item.action}</div>
            {:else if key === 'resource'}
              <span class="pill">{item.resource}</span>
            {:else if key === 'actor'}
              <div class="timeline-table-actor">{item.actor}</div>
            {:else if key === 'details'}
              <div class:subtle-empty={!item.details}>
                {item.details || 'No detail'}
              </div>
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>
    {/if}
  {/if}
</div>

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
          'Choose a package to control PPP profile and addressing for the selected router.'}
      </div>
    </label>

    {#if pppoePackageSelectionHasMissingMapping}
      <div class="field-hint warning">
        {$t('admin.network.pppoe.form.package_mapping_missing') ||
          'This package does not have a router mapping yet. Existing account values will be kept until a mapping is added.'}
      </div>
    {/if}

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
        <input class="input" bind:value={pppoeUsername} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input
          class="input"
          type="password"
          bind:value={pppoePassword}
          placeholder={$t('admin.customers.pppoe.edit.password_hint') || 'Leave blank to keep'}
        />
      </label>
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
      <Toggle
        bind:checked={pppoeDisabled}
        ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}
      />
    </div>

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showEditPppoe = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={submitUpdatePppoe}
        disabled={savingPppoe || pppoePackageSelectionHasMissingMapping || !pppoeUsername.trim()}
      >
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
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.latitude') || 'Latitude'}</span>
        <input class="input mono" bind:value={locLatitude} placeholder="-7.275233" />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.longitude') || 'Longitude'}</span>
        <input class="input mono" bind:value={locLongitude} placeholder="110.355211" />
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
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.latitude') || 'Latitude'}</span>
        <input class="input mono" bind:value={locLatitude} placeholder="-7.275233" />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.longitude') || 'Longitude'}</span>
        <input class="input mono" bind:value={locLongitude} placeholder="110.355211" />
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
  message={$t('admin.customers.delete.message') ||
    'This will remove the customer and all related data.'}
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
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      transform 0.02s ease;
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

  .pppoe-toolbar {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 0.75rem;
    width: min(100%, 28rem);
  }

  .pppoe-search {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: min(100%, 18rem);
    flex: 1 1 18rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 12%);
    border-radius: 14px;
    padding: 0.72rem 0.9rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    color: var(--text-secondary);
    transition:
      border-color 140ms ease,
      box-shadow 140ms ease,
      background 140ms ease;
  }

  .pppoe-search:focus-within {
    border-color: rgba(99, 102, 241, 0.55);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.12);
    background: color-mix(in srgb, var(--bg-surface), transparent 1%);
  }

  .pppoe-search-input {
    width: 100%;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    padding: 0;
  }

  .pppoe-search-input::placeholder {
    color: var(--text-secondary);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .lifecycle-observability {
    margin-bottom: 1rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
    border-radius: 16px;
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .observability-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.9rem;
  }

  .observability-head h4 {
    margin: 0;
    font-size: 1rem;
  }

  .observability-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.75rem;
  }

  .metric-tile {
    border-radius: 14px;
    padding: 0.85rem 0.9rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 5%);
  }

  .metric-tile.emphasis {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.08);
  }

  .metric-label {
    display: block;
    font-size: 0.78rem;
    color: var(--text-secondary);
    margin-bottom: 0.35rem;
  }

  .metric-tile strong {
    font-size: 1.4rem;
    line-height: 1;
  }

  .aging-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.9rem;
  }

  .aging-pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.34rem 0.7rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
    font-size: 0.8rem;
    color: var(--text-secondary);
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
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
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

  .timeline-table-time,
  .timeline-table-action,
  .timeline-table-actor {
    display: grid;
    gap: 0.2rem;
  }

  .timeline-table-action,
  .timeline-table-actor {
    font-weight: 560;
  }

  .subtle-empty {
    color: var(--text-secondary);
    font-style: italic;
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
    .section-head {
      flex-direction: column;
      align-items: stretch;
    }
    .pppoe-toolbar {
      width: 100%;
      justify-content: stretch;
    }
    .pppoe-search {
      min-width: 0;
      width: 100%;
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
