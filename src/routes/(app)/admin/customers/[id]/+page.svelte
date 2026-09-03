<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { Component } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { secureGetItem } from '$lib/utils/tauri-store';
  import { fetchAllRows } from '$lib/utils/fetchAllPages';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import {
    api,
    type AuditLog,
    type Customer,
    type CustomerPortalUser,
    type CustomerLifecycleObservability,
    type DhcpStaticServicePublic,
    type CustomerLocation,
    type NetworkAssetListItem,
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
  import {
    buildCustomerSubscriptionAccessState,
    buildCustomerSubscriptionPolicySummary,
    clampFixedSuspendDay,
  } from '$lib/utils/customerSubscriptionPolicy';
  import { formatDate, timeAgo } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import { getAdminCustomerNavigation } from '$lib/utils/adminCustomerNavigation';
  import { resolveBackTarget } from '$lib/utils/backNavigation';
  import {
    formatLocationCoordinates,
    validateOptionalCoordinates,
  } from '$lib/utils/customerLocationCoordinates';

  import Icon from '$lib/components/ui/Icon.svelte';
  import MobileOverflowActions from '$lib/components/ui/MobileOverflowActions.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
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
    loadCustomerAssetsTab,
    loadCustomerBillingTab,
    loadCustomerPppoeTab,
    loadCustomerSubscriptionsTab,
    loadCustomerTimelineTab,
  } from './customerDetailTabModules';
  import {
    buildCustomerBillingStats,
    filterCustomerBillingRows,
    type CustomerBillingFilter,
  } from './customerBillingState';

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
  const customerBackTarget = $derived(resolveBackTarget($page.url, customersPath));

  let activeTab = $state<CustomerDetailTab>('overview');
  let isMobile = $state(false);
  let CustomerDetailDialogsComponent = $state<DeferredComponent | null>(null);
  let SubscriptionsTabComponent = $state<DeferredComponent | null>(null);
  let BillingTabComponent = $state<DeferredComponent | null>(null);
  let AssetsTabComponent = $state<DeferredComponent | null>(null);
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
  let subscriptionsLoadSequence = 0;
  let lifecycleObservability = $state<CustomerLifecycleObservability | null>(null);
  let loadingLifecycleObservability = $state(false);
  let timelineLogs = $state<AuditLog[]>([]);
  let timelineType = $state<'all' | 'customer' | 'location' | 'subscription'>('all');
  let loadingTimeline = $state(false);
  let timelineLoadSequence = 0;
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
  let billingFilter = $state<CustomerBillingFilter>('all');
  let generatingInvoiceFor = $state<string | null>(null);

  // Change package (pro-rata upgrade/downgrade)
  let showChangePackage = $state(false);
  let changePackageSubscription = $state<CustomerSubscriptionView | null>(null);
  let changePackageNewId = $state('');
  let changePackageLoading = $state(false);
  let changePackageResult = $state<any>(null);
  const subscriptionMutationBusy = $derived(
    Boolean(savingSubscription || deletingSubscription || togglingSubscription || generatingInvoiceFor || changePackageLoading),
  );

  // PPPoE
  let pppoeAccounts = $state<PppoeAccountPublic[]>([]);
  let dhcpStaticServices = $state<DhcpStaticServicePublic[]>([]);
  let loadingDhcpStatic = $state(false);
  let customerAssets = $state<NetworkAssetListItem[]>([]);
  let loadingCustomerAssets = $state(false);
  let assetsLoadSequence = 0;
  let dhcpStaticLoadSequence = 0;
  let loadingPppoe = $state(false);
  let pppoeQuery = $state('');
  let pppoeRouters = $state<any[]>([]);
  let loadingPppoeRouters = $state(false);
  let showEditPppoe = $state(false);
  let editingPppoe = $state<PppoeAccountPublic | null>(null);
  let savingPppoe = $state(false);
  let applyingPppoe = $state<string | null>(null);
  let deletingPppoe = $state<string | null>(null);

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
  let showSubDeleteConfirm = $state(false);
  let subToDeleteId = $state<string | null>(null);
  let showPppoeDeleteConfirm = $state(false);
  let pppoeToDeleteId = $state<string | null>(null);
  let deletingCustomer = $state(false);

  // Portal Users
  let portalUsers = $state<CustomerPortalUser[]>([]);
  let loadingPortalUsers = $state(false);
  let showAddPortalUser = $state(false);
  let portalUserEmail = $state('');
  let portalUserName = $state('');
  let portalUserPassword = $state('');
  let portalUserPasswordConfirm = $state('');
  let addingPortalUser = $state(false);

  let showResetPasswordConfirm = $state(false);
  let portalUserToReset = $state<CustomerPortalUser | null>(null);
  let resettingPassword = $state(false);
  let manualResetPassword = $state('');
  let manualResetPasswordConfirm = $state('');
  let generatedPasswordResult = $state<string | null>(null);

  let showRemovePortalUserConfirm = $state(false);
  let portalUserToRemove = $state<CustomerPortalUser | null>(null);
  let removingPortalUser = $state(false);
  const portalMutationBusy = $derived(Boolean(addingPortalUser || removingPortalUser || resettingPassword));

  const locationsResourceLoader = createCustomerDetailResourceLoader<CustomerLocation[]>();
  const subscriptionsResourceLoader = createCustomerDetailResourceLoader<{
    rows: CustomerSubscriptionView[];
    lifecycle: CustomerLifecycleObservability | null;
  }>();
  const billingResourceLoader = createCustomerDetailResourceLoader<Invoice[]>();
  const customerAssetsResourceLoader = createCustomerDetailResourceLoader<NetworkAssetListItem[]>();
  const pppoeResourceLoader = createCustomerDetailResourceLoader<PppoeAccountPublic[]>();
  const dhcpStaticResourceLoader = createCustomerDetailResourceLoader<DhcpStaticServicePublic[]>();
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
    { key: 'package', label: $t('common.package') || 'Package' },
    { key: 'billing', label: $t('admin.customers.tabs.billing') || 'Billing' },
    { key: 'location', label: $t('common.location') || 'Location' },
    { key: 'router', label: 'Router' },
    { key: 'installation', label: $t('admin.customers.subscriptions.installation') || 'Installation' },
    { key: 'lifecycle', label: 'Lifecycle' },
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
    { label: $t('common.active') || 'Active', value: 'active' },
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
  const canCreateOrders = $derived($can('create', 'orders'));
  const canReadCustomerLocations = $derived(
    $can('read', 'customer_locations') || $can('manage', 'customer_locations'),
  );
  const canManageCustomerLocations = $derived($can('manage', 'customer_locations'));
  const canReadBilling = $derived($can('read', 'billing') || $can('manage', 'billing'));
  const canReadFtthAssets = $derived($can('read', 'ftth_assets') || $can('manage', 'ftth_assets'));
  const canReadAudit = $derived($can('read', 'audit_logs'));
  const canReadPppoe = $derived($can('read', 'pppoe') || $can('manage', 'pppoe'));
  const canReadDhcpStatic = $derived($can('read', 'dhcp_static') || $can('manage', 'dhcp_static'));
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
      canReadFtthAssets,
      canReadPppoe,
      canReadDhcpStatic,
      canReadAudit,
    }),
  );
  const customerActionItems = $derived.by(() => {
    const items: Array<{
      id: string;
      label: string;
      icon: string;
      tone?: 'default' | 'primary' | 'warning' | 'danger';
      disabled?: boolean;
    }> = [
      {
        id: 'refresh',
        label: $t('common.refresh') || 'Refresh',
        icon: 'refresh-cw',
      },
    ];

    if (canCreateOrders && customer) {
      items.push({
        id: 'create-order',
        label: $t('admin.network.installations.create_order_btn') || 'Create Order',
        icon: 'file-text',
      });
    }

    if (canManageCustomers && customer) {
      items.push({
        id: customer.is_active ? 'suspend' : 'activate',
        label: customer.is_active ? 'Suspend' : 'Activate',
        icon: customer.is_active ? 'pause' : 'play',
        tone: customer.is_active ? 'warning' : 'primary',
        disabled: togglingCustomerStatus,
      });
      items.push({
        id: 'whatsapp',
        label: 'WhatsApp',
        icon: 'message-circle',
        disabled: !customer.phone || !whatsappGatewayReady,
      });
      items.push({
        id: 'email',
        label: 'Email',
        icon: 'mail',
        disabled: !customer.email,
      });
      items.push({
        id: 'delete',
        label: $t('common.delete') || 'Delete',
        icon: 'trash-2',
        tone: 'danger',
      });
    }

    return items;
  });
  const customerTabItems = $derived.by(() => {
    const panelId = 'customer-detail-panel';
    const items: Array<{ id: CustomerDetailTab; label: string; panelId: string }> = [
      { id: 'overview', label: $t('admin.customers.tabs.overview') || 'Overview', panelId },
    ];
    if (visibleTabs.includes('billing')) {
      items.push({ id: 'billing', label: $t('admin.customers.tabs.billing') || 'Billing', panelId });
    }
    if (visibleTabs.includes('subscriptions')) {
      items.push({
        id: 'subscriptions',
        label: $t('admin.customers.tabs.subscriptions') || 'Subscriptions',
        panelId,
      });
    }
    if (visibleTabs.includes('locations')) {
      items.push({ id: 'locations', label: $t('admin.customers.tabs.locations') || 'Locations', panelId });
    }
    if (visibleTabs.includes('assets')) {
      items.push({ id: 'assets', label: $t('admin.customers.tabs.assets') || 'FTTH Assets', panelId });
    }
    if (visibleTabs.includes('pppoe')) {
      items.push({ id: 'pppoe', label: $t('admin.customers.tabs.pppoe') || 'PPPoE', panelId });
    }
    if (visibleTabs.includes('dhcp_static')) {
      items.push({
        id: 'dhcp_static',
        label: $t('admin.customers.tabs.dhcp_static') || 'DHCP Static',
        panelId,
      });
    }
    if (visibleTabs.includes('timeline')) {
      items.push({ id: 'timeline', label: $t('admin.customers.tabs.timeline') || 'Timeline', panelId });
    }
    return items;
  });
  const customerDetailAccess = $derived.by(() => ({
    canReadCustomerLocations,
    canReadBilling,
    canReadFtthAssets,
    canReadPppoe,
    canReadDhcpStatic,
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
    { key: 'created_at', label: $t('common.updated') || 'Waktu' },
    { key: 'action', label: $t('common.action') || 'Aksi' },
    { key: 'resource', label: 'Resource' },
    { key: 'actor', label: 'Actor' },
    { key: 'details', label: $t('common.details') || 'Detail' },
  ]);
  const timelineRows = $derived.by(() =>
    timelineHelperModule ? timelineHelperModule.buildCustomerTimelineRows(timelineFilteredLogs) : [],
  );
  const billingRows = $derived.by(() =>
    filterCustomerBillingRows({
      invoices: billingInvoices,
      subscriptionById,
      getSubscriptionIdFromInvoice,
      filter: billingFilter,
    }),
  );
  const billingStats = $derived.by(() =>
    buildCustomerBillingStats({
      invoices: billingInvoices,
      subscriptionById,
      getSubscriptionIdFromInvoice,
    }),
  );

  onMount(() => {
    const mq = window.matchMedia('(max-width: 900px)');
    const updateViewport = () => {
      isMobile = mq.matches;
    };
    updateViewport();
    mq.addEventListener('change', updateViewport);

    void (async () => {
      if (!canReadCustomers) {
        goto('/unauthorized');
        return;
      }
      const fromUrl = readActiveTabFromUrl();
      if (fromUrl) activeTab = fromUrl;
      await Promise.all([
        loadCustomer(),
        loadPortalUsers(),
        canManageCustomers ? loadCommunicationReadiness() : Promise.resolve(),
        canManageCustomers ? loadCommunicationTemplates() : Promise.resolve(),
      ]);
      if (canReadCustomerLocations) {
        await loadLocations({ force: true });
      }
    })();

    return () => {
      mq.removeEventListener('change', updateViewport);
    };
  });

  function handleCustomerActionSelect(actionId: string) {
    if (actionId === 'refresh') {
      void refreshCurrent();
      return;
    }
    if (actionId === 'create-order') {
      openCreateOrderForCustomer();
      return;
    }
    if (actionId === 'suspend') {
      void setCustomerActive(false);
      return;
    }
    if (actionId === 'activate') {
      void setCustomerActive(true);
      return;
    }
    if (actionId === 'whatsapp') {
      openWhatsAppCompose();
      return;
    }
    if (actionId === 'email') {
      openEmailCompose();
      return;
    }
    if (actionId === 'delete') {
      void openDeleteCustomerConfirm();
    }
  }

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
    if (tab === 'assets') {
      if (AssetsTabComponent) return;
      activeDeferredTabLoading = tab;
      const module = await loadCustomerAssetsTab();
      AssetsTabComponent = module.default;
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
      activeTab === 'assets' ||
      activeTab === 'pppoe' ||
      activeTab === 'dhcp_static' ||
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
    if (activeTab !== 'assets') return;
    if (!canReadFtthAssets) return;
    untrack(() => {
      void loadCustomerAssets({ force: true });
    });
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
    if (activeTab !== 'dhcp_static') return;
    if (!canReadDhcpStatic) return;
    untrack(() => {
      void loadDhcpStaticServices();
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

  async function loadPortalUsers() {
    loadingPortalUsers = true;
    try {
      portalUsers = await api.customers.portalUsers.list(customerId);
    } catch (e: any) {
      toast.error(get(t)('admin.customers.portal.toasts.load_failed'));
    } finally {
      loadingPortalUsers = false;
    }
  }

  function openAddPortalUser() {
    portalUserEmail = customer?.email || '';
    portalUserName = customer?.name || '';
    portalUserPassword = '';
    portalUserPasswordConfirm = '';
    showAddPortalUser = true;
  }

  async function addPortalUser() {
    const email = portalUserEmail.trim();
    const name = portalUserName.trim();
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return;
    if (!email || !name || portalUserPassword.length < 6 || portalUserPassword !== portalUserPasswordConfirm) return;
    if (addingPortalUser || removingPortalUser || resettingPassword) return;
    addingPortalUser = true;
    try {
      await api.customers.portalUsers.createNew({
        customer_id: customerId,
        email,
        name,
        password: portalUserPassword,
      });
      toast.success(get(t)('admin.customers.portal.toasts.created'));
      showAddPortalUser = false;
      portalUserEmail = '';
      portalUserName = '';
      portalUserPassword = '';
      portalUserPasswordConfirm = '';
      await loadPortalUsers();
    } catch (e: any) {
      toast.error(get(t)('admin.customers.portal.toasts.create_failed'));
    } finally {
      addingPortalUser = false;
    }
  }

  async function confirmRemovePortalUser() {
    if (!portalUserToRemove || addingPortalUser || removingPortalUser || resettingPassword) return;
    removingPortalUser = true;
    try {
      await api.customers.portalUsers.remove(portalUserToRemove.customer_user_id);
      toast.success(get(t)('admin.customers.portal.toasts.removed'));
      showRemovePortalUserConfirm = false;
      portalUserToRemove = null;
      await loadPortalUsers();
    } catch (e: any) {
      toast.error(get(t)('admin.customers.portal.toasts.remove_failed'));
    } finally {
      removingPortalUser = false;
    }
  }

  async function confirmResetPassword() {
    if (!portalUserToReset || addingPortalUser || removingPortalUser || resettingPassword) return;
    resettingPassword = true;
    try {
      const res = await api.customers.portalUsers.resetPassword(
        portalUserToReset.customer_user_id,
        manualResetPassword.trim() || undefined
      );
      if (res.generated_password) {
        generatedPasswordResult = res.generated_password;
        toast.success(get(t)('admin.customers.portal.reset_password.success_generated'));
      } else {
        toast.success(get(t)('admin.customers.portal.reset_password.success_manual'));
        showResetPasswordConfirm = false;
        portalUserToReset = null;
        manualResetPassword = '';
        manualResetPasswordConfirm = '';
      }
    } catch (e: any) {
      toast.error(get(t)('admin.customers.portal.toasts.reset_failed'));
    } finally {
      resettingPassword = false;
    }
  }

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
      toast.error(get(t)('admin.customers.communication.load_templates_failed'));
    }
  }

  function currentTenantName() {
    if ($page.data?.tenant?.name) return $page.data.tenant.name;
    if (typeof localStorage === 'undefined') return '';
    try {
      return JSON.parse(secureGetItem('auth_tenant') || '{}')?.name || '';
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
    const loadSequence = ++subscriptionsLoadSequence;
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
      toast.error(e?.message || 'Failed to load subscriptions');
    } finally {
      if (loadSequence === subscriptionsLoadSequence) {
        loadingSubscriptions = false;
        loadingLifecycleObservability = false;
      }
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

  const customerSubscriptionPolicy = $derived.by(() => ({
    enabled: String($appSettings['billing_auto_suspend_enabled'] ?? 'false') === 'true',
    mode: (
      String($appSettings['billing_auto_suspend_mode'] || 'grace_period') === 'fixed_day'
        ? 'fixed_day'
        : 'grace_period'
    ) as 'grace_period' | 'fixed_day',
    graceDays: Number.parseInt(String($appSettings['billing_auto_suspend_grace_days'] || '3'), 10),
    fixedDay: clampFixedSuspendDay(
      Number.parseInt(String($appSettings['billing_auto_suspend_fixed_day'] || '1'), 10),
    ),
  }));

  function getSubscriptionPolicySummary(row: CustomerSubscriptionView) {
    return buildCustomerSubscriptionPolicySummary({
      endsAt: row.ends_at,
      policy: customerSubscriptionPolicy,
    });
  }

  function getSubscriptionAccessState(row: CustomerSubscriptionView) {
    return buildCustomerSubscriptionAccessState({
      subscriptionStatus: row.status,
      pppoeDisabled: row.pppoe_disabled,
      pppoeAddressPool: row.pppoe_address_pool,
      isolationPool: row.pppoe_isolation_pool || String($appSettings['billing_auto_suspend_isolation_pool'] || ''),
    });
  }

  function getPppoeAccessState(row: PppoeAccountPublic) {
    const linkedSubscription = subscriptions.find((sub) => sub.location_id === row.location_id);
    return buildCustomerSubscriptionAccessState({
      subscriptionStatus: linkedSubscription?.status,
      pppoeDisabled: row.disabled,
      pppoeAddressPool: row.address_pool,
      isolationPool:
        linkedSubscription?.pppoe_isolation_pool ||
        String($appSettings['billing_auto_suspend_isolation_pool'] || ''),
    });
  }

  function formatCustomerPolicyDate(value: string | null) {
    if (!value) return '-';
    return formatDate(value, { timeZone: $appSettings.app_timezone || 'UTC' });
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
      canReadDhcpStatic,
      canReadFtthAssets,
      canReadAudit,
    });
  }

  function selectCustomerTab(tab: string) {
    const next = normalizeCustomerDetailTab(tab, customerDetailAccess);
    activeTab = next;
    const url = new URL($page.url);
    if (next === 'overview') url.searchParams.delete('tab');
    else url.searchParams.set('tab', next);
    void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
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
        /* Harus menarik SEMUA halaman, bukan `listCustomerPackageInvoices()`
           telanjang. Backend memakai `per_page.unwrap_or(25)` lalu
           `clamp(1, 100)`, jadi panggilan tanpa argumen hanya mengembalikan 25
           invoice TERBARU se-tenant — bukan 25 invoice milik pelanggan ini.
           Baris lalu difilter client-side ke langganan pelanggan yang dibuka,
           sehingga tab Tagihan tampak kosong padahal datanya ada.
           Terukur di DB produksi: 485 invoice paket milik 482 langganan,
           sementara 25 terbaru hanya menyentuh 24 langganan — 453 pelanggan
           kehilangan riwayat tagihannya. */
        () => fetchAllRows<Invoice>((page, per_page) =>
          api.payment.listCustomerPackageInvoices({ page, per_page }),
        ),
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

  function onSelectBillingFilter(filter: CustomerBillingFilter) {
    billingFilter = billingFilter === filter ? 'all' : filter;
  }

  async function generateInvoiceForSubscription(subscriptionId: string) {
    if (!subscriptionId || subscriptionMutationBusy) return;
    generatingInvoiceFor = subscriptionId;
    try {
      await api.payment.createInvoiceForCustomerSubscription(subscriptionId);
      toast.success(
        get(t)('admin.customers.billing.toasts.generated') || 'Invoice generated successfully',
      );
      selectCustomerTab('billing');
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

  function openCreateOrderForCustomer() {
    if (!customer) return;
    void goto(`${customersPath}/orders/new?customer_id=${encodeURIComponent(customer.id)}`);
  }

  async function loadTimeline(options: { force?: boolean } = {}) {
    if (!canReadAudit) return;
    const key = getCustomerResourceKey();
    if (!options.force && timelineResourceLoader.hasLoaded(key)) return;
    const loadSequence = ++timelineLoadSequence;
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
    } catch (e: any) {
      toast.error(get(t)('admin.customers.timeline.toasts.load_failed'));
    } finally {
      if (loadSequence === timelineLoadSequence) loadingTimeline = false;
    }
  }

  async function refreshCurrent() {
    await Promise.all([
      loadCustomer(),
      loadPortalUsers(),
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
    if (subscriptionMutationBusy) return;
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
      toast.success(get(t)('admin.customers.subscriptions.toasts.created'));
      showAddSubscription = false;
      selectCustomerTab('subscriptions');
      await loadSubscriptions({ force: true });
    } catch (e: any) {
      toast.error(get(t)('admin.customers.subscriptions.toasts.create_failed', { values: { message: e?.message || e } }));
    } finally {
      savingSubscription = false;
    }
  }

  async function submitUpdateSubscription() {
    if (!editingSubscription || subscriptionMutationBusy) return;
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
      toast.success(get(t)('admin.customers.subscriptions.toasts.updated'));
      showEditSubscription = false;
      editingSubscription = null;
      selectCustomerTab('subscriptions');
      await loadSubscriptions({ force: true });
    } catch (e: any) {
      toast.error(get(t)('admin.customers.subscriptions.toasts.update_failed', { values: { message: e?.message || e } }));
    } finally {
      savingSubscription = false;
    }
  }

  async function deleteSubscription(id: string) {
    subToDeleteId = id;
    showSubDeleteConfirm = true;
  }

  async function confirmDeleteSubscription() {
    const id = subToDeleteId;
    if (!id || subscriptionMutationBusy) return;
    deletingSubscription = id;
    showSubDeleteConfirm = false;
    try {
      await api.customers.subscriptions.delete(id);
      toast.success(get(t)('admin.customers.subscriptions.toasts.deleted'));
      await loadSubscriptions({ force: true });
    } catch (e: any) {
      toast.error(get(t)('admin.customers.subscriptions.toasts.delete_failed', { values: { message: e?.message || e } }));
    } finally {
      deletingSubscription = null;
    }
  }

  async function setSubscriptionStatus(
    row: CustomerSubscriptionView,
    nextStatus: 'active' | 'suspended',
  ) {
    if (subscriptionMutationBusy) return;
    togglingSubscription = row.id;
    try {
      await api.customers.subscriptions.update(row.id, { status: nextStatus });
      toast.success(
        get(t)(
          nextStatus === 'suspended'
            ? 'admin.customers.subscriptions.toasts.suspended'
            : 'admin.customers.subscriptions.toasts.resumed',
        ),
      );
      await loadSubscriptions({ force: true });
    } catch (e: any) {
      toast.error(get(t)('admin.customers.subscriptions.toasts.status_failed', { values: { message: e?.message || e } }));
    } finally {
      togglingSubscription = null;
    }
  }

  function openChangePackage(row: CustomerSubscriptionView) {
    if (subscriptionMutationBusy) return;
    changePackageSubscription = row;
    changePackageNewId = '';
    changePackageResult = null;
    showChangePackage = true;
    // Load available packages if not already loaded
    if (subscriptionPackages.length === 0) {
      loadSubscriptionPackages();
    }
  }

  async function submitChangePackage() {
    if (!changePackageSubscription || !changePackageNewId || subscriptionMutationBusy) return;
    changePackageLoading = true;
    changePackageResult = null;
    try {
      const result = await api.payment.changePackage({
        subscription_id: changePackageSubscription.id,
        new_package_id: changePackageNewId,
      });
      changePackageResult = result;
      toast.success(
        result.net_amount > 0
          ? `${$t('admin.customers.subscriptions.change_package.toasts.success_with_charge')} ${formatMoney(result.net_amount)}`
          : ($t('admin.customers.subscriptions.change_package.toasts.success_no_charge') || 'Paket diganti! Tidak ada tagihan tambahan'),
      );
      await loadSubscriptions({ force: true });
      await loadBillingInvoices({ force: true });
    } catch (e: any) {
      toast.error(`${$t('admin.customers.subscriptions.change_package.toasts.error')} ${e?.message || e}`);
    } finally {
      changePackageLoading = false;
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

  async function loadDhcpStaticServices(options: { force?: boolean } = {}) {
    const key = `${customerId}:${activeTab}:dhcp_static`;
    if (!options.force && dhcpStaticResourceLoader.hasLoaded(key)) return;
    const loadSequence = ++dhcpStaticLoadSequence;
    loadingDhcpStatic = true;
    try {
      const result = await dhcpStaticResourceLoader.load(
        key,
        async () => {
          const res = await api.dhcpStatic.services.list({
            customer_id: customerId,
            page: 1,
            per_page: 200,
          });
          return res.data || [];
        },
        options,
      );
      if (result.status === 'loaded' && loadSequence === dhcpStaticLoadSequence) {
        dhcpStaticServices = result.value;
      }
    } catch (e: any) {
      toast.error(get(t)('admin.customers.dhcp_static.toasts.load_failed'));
    } finally {
      if (loadSequence === dhcpStaticLoadSequence) loadingDhcpStatic = false;
    }
  }

  async function loadCustomerAssets(options: { force?: boolean } = {}) {
    const key = `${customerId}:${activeTab}:assets`;
    if (!options.force && customerAssetsResourceLoader.hasLoaded(key)) return;
    const loadSequence = ++assetsLoadSequence;
    loadingCustomerAssets = true;
    try {
      const result = await customerAssetsResourceLoader.load(
        key,
        async () => await api.networkAssets.listCustomerAssets(customerId),
        options,
      );
      if (result.status === 'loaded' && loadSequence === assetsLoadSequence) {
        customerAssets = result.value;
      }
    } catch (e: any) {
      toast.error(get(t)('admin.customers.assets.toasts.load_failed'));
    } finally {
      if (loadSequence === assetsLoadSequence) loadingCustomerAssets = false;
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
    if (applyingPppoe || deletingPppoe || savingPppoe) return;
    applyingPppoe = row.id;
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
    } finally {
      applyingPppoe = null;
    }
  }

  async function deletePppoe(row: PppoeAccountPublic) {
    pppoeToDeleteId = row.id;
    showPppoeDeleteConfirm = true;
  }

  async function confirmDeletePppoe() {
    const id = pppoeToDeleteId;
    if (!id || applyingPppoe || deletingPppoe || savingPppoe) return;
    showPppoeDeleteConfirm = false;
    deletingPppoe = id;
    try {
      await api.pppoe.accounts.delete(id);
      toast.success(get(t)('admin.customers.pppoe.toasts.deleted') || 'Deleted');
      await loadPppoeAccounts({ force: true });
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.pppoe.toasts.delete_failed', {
          values: { message: e?.message || e },
        }) || `Failed: ${e?.message || e}`,
      );
    } finally {
      deletingPppoe = null;
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
        toast.error(get(t)('admin.customers.locations.toasts.coordinates_both_required'));
      } else if (parsedCoordinates.error === 'invalid_number') {
        toast.error(get(t)('admin.customers.locations.toasts.coordinates_invalid'));
      } else if (parsedCoordinates.error === 'latitude_range') {
        toast.error(get(t)('admin.customers.locations.toasts.latitude_range'));
      } else if (parsedCoordinates.error === 'longitude_range') {
        toast.error(get(t)('admin.customers.locations.toasts.longitude_range'));
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
        toast.error(get(t)('admin.customers.locations.toasts.coordinates_both_required'));
      } else if (parsedCoordinates.error === 'invalid_number') {
        toast.error(get(t)('admin.customers.locations.toasts.coordinates_invalid'));
      } else if (parsedCoordinates.error === 'latitude_range') {
        toast.error(get(t)('admin.customers.locations.toasts.latitude_range'));
      } else if (parsedCoordinates.error === 'longitude_range') {
        toast.error(get(t)('admin.customers.locations.toasts.longitude_range'));
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
      toast.success(get(t)('admin.customers.locations.toasts.updated'));
      await loadLocations({ force: true });
    } catch (e: any) {
      toast.error(get(t)('admin.customers.locations.toasts.update_failed', { values: { message: e?.message || e } }));
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
      toast.success(get(t)('admin.customers.locations.toasts.deleted'));
      await loadLocations({ force: true });
    } catch (e: any) {
      toast.error(get(t)('admin.customers.locations.toasts.delete_failed', { values: { message: e?.message || e } }));
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
      <div class="breadcrumbs" aria-label={$t('admin.customers.detail.title') || 'Customer breadcrumbs'}>
        <button type="button" onclick={() => goto(customersPath)}>
          {$t('admin.customers.title') || 'Customers'}
        </button>
        <span aria-hidden="true">›</span>
        <b>{customer?.name || $t('admin.customers.detail.title') || 'Customer'}</b>
      </div>
      <button class="btn btn-secondary back-button" type="button" onclick={() => goto(customerBackTarget)}>
        <Icon name="arrow-left" size={16} />
        {$t('common.back')}
      </button>
      <div class="header-actions hero-actions">
        <MobileOverflowActions
          items={customerActionItems}
          primaryIds={['refresh', 'create-order']}
          {isMobile}
          on:select={(event) => handleCustomerActionSelect(event.detail)}
        />
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
            'Detail pelanggan'}
        </p>
        <div class="hero-badges">
          <span class={`status-pill ${customer?.is_active ? 'is-active' : 'is-inactive'}`}>
            <span class="dot"></span>
            {customer?.is_active
              ? $t('common.active') || 'Active'
              : $t('common.inactive') || 'Inactive'}
          </span>
          <span class="meta-pill">{customer?.updated_at ? `Updated ${timeAgo(customer.updated_at)}` : '-'}</span>
        </div>
      </div>
    </div>
  </div>

  <ResponsiveTabs
    items={customerTabItems}
    bind:activeId={activeTab}
    {isMobile}
    priorityCount={2}
    ariaLabel={$t('admin.customers.detail.title')}
    on:change={(event) => selectCustomerTab(event.detail)}
  />

  {#if loadingCustomer}
    <div class="card loading-card">
      <div class="spinner"></div>
      <p>{$t('common.loading')}</p>
    </div>
  {:else if customer}
    <div
      id="customer-detail-panel"
      role="tabpanel"
      aria-labelledby={`tab-${activeTab}`}
      class="customer-detail-panel"
    >
    {#if activeTab === 'overview'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.overview.title')}</h3>
            <p class="subtitle">{$t('admin.customers.detail.subtitle')}</p>
          </div>
          {#if canManageCustomers}
            <button
              class="btn btn-primary"
              onclick={saveOverview}
              disabled={saving || !name.trim()}
            >
              <Icon name="check-circle" size={16} />
              {$t('common.save')}
            </button>
          {/if}
        </div>

        <div class="overview-grid">
          <div class="form overview-form">
            <label>
              <span>{$t('admin.customers.fields.name')}</span>
              <input class="input" bind:value={name} disabled={!canManageCustomers} />
            </label>
            <div class="grid2">
              <label>
                <span>{$t('admin.customers.fields.email')}</span>
                <input class="input" bind:value={email} disabled={!canManageCustomers} />
              </label>
              <label>
                <span>{$t('admin.customers.fields.phone')}</span>
                <input class="input" bind:value={phone} disabled={!canManageCustomers} />
              </label>
            </div>
            <label>
              <span>{$t('admin.customers.fields.notes')}</span>
              <textarea class="input" rows="5" bind:value={notes} disabled={!canManageCustomers}
              ></textarea>
            </label>
          </div>
          <aside class="overview-side">
            <div class="side-title">{$t('admin.customers.overview.profile_quality')}</div>
            <div class="side-item">
              <span>{$t('common.name')}</span>
              <strong>{name.trim() ? ($t('common.complete') || 'Complete') : ($t('common.missing') || 'Missing')}</strong>
            </div>
            <div class="side-item">
              <span>{$t('common.email')}</span>
              <strong>{email.trim() ? ($t('common.complete') || 'Complete') : ($t('common.missing') || 'Missing')}</strong>
            </div>
            <div class="side-item">
              <span>{$t('common.phone')}</span>
              <strong>{phone.trim() ? ($t('common.complete') || 'Complete') : ($t('common.missing') || 'Missing')}</strong>
            </div>
            <div class="side-item">
              <span>{$t('common.status')}</span>
              <strong>{isActive ? ($t('common.active') || 'Active') : ($t('common.inactive') || 'Inactive')}</strong>
            </div>
            <div class="side-divider"></div>
            <p class="side-note">{$t('admin.customers.overview.contact_accuracy_note')}</p>
          </aside>
        </div>
      </div>

      <div class="card section" style="margin-top: 1.25rem;">
        <div class="section-head">
          <div>
            <h3>{get(t)('admin.customers.portal.title')}</h3>
            <p class="subtitle">{get(t)('admin.customers.portal.subtitle')}</p>
          </div>
          {#if canManageCustomers}
            <button class="btn btn-primary" type="button" onclick={openAddPortalUser} disabled={portalMutationBusy}>
              <Icon name="plus" size={16} />
              {get(t)('admin.customers.portal.actions.add')}
            </button>
          {/if}
        </div>

        {#if loadingPortalUsers}
          <div style="padding: 2rem 0; text-align: center;">
            <div class="spinner"></div>
            <p style="margin-top: 0.5rem; font-size: 0.875rem; color: var(--text-muted);">{$t('common.loading')}</p>
          </div>
        {:else if portalUsers.length === 0}
          <div style="padding: 3rem 1.5rem; text-align: center; border: 1px dashed var(--border); border-radius: var(--radius);">
            <p style="font-weight: 500; margin-bottom: 0.25rem;">{get(t)('admin.customers.portal.empty')}</p>
            <p style="font-size: 0.875rem; color: var(--text-muted);">{$t('admin.customers.portal.empty_hint') || 'This customer does not have a portal account yet.'}</p>
          </div>
        {:else}
          <div class="table-responsive" style="margin-top: 0.5rem;">
            <table style="width: 100%; border-collapse: collapse;">
              <thead>
                <tr style="border-bottom: 1px solid var(--border); text-align: left;">
                  <th style="padding: 0.75rem; font-weight: 600; color: var(--text-muted); font-size: 0.875rem;">{get(t)('admin.customers.portal.columns.user')}</th>
                  <th style="padding: 0.75rem; font-weight: 600; color: var(--text-muted); font-size: 0.875rem;">{$t('common.email')}</th>
                  <th style="padding: 0.75rem; font-weight: 600; color: var(--text-muted); font-size: 0.875rem;">{get(t)('admin.customers.portal.columns.added')}</th>
                  <th style="padding: 0.75rem; font-weight: 600; color: var(--text-muted); font-size: 0.875rem; text-align: right;">{$t('common.actions')}</th>
                </tr>
              </thead>
              <tbody>
                {#each portalUsers as user}
                  <tr style="border-bottom: 1px solid var(--border);">
                    <td style="padding: 0.75rem; font-size: 0.875rem; font-weight: 500;">{user.name}</td>
                    <td style="padding: 0.75rem; font-size: 0.875rem; color: var(--text-muted);">{user.email}</td>
                    <td style="padding: 0.75rem; font-size: 0.875rem; color: var(--text-muted);">{formatDate(user.created_at)}</td>
                    <td style="padding: 0.75rem; text-align: right;">
                      {#if canManageCustomers}
                        <div style="display: inline-flex; gap: 0.5rem; justify-content: flex-end;">
                          <button class="btn btn-secondary btn-sm" onclick={() => {
                            portalUserToReset = user;
                            generatedPasswordResult = null;
                            manualResetPassword = '';
                            manualResetPasswordConfirm = '';
                            showResetPasswordConfirm = true;
                          }} disabled={portalMutationBusy}>
                            <Icon name="key" size={14} style="margin-right: 4px;" />
                            {$t('common.reset_password')}
                          </button>
                          <button class="btn btn-danger btn-sm" type="button" title={$t('common.delete')} aria-label={$t('common.delete')} onclick={() => {
                            portalUserToRemove = user;
                            showRemovePortalUserConfirm = true;
                          }} disabled={portalMutationBusy}>
                            <Icon name="trash" size={14} />
                          </button>
                        </div>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    {:else if activeTab === 'locations' && canReadCustomerLocations}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.locations.title')}</h3>
            <p class="subtitle">{$t('admin.customers.locations.subtitle')}</p>
          </div>
          {#if canManageCustomerLocations}
            <button class="btn btn-primary" onclick={() => void openCreateLocation()}>
              <Icon name="plus" size={16} />
              {$t('admin.customers.locations.actions.add')}
            </button>
          {/if}
        </div>

        <Table
          columns={locColumns}
          data={locations}
          loading={loadingLocations}
          emptyText={$t('admin.customers.locations.empty')}
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
                  title={$t('common.refresh')}
                  onclick={() => void loadLocations({ force: true })}
                >
                  <Icon name="refresh-cw" size={16} />
                </button>
                {#if canManageCustomerLocations}
                  <button
                    class="btn-icon"
                    title={$t('common.edit')}
                    onclick={() => void openEditLocation(loc)}
                  >
                    <Icon name="edit-3" size={16} />
                  </button>
                  <button
                    class="btn-icon danger"
                    title={$t('common.delete')}
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
          loadingSubscriptions={loadingSubscriptions}
          metricCount={metricCount}
          subscriptionColumns={subscriptionColumns}
          subscriptions={subscriptions}
          subscriptionStatusLabel={subscriptionStatusLabel}
          getSubscriptionPolicySummary={getSubscriptionPolicySummary}
          getSubscriptionAccessState={getSubscriptionAccessState}
          formatCustomerPolicyDate={formatCustomerPolicyDate}
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
          onChangePackage={openChangePackage}
          {subscriptionMutationBusy}
        />
      {:else if activeDeferredTabLoading === 'subscriptions'}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading')}</p>
        </div>
      {/if}
    {:else if activeTab === 'billing'}
      {#if BillingTabComponent}
        <BillingTabComponent
          bind:billingFilter
          {onSelectBillingFilter}
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
          <p>{$t('common.loading')}</p>
        </div>
      {/if}
    {:else if activeTab === 'assets'}
      {#if AssetsTabComponent}
        <AssetsTabComponent assets={customerAssets} loading={loadingCustomerAssets} />
      {:else if activeDeferredTabLoading === 'assets'}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading')}</p>
        </div>
      {/if}
    {:else if activeTab === 'pppoe'}
      {#if PppoeTabComponent && pppoeHelperModule}
        <PppoeTabComponent
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
          getPppoeAccessState={getPppoeAccessState}
          timeAgo={timeAgo}
          canManagePppoe={$can('manage', 'pppoe')}
          {applyingPppoe}
          {deletingPppoe}
          {savingPppoe}
          onApplyPppoe={applyPppoe}
          onEditPppoe={openEditPppoe}
          onDeletePppoe={deletePppoe}
        />
      {:else if activeDeferredTabLoading === 'pppoe' || !pppoeHelperModule}
        <div class="card loading-card">
          <div class="spinner"></div>
          <p>{$t('common.loading')}</p>
        </div>
      {/if}
    {:else if activeTab === 'dhcp_static'}
      <div class="card section-card">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.tabs.dhcp_static')}</h3>
            <p class="muted">
              {$t('admin.customers.dhcp_static.subtitle')}
            </p>
          </div>
          <button class="btn ghost" onclick={() => loadDhcpStaticServices({ force: true })} disabled={loadingDhcpStatic}>
            {$t('common.refresh')}
          </button>
        </div>
        {#if loadingDhcpStatic}
          <p class="muted">{$t('common.loading')}</p>
        {:else if dhcpStaticServices.length === 0}
          <p class="muted">
            {$t('admin.customers.dhcp_static.empty')}
          </p>
        {:else}
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>{$t('admin.customers.dhcp_static.columns.server')}</th>
                  <th>{$t('admin.customers.dhcp_static.columns.mac')}</th>
                  <th>{$t('admin.customers.dhcp_static.columns.ip')}</th>
                  <th>{$t('admin.customers.dhcp_static.columns.lease')}</th>
                  <th>{$t('admin.customers.dhcp_static.columns.queue')}</th>
                </tr>
              </thead>
              <tbody>
                {#each dhcpStaticServices as row}
                  <tr>
                    <td data-label={$t('admin.customers.dhcp_static.columns.server')}>
                      {row.dhcp_server_name}
                    </td>
                    <td data-label={$t('admin.customers.dhcp_static.columns.mac')}>
                      {row.mac_address}
                    </td>
                    <td data-label={$t('admin.customers.dhcp_static.columns.ip')}>
                      {row.ip_address}
                    </td>
                    <td data-label={$t('admin.customers.dhcp_static.columns.lease')}>
                      {row.lease_present
                        ? $t('admin.customers.dhcp_static.sync.present') || 'Present'
                        : row.lease_last_error ||
                          ($t('admin.customers.dhcp_static.sync.missing') || 'Missing')}
                    </td>
                    <td data-label={$t('admin.customers.dhcp_static.columns.queue')}>
                      {row.queue_mode === 'none'
                        ? $t('admin.customers.dhcp_static.sync.none') || 'None'
                        : row.queue_present
                          ? $t('admin.customers.dhcp_static.sync.present') || 'Present'
                          : row.queue_last_error ||
                            ($t('admin.customers.dhcp_static.sync.missing') || 'Missing')}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
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
          <p>{$t('common.loading')}</p>
        </div>
      {/if}
    {/if}
    </div>
  {/if}
</div>

<Modal
  show={showWhatsAppCompose}
  title={$t('admin.customers.communication.title_whatsapp')}
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
            ? `${whatsappGatewayProvider || 'gateway'} ${$t('admin.customers.communication.gateway_ready')}`
            : whatsappGatewayReason || $t('admin.customers.communication.gateway_not_ready') || 'Gateway not ready'}
        </span>
      </div>
      <label>
        <span>{$t('admin.customers.communication.template')}</span>
        <select
          class="input"
          bind:value={selectedWhatsappTemplateId}
          onchange={(event) => applyWhatsAppTemplate(event.currentTarget.value)}
        >
          {#each whatsappTemplateOptions as template}
            <option value={template.id}>{template.name}</option>
          {/each}
          <option value="custom">{$t('admin.customers.communication.custom_message')}</option>
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.communication.message')}</span>
        <textarea class="input" rows="7" bind:value={whatsappMessage}></textarea>
      </label>
      <div class="compose-footnote">
        <span>{whatsappMessage.trim().length} {$t('admin.customers.communication.characters')}</span>
        {#if !whatsappGatewayReady}
          <span>{whatsappGatewayReason}</span>
        {/if}
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick={() => (showWhatsAppCompose = false)}>
          {$t('common.cancel')}
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
  title={$t('admin.customers.communication.title_email')}
  onclose={() => (showEmailCompose = false)}
>
  <div class="form">
    {#if customer}
      <div class="compose-target">
        <div>
          <strong>{customer.name}</strong>
          <span>{customer.email}</span>
        </div>
        <span class="status-pill is-active">{$t('admin.customers.communication.email_outbox')}</span>
      </div>
      <label>
        <span>{$t('admin.customers.communication.template')}</span>
        <select
          class="input"
          bind:value={selectedEmailTemplateId}
          onchange={(event) => applyEmailTemplate(event.currentTarget.value)}
        >
          {#each emailTemplateOptions as template}
            <option value={template.id}>{template.name}</option>
          {/each}
          <option value="custom">{$t('admin.customers.communication.custom_email')}</option>
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.communication.subject')}</span>
        <input class="input" bind:value={emailSubject} />
      </label>
      <label>
        <span>{$t('admin.customers.communication.body')}</span>
        <textarea class="input" rows="9" bind:value={emailBody}></textarea>
      </label>
      <div class="compose-footnote">
        <span>{emailBody.trim().length} {$t('admin.customers.communication.characters')}</span>
        <span>{$t('admin.customers.communication.queued_through_outbox')}</span>
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick={() => (showEmailCompose = false)}>
          {$t('common.cancel')}
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
    customerName={name}
    {deletingCustomer}
    onDeleteCustomer={doDeleteCustomer}
    bind:showDeleteLocation
    {deletingLocation}
    onDeleteLocation={doDeleteLocation}
  />
{/if}

<!-- Change Package Dialog (Pro-rata) -->
<Modal bind:show={showChangePackage} title="{$t('admin.customers.subscriptions.change_package.title')} — {changePackageSubscription?.package_name || 'Subscription'}" width="520px">
  {#if !changePackageResult}
    <div class="field-group">
      <label class="field-label">{$t('admin.customers.subscriptions.change_package.current_package')}</label>
      <div class="field-static">
        <strong>{changePackageSubscription?.package_name || changePackageSubscription?.package_id}</strong>
        <span class="subtle"> · {changePackageSubscription?.billing_cycle} · {formatMoney(changePackageSubscription?.price || 0)}</span>
      </div>
    </div>

    <div class="field-group">
      <label class="field-label" for="new-package">{$t('admin.customers.subscriptions.change_package.new_package')}</label>
      <select id="new-package" bind:value={changePackageNewId} class="field-input">
        <option value="">{$t('admin.customers.subscriptions.change_package.select_placeholder')}</option>
        {#each subscriptionPackages.filter((p) => p.id !== changePackageSubscription?.package_id && p.is_active) as pkg}
          <option value={pkg.id}>
            {pkg.name} — {formatMoney(changePackageSubscription?.billing_cycle === 'yearly' ? pkg.price_yearly : pkg.price_monthly)}/{changePackageSubscription?.billing_cycle === 'yearly' ? 'tahun' : 'bulan'}
          </option>
        {/each}
      </select>
    </div>

    {#if changePackageNewId}
      {@const selectedPkg = subscriptionPackages.find((p) => p.id === changePackageNewId)}
      {#if selectedPkg}
        <div class="pro-rata-preview">
          <div class="preview-row">
            <span>{$t('admin.customers.subscriptions.change_package.old_package_label')}</span>
            <span class="preview-value">-{formatMoney(changePackageSubscription?.price || 0)}</span>
          </div>
          <div class="preview-row">
            <span>{$t('admin.customers.subscriptions.change_package.new_package_label')}</span>
            <span class="preview-value">+{formatMoney(changePackageSubscription?.billing_cycle === 'yearly' ? selectedPkg.price_yearly : selectedPkg.price_monthly)}</span>
          </div>
          <div class="preview-divider"></div>
          <div class="preview-row preview-total">
            <span>{$t('admin.customers.subscriptions.change_package.pro_rata_estimate')}</span>
            <span class="preview-value">
              {#if (changePackageSubscription?.billing_cycle === 'yearly' ? selectedPkg.price_yearly : selectedPkg.price_monthly) > (changePackageSubscription?.price || 0)}
                {formatMoney((changePackageSubscription?.billing_cycle === 'yearly' ? selectedPkg.price_yearly : selectedPkg.price_monthly) - (changePackageSubscription?.price || 0))}
              {:else}
                Rp 0
              {/if}
            </span>
          </div>
          <p class="preview-note">{$t('admin.customers.subscriptions.change_package.pro_rata_note')}</p>
        </div>
      {/if}
    {/if}
  {:else}
    <div class="result-card success">
      <Icon name="check-circle" size={24} />
      <div>
        <strong>{$t('admin.customers.subscriptions.change_package.success_title')}</strong>
        <p>{$t('admin.customers.subscriptions.change_package.success_message')} <strong>{changePackageResult.old_package_name}</strong> {$t('admin.customers.subscriptions.change_package.success_message_to')} <strong>{changePackageResult.new_package_name}</strong></p>
      </div>
    </div>

    <div class="result-details">
      <div class="result-row">
        <span>{$t('admin.customers.subscriptions.change_package.old_price')}</span>
        <span>{formatMoney(changePackageResult.old_price)}/{changePackageResult.billing_cycle}</span>
      </div>
      <div class="result-row">
        <span>{$t('admin.customers.subscriptions.change_package.new_price')}</span>
        <span>{formatMoney(changePackageResult.new_price)}/{changePackageResult.billing_cycle}</span>
      </div>
      <div class="result-row">
        <span>{$t('admin.customers.subscriptions.change_package.credit')}</span>
        <span>-{formatMoney(changePackageResult.pro_rata_credit)}</span>
      </div>
      <div class="result-row">
        <span>{$t('admin.customers.subscriptions.change_package.charge')}</span>
        <span>+{formatMoney(changePackageResult.pro_rata_charge)}</span>
      </div>
      <div class="result-divider"></div>
      <div class="result-row result-total">
        <span>{$t('admin.customers.subscriptions.change_package.net_amount')}</span>
        <span>{formatMoney(changePackageResult.net_amount)}</span>
      </div>
      {#if changePackageResult.invoice_id}
        <div class="result-row">
          <span>{$t('common.invoice')}</span>
          <a href="/admin/invoices/{changePackageResult.invoice_id}" class="link">{$t('admin.customers.subscriptions.change_package.view_invoice')}</a>
        </div>
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    {#if !changePackageResult}
      <button class="btn btn-secondary" onclick={() => { showChangePackage = false; }}>{$t('admin.customers.subscriptions.change_package.cancel')}</button>
      <button
        class="btn btn-primary"
        onclick={submitChangePackage}
        disabled={!changePackageNewId || changePackageLoading}
      >
        {changePackageLoading ? ($t('admin.customers.subscriptions.change_package.processing') || 'Memproses...') : ($t('admin.customers.subscriptions.change_package.submit') || 'Ganti Paket')}
      </button>
    {:else}
      <button class="btn btn-primary" onclick={() => { showChangePackage = false; changePackageResult = null; }}>{$t('admin.customers.subscriptions.change_package.close')}</button>
    {/if}
  {/snippet}
</Modal>

<ConfirmDialog
  bind:show={showSubDeleteConfirm}
  title={$t('common.confirm_delete_title')}
  message={$t('common.confirm_delete')}
  confirmText={$t('common.delete')}
  cancelText={$t('common.cancel')}
  type="danger"
  onconfirm={confirmDeleteSubscription}
  oncancel={() => { subToDeleteId = null; }}
/>

<ConfirmDialog
  bind:show={showPppoeDeleteConfirm}
  title={$t('common.confirm_delete_title')}
  message={$t('admin.customers.pppoe.confirm_delete')}
  confirmText={$t('common.delete')}
  cancelText={$t('common.cancel')}
  type="danger"
  onconfirm={confirmDeletePppoe}
  oncancel={() => { pppoeToDeleteId = null; }}
/>

<Modal bind:show={showResetPasswordConfirm} title={get(t)('admin.customers.portal.reset_password.title')}>
  {#if generatedPasswordResult}
    <div style="padding: 1rem 0; text-align: center;">
      <p style="font-weight: 500; margin-bottom: 0.5rem;">{get(t)('admin.customers.portal.reset_password.success_generated')}</p>
      <div style="background: var(--bg-muted); padding: 0.75rem 1.25rem; border-radius: var(--radius); font-family: monospace; font-size: 1.25rem; font-weight: 600; color: var(--color-primary); letter-spacing: 0.05em; margin-bottom: 1rem; display: inline-block; border: 1px solid var(--border);">
        {generatedPasswordResult}
      </div>
      <p style="font-size: 0.875rem; color: var(--text-muted);">{get(t)('admin.customers.portal.reset_password.generated_notice')}</p>
    </div>
    <div style="display: flex; justify-content: flex-end; margin-top: 1.5rem;">
      <button class="btn btn-primary" onclick={() => { showResetPasswordConfirm = false; generatedPasswordResult = null; portalUserToReset = null; }}>{get(t)('admin.customers.portal.reset_password.done')}</button>
    </div>
  {:else}
    <div class="form" style="padding: 1rem 0;">
      <p style="font-size: 0.875rem; margin-bottom: 1.25rem; color: var(--text-muted);">
        {@html get(t)('admin.customers.portal.reset_password.message', { values: { email: portalUserToReset?.email || '' } })}
      </p>
      <label>
        <span style="font-weight: 500; font-size: 0.875rem; margin-bottom: 0.25rem; display: block;">{get(t)('admin.customers.portal.reset_password.new_password_label')}</span>
        <input class="input" type="password" autocomplete="new-password" bind:value={manualResetPassword} placeholder={get(t)('admin.customers.portal.reset_password.placeholder')} />
      </label>
      {#if manualResetPassword.trim()}
        <label style="margin-top: 1rem; display: block;">
          <span style="font-weight: 500; font-size: 0.875rem; margin-bottom: 0.25rem; display: block;">{get(t)('admin.customers.portal.reset_password.confirm_password_label')}</span>
          <input class="input" type="password" autocomplete="new-password" bind:value={manualResetPasswordConfirm} placeholder={get(t)('admin.customers.portal.reset_password.confirm_placeholder')} />
          {#if manualResetPasswordConfirm && manualResetPassword !== manualResetPasswordConfirm}
            <p style="color: var(--color-danger); font-size: 0.8rem; margin-top: 0.3rem;">{get(t)('admin.customers.portal.reset_password.mismatch')}</p>
          {/if}
        </label>
      {/if}
    </div>
    <div style="display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 1.5rem;">
      <button class="btn btn-secondary" onclick={() => { showResetPasswordConfirm = false; portalUserToReset = null; manualResetPassword = ''; manualResetPasswordConfirm = ''; }}>{$t('common.cancel')}</button>
      <button class="btn btn-warning" onclick={confirmResetPassword} disabled={resettingPassword || (manualResetPassword.trim() !== '' && manualResetPassword !== manualResetPasswordConfirm)}>
        {resettingPassword ? get(t)('admin.customers.portal.reset_password.processing') : get(t)('admin.customers.portal.reset_password.reset')}
      </button>
    </div>
  {/if}
</Modal>

<Modal bind:show={showAddPortalUser} title={get(t)('admin.customers.portal.new.title')}>
  <div class="form" style="padding: 1rem 0;">
    <p class="field-hint">{get(t)('admin.customers.portal.new.hint')}</p>
    <label>
      <span>{get(t)('admin.customers.portal.new.fields.email')}</span>
      <input class="input" type="email" bind:value={portalUserEmail} autocomplete="email" />
    </label>
    <label>
      <span>{get(t)('admin.customers.portal.new.fields.name')}</span>
      <input class="input" bind:value={portalUserName} autocomplete="name" />
    </label>
    <label>
      <span>{get(t)('admin.customers.portal.new.fields.password')}</span>
      <input class="input" type="password" bind:value={portalUserPassword} autocomplete="new-password" />
    </label>
    <label>
      <span>{get(t)('admin.customers.portal.reset_password.confirm_password_label')}</span>
      <input class="input" type="password" bind:value={portalUserPasswordConfirm} autocomplete="new-password" />
    </label>
    {#if portalUserPasswordConfirm && portalUserPassword !== portalUserPasswordConfirm}
      <p class="field-error">{get(t)('admin.customers.portal.reset_password.mismatch')}</p>
    {/if}
    <div class="modal-actions">
      <button class="btn btn-secondary" type="button" onclick={() => (showAddPortalUser = false)} disabled={addingPortalUser}>
        {$t('common.cancel')}
      </button>
      <button
        class="btn btn-primary"
        type="button"
        onclick={addPortalUser}
        disabled={addingPortalUser || !portalUserEmail.trim() || !portalUserName.trim() || portalUserPassword.length < 6 || portalUserPassword !== portalUserPasswordConfirm}
      >
        {addingPortalUser ? $t('common.saving') : $t('common.create')}
      </button>
    </div>
  </div>
</Modal>

<ConfirmDialog
  bind:show={showRemovePortalUserConfirm}
  title={get(t)('admin.customers.portal.remove.title')}
  message={get(t)('admin.customers.portal.remove.message')}
  confirmText={$t('common.delete')}
  cancelText={$t('common.cancel')}
  type="danger"
  loading={removingPortalUser}
  confirmationKeyword="DELETE"
  onconfirm={confirmRemovePortalUser}
  oncancel={() => { portalUserToRemove = null; }}
/>

<style>
  .page-content {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  .customer-hero {
    margin-bottom: 1rem;
    padding: 0.9rem 0.95rem;
    background: var(--bg-surface);
  }

  .hero-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .breadcrumbs {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 0.8rem;
  }

  .breadcrumbs button {
    border: 0;
    padding: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .breadcrumbs button:hover {
    color: var(--text-primary);
  }

  .breadcrumbs b {
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hero-main {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    min-width: 0;
  }

  .avatar {
    width: 48px;
    height: 48px;
    border-radius: 12px;
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
    font-size: 1.45rem;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
  }

  .hero-badges {
    margin-top: 0.35rem;
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
    padding: 0.24rem 0.55rem;
    font-size: 0.76rem;
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
    gap: 0.55rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    min-height: 42px;
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

  .overview-form {
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
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

  .section-card {
    padding: 1.1rem;
    background: var(--bg-surface);
  }

  .table-wrap {
    overflow: auto;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 14%);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
  }

  .table-wrap table {
    width: 100%;
    border-collapse: collapse;
  }

  .table-wrap th,
  .table-wrap td {
    padding: 0.85rem 1rem;
    text-align: left;
    border-bottom: 1px solid color-mix(in srgb, var(--border-color), transparent 22%);
    vertical-align: top;
  }

  .table-wrap th {
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .table-wrap tbody tr:last-child td {
    border-bottom: none;
  }

  .muted {
    color: var(--text-secondary);
  }

  .ghost {
    background: var(--bg-surface);
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
    width: 38px;
    height: 38px;
    padding: 0;
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

  .field-error {
    margin: 0;
    color: var(--color-danger);
    font-size: 0.8rem;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
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
    .back-button {
      align-self: flex-start;
    }
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
    .hero-actions {
      width: 100%;
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
    .table-wrap {
      overflow: visible;
      border: none;
      background: transparent;
    }
    .table-wrap table,
    .table-wrap tbody,
    .table-wrap tr,
    .table-wrap td {
      display: block;
      width: 100%;
    }
    .table-wrap thead {
      display: none;
    }
    .table-wrap tr {
      margin-bottom: 0.8rem;
      border: 1px solid color-mix(in srgb, var(--border-color), transparent 14%);
      border-radius: 14px;
      background: color-mix(in srgb, var(--bg-surface), transparent 4%);
      overflow: hidden;
    }
    .table-wrap td {
      display: flex;
      justify-content: space-between;
      gap: 1rem;
      padding: 0.8rem 0.9rem;
      text-align: right;
    }
    .table-wrap td::before {
      content: attr(data-label);
      color: var(--text-secondary);
      font-size: 0.78rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      text-align: left;
    }
    .table-wrap td:last-child {
      border-bottom: none;
    }
  }

  /* ── Change Package Modal ── */

  .field-group {
    display: grid;
    gap: 0.35rem;
    margin-bottom: 1rem;
  }

  .field-label {
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .field-static {
    padding: 0.65rem 0.75rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 6%);
    font-size: 0.9rem;
  }

  .field-static .subtle {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .field-input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.65rem 0.75rem;
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .field-input:focus {
    border-color: color-mix(in srgb, var(--color-primary) 52%, var(--border-color));
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .pro-rata-preview {
    margin-top: 0.75rem;
    padding: 0.85rem 0.9rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
  }

  .preview-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0;
    font-size: 0.88rem;
    color: var(--text-secondary);
  }

  .preview-value {
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .preview-divider {
    border-top: 1px solid color-mix(in srgb, var(--border-color), transparent 30%);
    margin: 0.5rem 0;
  }

  .preview-total {
    font-weight: 700;
    color: var(--text-primary);
  }

  .preview-total .preview-value {
    font-size: 1rem;
    color: var(--color-primary);
  }

  .preview-note {
    margin: 0.5rem 0 0;
    font-size: 0.78rem;
    color: var(--text-secondary);
    line-height: 1.4;
    font-style: italic;
  }

  .result-card {
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
    padding: 1rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--color-success) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-success) 8%, transparent);
    margin-bottom: 1rem;
  }

  .result-card.success {
    color: var(--color-success);
  }

  .result-card strong {
    color: var(--text-primary);
  }

  .result-card p {
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
    color: var(--text-secondary);
  }

  .result-details {
    padding: 0.85rem 0.9rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
  }

  .result-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0;
    font-size: 0.88rem;
    color: var(--text-secondary);
  }

  .result-row span:last-child {
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .result-divider {
    border-top: 1px solid color-mix(in srgb, var(--border-color), transparent 30%);
    margin: 0.5rem 0;
  }

  .result-total {
    font-weight: 700;
    color: var(--text-primary);
  }

  .result-total span:last-child {
    font-size: 1rem;
    color: var(--color-primary);
  }

  .result-row .link {
    color: var(--color-primary);
    text-decoration: none;
    font-weight: 600;
  }

  .result-row .link:hover {
    text-decoration: underline;
  }

  @media (max-width: 560px) {
    .hero-badges,
    .row-actions,
    .actions {
      flex-wrap: wrap;
    }
    .actions .btn,
    .section-head .btn {
      width: 100%;
    }
  }
</style>
