<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { Component } from 'svelte';
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
    type MessageTemplate,
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
  import { timeAgo } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import { getAdminCustomerNavigation } from '$lib/utils/adminCustomerNavigation';
  import {
    formatLocationCoordinates,
    validateOptionalCoordinates,
  } from '$lib/utils/customerLocationCoordinates';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { loadCustomerDetailDialogsOverlay } from './customerDetailModules';
  import { createCustomerDetailResourceLoader } from './customerDetailResourceLoader';
  import {
    loadCustomerPppoeHelperModule,
    loadCustomerTimelineHelperModule,
    type CustomerPppoeHelperModule,
    type CustomerTimelineHelperModule,
  } from './customerDetailDeferredModules';
  import {
    loadCustomerBillingTab,
    loadCustomerPppoeTab,
    loadCustomerSubscriptionsTab,
    loadCustomerTimelineTab,
  } from './customerDetailTabModules';

  const customerId = $derived(String($page.params.id || ''));
  type DeferredComponent = Component<any>;
  const customerNav = $derived.by(() =>
    getAdminCustomerNavigation({
      hostname: $page.url.hostname,
      tenantSlug: $page.data?.tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const customersPath = $derived(customerNav.customersPath);

  let activeTab = $state<CustomerDetailTab>('overview');
  let CustomerDetailDialogsComponent = $state<DeferredComponent | null>(null);
  let SubscriptionsTabComponent = $state<DeferredComponent | null>(null);
  let BillingTabComponent = $state<DeferredComponent | null>(null);
  let PppoeTabComponent = $state<DeferredComponent | null>(null);
  let TimelineTabComponent = $state<DeferredComponent | null>(null);
  let pppoeHelperModule = $state<CustomerPppoeHelperModule | null>(null);
  let timelineHelperModule = $state<CustomerTimelineHelperModule | null>(null);
  let activeDeferredTabLoading = $state<CustomerDetailTab | null>(null);

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
  let whatsappGatewayReady = $state(false);
  let whatsappGatewayReason = $state('WhatsApp gateway is not configured');
  let whatsappGatewayProvider = $state('');
  let whatsappSending = $state(false);
  let showWhatsAppCompose = $state(false);
  let whatsappTemplateOptions = $state<MessageTemplate[]>([]);
  let selectedWhatsappTemplateId = $state('custom');
  let whatsappMessage = $state('');
  let emailSending = $state(false);
  let showEmailCompose = $state(false);
  let emailTemplateOptions = $state<MessageTemplate[]>([]);
  let selectedEmailTemplateId = $state('custom');
  let emailSubject = $state('');
  let emailBody = $state('');

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

  const locationsResourceLoader = createCustomerDetailResourceLoader<CustomerLocation[]>();
  const subscriptionsResourceLoader = createCustomerDetailResourceLoader<{
    rows: CustomerSubscriptionView[];
    lifecycle: CustomerLifecycleObservability | null;
  }>();
  const billingResourceLoader = createCustomerDetailResourceLoader<Invoice[]>();
  const pppoeResourceLoader = createCustomerDetailResourceLoader<PppoeAccountPublic[]>();
  const timelineResourceLoader = createCustomerDetailResourceLoader<AuditLog[]>();

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
  const pppoeToolbar = $derived(
    pppoeHelperModule?.pppoeToolbar ?? {
      showSearch: true,
      showRefresh: true,
      showCreate: false,
      showReconcile: false,
    },
  );
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
  const timelineRows = $derived.by(() =>
    timelineHelperModule ? timelineHelperModule.buildCustomerTimelineRows(timelineFilteredLogs) : [],
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
    await Promise.all([
      loadCustomer(),
      canManageCustomers ? loadCommunicationReadiness() : Promise.resolve(),
      canManageCustomers ? loadCommunicationTemplates() : Promise.resolve(),
    ]);
    if (canReadCustomerLocations) {
      await loadLocations({ force: true });
    }
  });

  async function ensureCustomerDetailDialogsLoaded() {
    if (CustomerDetailDialogsComponent) return;

    const modules = await loadCustomerDetailDialogsOverlay();
    CustomerDetailDialogsComponent = modules.CustomerDetailDialogsComponent;
  }

  async function ensureCustomerDeferredTabComponent(tab: CustomerDetailTab) {
    if (tab === 'subscriptions') {
      if (SubscriptionsTabComponent) return;
      activeDeferredTabLoading = tab;
      const module = await loadCustomerSubscriptionsTab();
      SubscriptionsTabComponent = module.default;
      activeDeferredTabLoading = null;
      return;
    }
    if (tab === 'billing') {
      if (BillingTabComponent) return;
      activeDeferredTabLoading = tab;
      const module = await loadCustomerBillingTab();
      BillingTabComponent = module.default;
      activeDeferredTabLoading = null;
      return;
    }
    if (tab === 'pppoe') {
      if (PppoeTabComponent) return;
      activeDeferredTabLoading = tab;
      const module = await loadCustomerPppoeTab();
      PppoeTabComponent = module.default;
      activeDeferredTabLoading = null;
      return;
    }
    if (tab === 'timeline') {
      if (TimelineTabComponent) return;
      activeDeferredTabLoading = tab;
      const module = await loadCustomerTimelineTab();
      TimelineTabComponent = module.default;
      activeDeferredTabLoading = null;
    }
  }

  async function ensureCustomerPppoeHelper() {
    if (pppoeHelperModule) return;
    pppoeHelperModule = await loadCustomerPppoeHelperModule();
  }

  async function ensureCustomerTimelineHelper() {
    if (timelineHelperModule) return;
    timelineHelperModule = await loadCustomerTimelineHelperModule();
  }

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
    if (
      activeTab === 'subscriptions' ||
      activeTab === 'billing' ||
      activeTab === 'pppoe' ||
      activeTab === 'timeline'
    ) {
      untrack(() => {
        void ensureCustomerDeferredTabComponent(activeTab);
      });
    }
  });

  $effect(() => {
    const autoLoadKey = getCustomerDetailAutoLoadKey(activeTab, customerId, customerDetailAccess);
    if (!autoLoadKey) return;
    if (activeTab !== 'pppoe') return;
    untrack(() => {
      void ensureCustomerPppoeHelper();
    });
  });

  $effect(() => {
    const autoLoadKey = getCustomerDetailAutoLoadKey(activeTab, customerId, customerDetailAccess);
    if (!autoLoadKey) return;
    if (activeTab !== 'timeline') return;
    untrack(() => {
      void ensureCustomerTimelineHelper();
    });
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

  async function loadCommunicationReadiness() {
    try {
      const readiness = await api.whatsapp.readiness();
      whatsappGatewayReady = readiness.ready;
      whatsappGatewayReason = readiness.reason || '';
      whatsappGatewayProvider = readiness.provider || '';
    } catch (e: any) {
      whatsappGatewayReady = false;
      whatsappGatewayReason = e?.message || 'Failed to check WhatsApp gateway';
      whatsappGatewayProvider = '';
    }
  }

  async function loadCommunicationTemplates() {
    try {
      const [wa, email] = await Promise.all([
        api.messageTemplates.list({
          channel: 'whatsapp',
          status: 'active',
          target: 'customer',
          triggerMode: 'manual',
        }),
        api.messageTemplates.list({
          channel: 'email',
          status: 'active',
          target: 'customer',
          triggerMode: 'manual',
        }),
      ]);
      whatsappTemplateOptions = wa;
      emailTemplateOptions = email;
    } catch (e: any) {
      whatsappTemplateOptions = [];
      emailTemplateOptions = [];
      toast.error(e?.message || 'Failed to load message templates');
    }
  }

  function currentTenantName() {
    if ($page.data?.tenant?.name) return $page.data.tenant.name;
    if (typeof localStorage === 'undefined') return '';
    try {
      return JSON.parse(localStorage.getItem('auth_tenant') || '{}')?.name || '';
    } catch {
      return '';
    }
  }

  function renderCustomerTemplate(body: string) {
    if (!customer) return body;
    const values: Record<string, string> = {
      'tenant.name': currentTenantName(),
      'customer.id': customer.id,
      'customer.name': customer.name,
      'customer.email': customer.email || '',
      'customer.phone': customer.phone || '',
      'customer.status': customer.is_active ? 'active' : 'inactive',
      'customer.notes': customer.notes || '',
    };
    return body.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (_match, key) => values[key] ?? '');
  }

  function whatsappActionTitle() {
    if (!customer?.phone) return $t('admin.customers.communication.phone_not_set');
    if (!whatsappGatewayReady)
      return whatsappGatewayReason || $t('admin.customers.communication.gateway_not_ready');
    return $t('admin.customers.communication.actions.send_whatsapp');
  }

  function applyWhatsAppTemplate(templateId = selectedWhatsappTemplateId) {
    selectedWhatsappTemplateId = templateId;
    const template = whatsappTemplateOptions.find((item) => item.id === templateId);
    whatsappMessage = template?.whatsapp_body
      ? renderCustomerTemplate(template.whatsapp_body)
      : ($t('admin.customers.communication.fallback_whatsapp') || '').replace(
          '{name}',
          customer?.name || '',
        );
  }

  function openWhatsAppCompose() {
    if (!customer?.phone) {
      toast.error($t('admin.customers.communication.phone_not_set'));
      return;
    }
    if (!whatsappGatewayReady) {
      toast.error(whatsappGatewayReason || $t('admin.customers.communication.gateway_not_ready'));
      return;
    }
    showWhatsAppCompose = true;
    applyWhatsAppTemplate(whatsappTemplateOptions[0]?.id || 'custom');
  }

  async function sendCustomerWhatsApp() {
    if (!customer || whatsappSending) return;
    const message = whatsappMessage.trim();
    if (!message) {
      toast.error($t('admin.customers.communication.whatsapp_body_required'));
      return;
    }
    whatsappSending = true;
    try {
      const result = await api.whatsapp.sendCustomer({
        customerId: customer.id,
        message,
        template: selectedWhatsappTemplateId,
        templateId: selectedWhatsappTemplateId === 'custom' ? null : selectedWhatsappTemplateId,
      });
      if (!result.ok) {
        toast.error(result.error || $t('admin.customers.communication.whatsapp_failed'));
        return;
      }
      toast.success($t('admin.customers.communication.whatsapp_sent'));
      showWhatsAppCompose = false;
      whatsappMessage = '';
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.communication.whatsapp_send_failed'));
    } finally {
      whatsappSending = false;
      await loadCommunicationReadiness();
    }
  }

  function applyEmailTemplate(templateId = selectedEmailTemplateId) {
    selectedEmailTemplateId = templateId;
    const template = emailTemplateOptions.find((item) => item.id === templateId);
    emailSubject = template?.email_subject
      ? renderCustomerTemplate(template.email_subject)
      : ($t('admin.customers.communication.fallback_email_subject') || '').replace(
          '{name}',
          customer?.name || '',
        );
    emailBody = template?.email_body
      ? renderCustomerTemplate(template.email_body)
      : ($t('admin.customers.communication.fallback_email_body') || '').replace(
          '{name}',
          customer?.name || '',
        );
  }

  function openEmailCompose() {
    if (!customer?.email) {
      toast.error($t('admin.customers.communication.email_not_set'));
      return;
    }
    showEmailCompose = true;
    applyEmailTemplate(emailTemplateOptions[0]?.id || 'custom');
  }

  async function sendCustomerEmail() {
    if (!customer || emailSending) return;
    const subject = emailSubject.trim();
    const body = emailBody.trim();
    if (!subject) {
      toast.error($t('admin.customers.communication.email_subject_required'));
      return;
    }
    if (!body) {
      toast.error($t('admin.customers.communication.email_body_required'));
      return;
    }
    emailSending = true;
    try {
      await api.customerCommunication.sendEmail({
        customerId: customer.id,
        templateId: selectedEmailTemplateId === 'custom' ? null : selectedEmailTemplateId,
        subject,
        body,
      });
      toast.success($t('admin.customers.communication.email_queued'));
      showEmailCompose = false;
      emailSubject = '';
      emailBody = '';
    } catch (e: any) {
      toast.error(e?.message || $t('admin.customers.communication.email_send_failed'));
    } finally {
      emailSending = false;
    }
  }

  function getCustomerResourceKey(): string {
    return customerId;
  }

  function getPppoeResourceKey(): string {
    return `${customerId}:${pppoeQuery.trim()}`;
  }

  async function loadLocations(options: { force?: boolean } = {}) {
    if (!$can('read', 'customer_locations') && !$can('manage', 'customer_locations')) return;
    const key = getCustomerResourceKey();
    if (!options.force && locationsResourceLoader.hasLoaded(key)) return;
    loadingLocations = true;
    try {
      const result = await locationsResourceLoader.load(
        key,
        () => api.customers.locations.list(customerId),
        options,
      );
      if (result.status === 'loaded') {
        locations = result.value;
      }
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

  async function loadSubscriptions(options: { force?: boolean } = {}) {
    const key = getCustomerResourceKey();
    if (!options.force && subscriptionsResourceLoader.hasLoaded(key)) return;
    loadingSubscriptions = true;
    loadingLifecycleObservability = true;
    try {
      const result = await subscriptionsResourceLoader.load(
        key,
        async () => {
          const [res, metrics] = await Promise.all([
            api.customers.subscriptions.list(customerId, { page: 1, per_page: 200 }),
            api.customers.observability.lifecycle(customerId),
          ]);

          return {
            rows: res.data || [],
            lifecycle: metrics,
          };
        },
        options,
      );
      if (result.status === 'loaded') {
        subscriptions = result.value.rows;
        lifecycleObservability = result.value.lifecycle;
      }
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

  async function loadBillingInvoices(options: { force?: boolean } = {}) {
    const key = getCustomerResourceKey();
    if (!options.force && billingResourceLoader.hasLoaded(key)) return;
    loadingBilling = true;
    try {
      const result = await billingResourceLoader.load(
        key,
        () => api.payment.listCustomerPackageInvoices(),
        options,
      );
      if (result.status === 'loaded') {
        billingInvoices = result.value;
      }
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.billing.toasts.load_failed', {
          values: { message: e?.message || e },
        }) || `Failed to load billing invoices: ${e?.message || e}`,
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
      await loadBillingInvoices({ force: true });
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

  async function loadTimeline(options: { force?: boolean } = {}) {
    if (!canReadAudit) return;
    const key = getCustomerResourceKey();
    if (!options.force && timelineResourceLoader.hasLoaded(key)) {
      timelineType = 'all';
      return;
    }
    loadingTimeline = true;
    try {
      const result = await timelineResourceLoader.load(
        key,
        async () => {
          const [res, locRows, subRes] = await Promise.all([
            api.audit.listTenant(1, 100, { customer_id: customerId }),
            locationsResourceLoader.hasLoaded(key)
              ? Promise.resolve(locations)
              : api.customers.locations.list(customerId).catch(() => [] as CustomerLocation[]),
            api.customers.subscriptions
              .list(customerId, { page: 1, per_page: 500 })
              .catch(() => ({ data: [] as CustomerSubscriptionView[] }) as any),
          ]);

          const allowedLocationIds = new Set((locRows || []).map((l) => l.id));
          const allowedSubscriptionIds = new Set(
            ((subRes?.data as CustomerSubscriptionView[]) || []).map((s) => s.id),
          );

          return (res.data || []).filter((log) => {
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
        },
        options,
      );
      if (result.status === 'loaded') {
        timelineLogs = result.value;
      }
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
      loadLocations({ force: true }),
      activeTab === 'subscriptions' ? loadSubscriptions({ force: true }) : Promise.resolve(),
      activeTab === 'billing' ? loadBillingInvoices({ force: true }) : Promise.resolve(),
      activeTab === 'pppoe' ? loadPppoeAccounts({ force: true }) : Promise.resolve(),
      activeTab === 'timeline' && canReadAudit ? loadTimeline({ force: true }) : Promise.resolve(),
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

  async function openCreateSubscription() {
    await ensureCustomerDetailDialogsLoaded();
    resetSubscriptionForm();
    subCurrency = subCurrency || 'IDR';
    showAddSubscription = true;
  }

  async function openEditSubscription(row: CustomerSubscriptionView) {
    await ensureCustomerDetailDialogsLoaded();
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
      await loadSubscriptions({ force: true });
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
      await loadSubscriptions({ force: true });
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
      await loadSubscriptions({ force: true });
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
      await loadSubscriptions({ force: true });
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

  async function loadPppoeAccounts(options: { force?: boolean } = {}) {
    const key = getPppoeResourceKey();
    if (!options.force && pppoeResourceLoader.hasLoaded(key)) return;
    loadingPppoe = true;
    try {
      const result = await pppoeResourceLoader.load(
        key,
        async () => {
          const res = await api.pppoe.accounts.list({
            customer_id: customerId,
            q: pppoeQuery.trim() || undefined,
            page: 1,
            per_page: 200,
          });
          return res.data || [];
        },
        options,
      );
      if (result.status === 'loaded') {
        pppoeAccounts = result.value;
      }
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

  async function openEditPppoe(row: PppoeAccountPublic) {
    await ensureCustomerDetailDialogsLoaded();
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
      await loadPppoeAccounts({ force: true });
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
      await loadPppoeAccounts({ force: true });
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
      await loadPppoeAccounts({ force: true });
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
      await loadLocations({ force: true });
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

  async function openCreateLocation() {
    await ensureCustomerDetailDialogsLoaded();
    editingLocation = null;
    resetLocationForm();
    showAddLocation = true;
  }

  async function openEditLocation(row: CustomerLocation) {
    await ensureCustomerDetailDialogsLoaded();
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
      await loadLocations({ force: true });
    } catch (e: any) {
      toast.error(`Failed to update location: ${e?.message || e}`);
    } finally {
      updatingLocation = false;
    }
  }

  async function confirmDeleteLocation(row: CustomerLocation) {
    await ensureCustomerDetailDialogsLoaded();
    locationToDelete = row;
    showDeleteLocation = true;
  }

  async function openDeleteCustomerConfirm() {
    await ensureCustomerDetailDialogsLoaded();
    showDeleteCustomer = true;
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
          <button
            class="btn btn-secondary"
            title={whatsappActionTitle()}
            disabled={!customer?.phone || !whatsappGatewayReady}
            onclick={openWhatsAppCompose}
          >
            <Icon name="message-circle" size={16} />
            WhatsApp
          </button>
          <button
            class="btn btn-secondary"
            title={customer?.email
              ? $t('admin.customers.communication.actions.send_email')
              : $t('admin.customers.communication.email_not_set')}
            disabled={!customer?.email}
            onclick={openEmailCompose}
          >
            <Icon name="mail" size={16} />
            Email
          </button>
          <button class="btn btn-danger" onclick={() => void openDeleteCustomerConfirm()}>
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
            <button class="btn btn-primary" onclick={() => void openCreateLocation()}>
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
                  onclick={() => void loadLocations({ force: true })}
                >
                  <Icon name="refresh-cw" size={16} />
                </button>
                {#if canManageCustomerLocations}
                  <button
                    class="btn-icon"
                    title={$t('common.edit') || 'Edit'}
                    onclick={() => void openEditLocation(loc)}
                  >
                    <Icon name="edit-3" size={16} />
                  </button>
                  <button
                    class="btn-icon danger"
                    title={$t('common.delete') || 'Delete'}
                    onclick={() => void confirmDeleteLocation(loc)}
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
      {#if SubscriptionsTabComponent}
        <SubscriptionsTabComponent
          t={t}
          loadingSubscriptions={loadingSubscriptions}
          loadingLifecycleObservability={loadingLifecycleObservability}
          lifecycleObservability={lifecycleObservability}
          metricCount={metricCount}
          agingBucketCount={agingBucketCount}
          timeAgo={timeAgo}
          subscriptionColumns={subscriptionColumns}
          subscriptions={subscriptions}
          subscriptionStatusLabel={subscriptionStatusLabel}
          canManageCustomers={$can('manage', 'customers')}
          onRefresh={() => loadSubscriptions({ force: true })}
          onAdd={openCreateSubscription}
          onGenerateInvoice={generateInvoiceForSubscription}
          generatingInvoiceFor={generatingInvoiceFor}
          deletingSubscription={deletingSubscription}
          onSetSubscriptionStatus={setSubscriptionStatus}
          togglingSubscription={togglingSubscription}
          onEditSubscription={openEditSubscription}
          onDeleteSubscription={deleteSubscription}
        />
      {:else if activeDeferredTabLoading === 'subscriptions'}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading') || 'Loading...'}</p>
        </div>
      {/if}
    {:else if activeTab === 'billing'}
      {#if BillingTabComponent}
        <BillingTabComponent
          t={t}
          bind:billingStatus
          bind:billingDateFrom
          bind:billingDateTo
          bind:billingQuickRange
          onApplyQuickRange={applyBillingQuickRange}
          onBillingDateChange={onBillingDateChange}
          onClearFilters={clearBillingFilters}
          onRefresh={() => loadBillingInvoices({ force: true })}
          loadingBilling={loadingBilling}
          billingStats={billingStats}
          billingColumns={billingColumns}
          billingRows={billingRows}
          getSubscriptionIdFromInvoice={getSubscriptionIdFromInvoice}
          subscriptionById={subscriptionById}
          billingStatusLabel={billingStatusLabel}
          formatMoney={formatMoney}
          onOpenInvoiceDetail={openInvoiceDetail}
        />
      {:else if activeDeferredTabLoading === 'billing'}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading') || 'Loading...'}</p>
        </div>
      {/if}
    {:else if activeTab === 'pppoe'}
      {#if PppoeTabComponent && pppoeHelperModule}
        <PppoeTabComponent
          t={t}
          pppoeToolbar={pppoeToolbar}
          bind:pppoeQuery
          onRefresh={() => loadPppoeAccounts({ force: true })}
          loadingPppoe={loadingPppoe}
          pppoeColumns={pppoeColumns}
          pppoeAccounts={pppoeAccounts}
          pppoeRouters={pppoeRouters}
          locations={locations}
          getPppoeSyncDisplay={pppoeHelperModule.getPppoeSyncDisplay}
          getPppoeProvisioningTargetFallback={pppoeHelperModule.getPppoeProvisioningTargetFallback}
          getPppoeApplyActionFallback={pppoeHelperModule.getPppoeApplyActionFallback}
          timeAgo={timeAgo}
          canManagePppoe={$can('manage', 'pppoe')}
          onApplyPppoe={applyPppoe}
          onEditPppoe={openEditPppoe}
          onDeletePppoe={deletePppoe}
        />
      {:else if activeDeferredTabLoading === 'pppoe' || !pppoeHelperModule}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading') || 'Loading...'}</p>
        </div>
      {/if}
    {:else if activeTab === 'timeline'}
      {#if TimelineTabComponent && timelineHelperModule}
        <TimelineTabComponent
          loadingTimeline={loadingTimeline}
          onRefresh={() => loadTimeline({ force: true })}
          bind:timelineType
          timelineColumns={timelineColumns}
          timelineRows={timelineRows}
          timeAgo={timeAgo}
        />
      {:else if activeDeferredTabLoading === 'timeline' || !timelineHelperModule}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading') || 'Loading...'}</p>
        </div>
      {/if}
    {/if}
  {/if}
</div>

<Modal
  show={showWhatsAppCompose}
  title={$t('admin.customers.communication.title_whatsapp') || 'Send WhatsApp'}
  onclose={() => (showWhatsAppCompose = false)}
>
  <div class="form">
    {#if customer}
      <div class="compose-target">
        <div>
          <strong>{customer.name}</strong>
          <span>{customer.phone}</span>
        </div>
        <span class="status-pill" class:is-active={whatsappGatewayReady}>
          {whatsappGatewayReady
            ? `${whatsappGatewayProvider || 'gateway'} ${$t('admin.customers.communication.gateway_ready') || 'ready'}`
            : whatsappGatewayReason || $t('admin.customers.communication.gateway_not_ready') || 'Gateway not ready'}
        </span>
      </div>
      <label>
        <span>{$t('admin.customers.communication.template') || 'Template'}</span>
        <select
          class="input"
          bind:value={selectedWhatsappTemplateId}
          onchange={(event) => applyWhatsAppTemplate(event.currentTarget.value)}
        >
          {#each whatsappTemplateOptions as template}
            <option value={template.id}>{template.name}</option>
          {/each}
          <option value="custom">{$t('admin.customers.communication.custom_message') || 'Custom message'}</option>
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.communication.message') || 'Message'}</span>
        <textarea class="input" rows="7" bind:value={whatsappMessage}></textarea>
      </label>
      <div class="compose-footnote">
        <span>{whatsappMessage.trim().length} {$t('admin.customers.communication.characters') || 'characters'}</span>
        {#if !whatsappGatewayReady}
          <span>{whatsappGatewayReason}</span>
        {/if}
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick={() => (showWhatsAppCompose = false)}>
          {$t('common.cancel') || 'Cancel'}
        </button>
        <button
          class="btn btn-primary"
          onclick={sendCustomerWhatsApp}
          disabled={!whatsappGatewayReady || whatsappSending || !whatsappMessage.trim()}
        >
          <Icon name="send" size={16} />
          {whatsappSending
            ? $t('admin.customers.communication.actions.sending') || 'Sending...'
            : $t('admin.customers.communication.actions.send') || 'Send'}
        </button>
      </div>
    {/if}
  </div>
</Modal>

<Modal
  show={showEmailCompose}
  title={$t('admin.customers.communication.title_email') || 'Send Email'}
  onclose={() => (showEmailCompose = false)}
>
  <div class="form">
    {#if customer}
      <div class="compose-target">
        <div>
          <strong>{customer.name}</strong>
          <span>{customer.email}</span>
        </div>
        <span class="status-pill is-active">{$t('admin.customers.communication.email_outbox') || 'Email outbox'}</span>
      </div>
      <label>
        <span>{$t('admin.customers.communication.template') || 'Template'}</span>
        <select
          class="input"
          bind:value={selectedEmailTemplateId}
          onchange={(event) => applyEmailTemplate(event.currentTarget.value)}
        >
          {#each emailTemplateOptions as template}
            <option value={template.id}>{template.name}</option>
          {/each}
          <option value="custom">{$t('admin.customers.communication.custom_email') || 'Custom email'}</option>
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.communication.subject') || 'Subject'}</span>
        <input class="input" bind:value={emailSubject} />
      </label>
      <label>
        <span>{$t('admin.customers.communication.body') || 'Body'}</span>
        <textarea class="input" rows="9" bind:value={emailBody}></textarea>
      </label>
      <div class="compose-footnote">
        <span>{emailBody.trim().length} {$t('admin.customers.communication.characters') || 'characters'}</span>
        <span>{$t('admin.customers.communication.queued_through_outbox') || 'Queued through email outbox'}</span>
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick={() => (showEmailCompose = false)}>
          {$t('common.cancel') || 'Cancel'}
        </button>
        <button
          class="btn btn-primary"
          onclick={sendCustomerEmail}
          disabled={emailSending || !emailSubject.trim() || !emailBody.trim()}
        >
          <Icon name="send" size={16} />
          {emailSending
            ? $t('admin.customers.communication.actions.sending') || 'Sending...'
            : $t('admin.customers.communication.actions.send_email') || 'Send Email'}
        </button>
      </div>
    {/if}
  </div>
</Modal>

{#if CustomerDetailDialogsComponent}
  {@const CustomerDialogs = CustomerDetailDialogsComponent}
  <CustomerDialogs
    {t}
    bind:showEditPppoe
    bind:pppoeRouterId
    {pppoeRouters}
    {loadingPppoeRouters}
    bind:pppoePackageId
    {pppoePackageOptions}
    onPppoeRouterChange={() => {
      pppoePackageId = '';
      pppoeRouterProfileName = '';
      pppoeRemoteAddress = '';
      pppoeAddressPool = '';
    }}
    onPppoePackageChange={() => applyPppoePackage(pppoePackageId)}
    {pppoePackageSelectionHasMissingMapping}
    bind:pppoeUsername
    bind:pppoePassword
    bind:pppoeComment
    bind:pppoeDisabled
    {savingPppoe}
    onSubmitUpdatePppoe={submitUpdatePppoe}
    bind:showAddSubscription
    bind:subLocationId
    {subscriptionLocationOptions}
    bind:subPackageId
    {subscriptionPackageOptions}
    bind:subRouterId
    {subscriptionRouterOptions}
    bind:subBillingCycle
    {billingCycleOptions}
    bind:subPrice
    bind:subCurrency
    bind:subStatus
    {subscriptionStatusOptions}
    bind:subStartsAt
    bind:subEndsAt
    bind:subNotes
    {savingSubscription}
    onSubmitCreateSubscription={submitCreateSubscription}
    bind:showEditSubscription
    onCloseEditSubscription={() => {
      showEditSubscription = false;
      editingSubscription = null;
    }}
    onSubmitUpdateSubscription={submitUpdateSubscription}
    bind:showAddLocation
    bind:locLabel
    bind:locAddress1
    bind:locAddress2
    bind:locCity
    bind:locState
    bind:locPostal
    bind:locCountry
    bind:locLatitude
    bind:locLongitude
    bind:locNotes
    {creatingLocation}
    onAddLocation={addLocation}
    bind:showEditLocation
    {updatingLocation}
    onSubmitUpdateLocation={submitUpdateLocation}
    bind:showDeleteCustomer
    {deletingCustomer}
    onDeleteCustomer={doDeleteCustomer}
    bind:showDeleteLocation
    {deletingLocation}
    onDeleteLocation={doDeleteLocation}
  />
{/if}

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
    min-width: 0;
  }

  .avatar {
    width: 52px;
    height: 52px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    font-weight: 800;
    letter-spacing: 0.4px;
    color: var(--color-primary);
    background: var(--bg-surface);
    border: 1px solid color-mix(in srgb, var(--color-primary) 34%, var(--border-color));
    box-shadow: inset 0 1px 0 color-mix(in srgb, var(--text-primary) 10%, transparent);
  }

  .meta h1 {
    margin: 0;
    font-size: 1.65rem;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
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
    border-color: color-mix(in srgb, var(--color-success) 30%, var(--border-color));
    color: var(--color-success);
    background: var(--bg-success);
  }

  .status-pill.is-inactive {
    border-color: color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
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
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 58%, var(--border-color));
    color: white;
  }

  .btn-primary:hover {
    background: var(--color-primary-hover);
  }

  .btn-secondary {
    background: var(--bg-surface);
  }

  .btn-danger {
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    color: var(--color-danger);
  }

  .btn-danger:hover {
    background: color-mix(in srgb, var(--color-danger) 16%, transparent);
  }

  .btn-warning {
    border-color: color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-warning);
  }

  .btn-warning:hover {
    background: color-mix(in srgb, var(--color-warning) 16%, transparent);
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
    border-color: color-mix(in srgb, var(--color-primary) 52%, var(--border-color));
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
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
    border-color: color-mix(in srgb, var(--color-primary) 52%, var(--border-color));
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
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
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .observability-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.9rem;
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
    border-color: color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
  }

  .metric-label {
    display: block;
    font-size: 0.78rem;
    color: var(--text-secondary);
    margin-bottom: 0.35rem;
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

  .compose-target {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.8rem;
  }

  .compose-target div {
    min-width: 0;
  }

  .compose-target strong,
  .compose-target div span {
    display: block;
  }

  .compose-target div span {
    color: var(--text-secondary);
    font-size: 0.88rem;
    overflow-wrap: anywhere;
  }

  .compose-footnote {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    color: var(--text-secondary);
    font-size: 0.82rem;
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
    border-color: color-mix(in srgb, var(--color-primary) 50%, var(--border-color));
    background: var(--color-primary-subtle);
    color: var(--color-primary);
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
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-color));
    color: var(--color-danger);
  }

  .badge.danger {
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-color));
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
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
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    animation: spin 0.9s linear infinite;
  }

  .callout {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .timeline-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    margin-bottom: 0.75rem;
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
    .header-actions .btn {
      flex: 1 1 11rem;
      min-width: 0;
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
    .actions {
      justify-content: stretch;
      flex-wrap: wrap;
    }
  }
</style>
