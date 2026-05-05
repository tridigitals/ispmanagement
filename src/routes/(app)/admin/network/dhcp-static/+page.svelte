<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { can, tenant, user } from '$lib/stores/auth';
  import {
    api,
    type CustomerSubscriptionView,
    type DhcpStaticServicePublic,
    type IspPackage,
  } from '$lib/api/client';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { buildDhcpStaticQueueRateLimitPresets } from '$lib/utils/dhcpStaticQueuePresets';
  import type { MikrotikDhcpServerOption } from '$lib/api/mikrotik';
  import {
    formatDhcpStaticMacAddressInput,
    normalizeDhcpStaticMacAddress,
    validateDhcpStaticIpv4Address,
    validateDhcpStaticQueueRateLimit,
  } from '$lib/utils/dhcpStaticValidation';

  type RouterRow = { id: string; name: string };
  type CustomerRow = { id: string; name: string };
  type LocationRow = { id: string; label: string };
  type TranslationValues = Record<string, string | number | boolean | Date | null | undefined>;

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const tr = (key: string, fallback: string, values?: TranslationValues) =>
    get(t)(key, values ? { values } : undefined) || fallback;

  let loading = $state(true);
  let rows = $state<DhcpStaticServicePublic[]>([]);
  let search = $state('');
  let routerId = $state('');
  let routerOptions = $state<Array<{ label: string; value: string }>>([]);
  let routers = $state<RouterRow[]>([]);
  let customers = $state<CustomerRow[]>([]);
  let packages = $state<IspPackage[]>([]);
  let locations = $state<LocationRow[]>([]);
  let customerSubscriptions = $state<CustomerSubscriptionView[]>([]);
  let routerDhcpServers = $state<MikrotikDhcpServerOption[]>([]);
  let loadingFormOptions = $state(false);
  let loadingRouterDhcpServers = $state(false);
  let saving = $state(false);
  let showCreate = $state(false);
  let showEdit = $state(false);
  let editRow = $state<DhcpStaticServicePublic | null>(null);
  let dhcpServerLookupToken = 0;

  let formCustomerId = $state('');
  let formLocationId = $state('');
  let formSubscriptionId = $state('');
  let formPackageId = $state('');
  let formRouterId = $state('');
  let formDhcpServerName = $state('');
  let formMacAddress = $state('');
  let formIpAddress = $state('');
  let formComment = $state('');
  let formDisabled = $state(false);
  let formQueueMode = $state<'none' | 'simple_queue'>('none');
  let formQueueRateLimit = $state('');
  let formRouterIdError = $state<string | null>(null);
  let formDhcpServerNameError = $state<string | null>(null);
  let formMacAddressError = $state<string | null>(null);
  let formIpAddressError = $state<string | null>(null);
  let formQueueRateLimitError = $state<string | null>(null);

  const dhcpStaticPackageIds = $derived.by(
    () =>
      new Set(
        packages
          .filter(
            (pkg) =>
              pkg.service_type === 'internet_pppoe' && pkg.provisioning_type === 'dhcp_static',
          )
          .map((pkg) => pkg.id),
      ),
  );

  const packageNameById = $derived.by(() => {
    const map = new Map<string, string>();
    for (const pkg of packages) {
      map.set(pkg.id, pkg.name);
    }
    return map;
  });

  const customerNameById = $derived.by(() => {
    const map = new Map<string, string>();
    for (const customer of customers) {
      map.set(customer.id, customer.name);
    }
    return map;
  });

  const locationNameById = $derived.by(() => {
    const map = new Map<string, string>();
    for (const location of locations) {
      map.set(location.id, location.label);
    }
    return map;
  });

  const selectedSubscription = $derived.by(
    () => customerSubscriptions.find((subscription) => subscription.id === formSubscriptionId) || null,
  );
  const selectedLocation = $derived.by(
    () => locations.find((location) => location.id === formLocationId) || null,
  );
  const selectedPackage = $derived.by(
    () => packages.find((pkg) => pkg.id === formPackageId) || null,
  );
  const queueRateLimitPresets = $derived.by(() =>
    buildDhcpStaticQueueRateLimitPresets({
      name: selectedPackage?.name || selectedSubscription?.package_name || null,
      description: selectedPackage?.description || null,
      features: selectedPackage?.features || [],
    }),
  );

  const visibleSubscriptions = $derived.by(() =>
    customerSubscriptions.filter(
      (subscription) =>
        dhcpStaticPackageIds.has(subscription.package_id) &&
        (!formLocationId || subscription.location_id === formLocationId) &&
        subscription.status !== 'cancelled',
    ),
  );
  const existingCustomerDhcpServices = $derived.by(() =>
    rows.filter(
      (row) =>
        row.customer_id === formCustomerId &&
        (!formLocationId || row.location_id === formLocationId),
    ),
  );

  const selectedRouterName = $derived.by(
    () => routers.find((router) => router.id === formRouterId)?.name || '-',
  );
  const selectedRouterDhcpServer = $derived.by(
    () => routerDhcpServers.find((server) => server.name === formDhcpServerName) || null,
  );
  const routerDhcpServerOptions = $derived.by(() => {
    const options = routerDhcpServers.map((server) => ({
      value: server.name,
      label: server.interface ? `${server.name} • ${server.interface}` : server.name,
    }));
    if (
      formDhcpServerName &&
      !options.some((option) => option.value === formDhcpServerName)
    ) {
      options.unshift({
        value: formDhcpServerName,
        label: `${formDhcpServerName} (${tr(
          'admin.network.dhcp_static.fields.current_dhcp_server_unavailable',
          'current value',
        )})`,
      });
    }
    return options;
  });
  const selectedPackageName = $derived.by(() => packageNameById.get(formPackageId) || '-');
  const selectedLocationName = $derived.by(() => locationNameById.get(formLocationId) || '-');

  const filteredRows = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return rows.filter((row) => {
      if (routerId && row.router_id !== routerId) return false;
      if (!q) return true;
      return [
        row.mac_address,
        row.ip_address,
        row.dhcp_server_name,
        row.comment || '',
        customerNameById.get(row.customer_id) || '',
        packageNameById.get(row.package_id) || '',
      ]
        .join(' ')
        .toLowerCase()
        .includes(q);
    });
  });

  const stats = $derived.by(() => ({
    total: rows.length,
    leaseReady: rows.filter((row) => row.lease_present).length,
    queueIssues: rows.filter((row) => row.queue_mode !== 'none' && !row.queue_present).length,
    disabled: rows.filter((row) => row.disabled).length,
  }));

  onMount(() => {
    if (!$can('read', 'dhcp_static') && !$can('manage', 'dhcp_static')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });

  async function load() {
    loading = true;
    try {
      const [services, routersResult, customersResult, packagesResult] = await Promise.all([
        api.dhcpStatic.services.list({ page: 1, per_page: 200 }),
        api.mikrotik.routers.list().catch(() => [] as any[]),
        api.customers.list({ page: 1, perPage: 1000 }).catch(() => ({ data: [] as any[] })),
        api.ispPackages.packages
          .list({ page: 1, per_page: 500, q: '' })
          .catch(() => ({ data: [] as IspPackage[] })),
      ]);
      rows = services.data || [];
      routers = (routersResult || []).map((router: any) => ({ id: router.id, name: router.name }));
      routerOptions = routers.map((router) => ({ label: router.name, value: router.id }));
      customers = ((customersResult as any)?.data || []).map((customer: any) => ({
        id: customer.id,
        name: customer.name,
      }));
      packages = ((packagesResult as any)?.data || []) as IspPackage[];
    } catch (e: any) {
      toast.error(
        e?.message || tr('admin.network.dhcp_static.toasts.load_failed', 'Failed to load DHCP static services'),
      );
    } finally {
      loading = false;
    }
  }

  async function applyRow(row: DhcpStaticServicePublic) {
    try {
      await api.dhcpStatic.services.apply(row.id);
      toast.success(tr('admin.network.dhcp_static.toasts.applied', 'DHCP static lease applied'));
      await load();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr('admin.network.dhcp_static.toasts.apply_failed', 'Failed to apply DHCP static lease'),
      );
    }
  }

  async function reconcileRouter() {
    if (!routerId) {
      toast.error(
        tr('admin.network.dhcp_static.validation.select_router', 'Select a router first'),
      );
      return;
    }
    try {
      await api.dhcpStatic.services.reconcileRouter(routerId);
      toast.success(
        tr('admin.network.dhcp_static.toasts.reconciled', 'DHCP static services reconciled'),
      );
      await load();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr(
            'admin.network.dhcp_static.toasts.reconcile_failed',
            'Failed to reconcile DHCP static services',
          ),
      );
    }
  }

  async function deleteRow(row: DhcpStaticServicePublic) {
    if (
      !confirm(
        tr(
          'admin.network.dhcp_static.confirm_delete',
          'Delete this DHCP static service?',
        ),
      )
    )
      return;
    try {
      await api.dhcpStatic.services.delete(row.id);
      toast.success(tr('admin.network.dhcp_static.toasts.deleted', 'Deleted'));
      await load();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr(
            'admin.network.dhcp_static.toasts.delete_failed',
            'Failed to delete DHCP static service',
          ),
      );
    }
  }

  function resetForm() {
    formCustomerId = '';
    formLocationId = '';
    formSubscriptionId = '';
    formPackageId = '';
    formRouterId = '';
    formDhcpServerName = '';
    formMacAddress = '';
    formIpAddress = '';
    formComment = '';
    formDisabled = false;
    formQueueMode = 'none';
    formQueueRateLimit = '';
    formRouterIdError = null;
    formDhcpServerNameError = null;
    formMacAddressError = null;
    formIpAddressError = null;
    formQueueRateLimitError = null;
    locations = [];
    customerSubscriptions = [];
    routerDhcpServers = [];
    loadingRouterDhcpServers = false;
    editRow = null;
  }

  async function loadCustomerScopedOptions(customerId: string) {
    if (!customerId) {
      locations = [];
      customerSubscriptions = [];
      return;
    }

    loadingFormOptions = true;
    try {
      const [locationsResult, subscriptionsResult] = await Promise.all([
        api.customers.locations.list(customerId).catch(() => [] as any[]),
        api.customers.subscriptions
          .list(customerId, { page: 1, per_page: 200 })
          .catch(() => ({ data: [] as CustomerSubscriptionView[] })),
      ]);

      locations = (locationsResult || []).map((location: any) => ({
        id: location.id,
        label: location.label,
      }));
      customerSubscriptions = ((subscriptionsResult as any)?.data || []) as CustomerSubscriptionView[];
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr(
            'admin.network.dhcp_static.toasts.load_customer_scope_failed',
            'Failed to load customer DHCP options',
          ),
      );
      locations = [];
      customerSubscriptions = [];
    } finally {
      loadingFormOptions = false;
    }
  }

  function syncFormFromSubscription(subscriptionId: string) {
    const subscription = customerSubscriptions.find((item) => item.id === subscriptionId);
    if (!subscription) return;

    formLocationId = subscription.location_id;
    formPackageId = subscription.package_id;
    if (subscription.router_id) {
      void handleRouterChange(subscription.router_id, { preserveServerName: true });
    }
  }

  async function loadRouterDhcpServers(
    routerId: string,
    options?: { preserveServerName?: boolean },
  ) {
    const preserveServerName = options?.preserveServerName ?? false;
    const requestToken = ++dhcpServerLookupToken;

    if (!routerId) {
      routerDhcpServers = [];
      loadingRouterDhcpServers = false;
      if (!preserveServerName) {
        formDhcpServerName = '';
      }
      return;
    }

    loadingRouterDhcpServers = true;
    try {
      const servers = await api.mikrotik.routers.dhcpServers(routerId);
      if (requestToken !== dhcpServerLookupToken) return;

      routerDhcpServers = (servers || []).filter((server) => !server.disabled);
      const matchingServer = routerDhcpServers.find((server) => server.name === formDhcpServerName);
      if (!matchingServer) {
        if (routerDhcpServers.length === 1) {
          formDhcpServerName = routerDhcpServers[0].name;
        } else if (!preserveServerName) {
          formDhcpServerName = '';
        }
      }
    } catch (e: any) {
      if (requestToken !== dhcpServerLookupToken) return;
      routerDhcpServers = [];
      if (!preserveServerName) {
        formDhcpServerName = '';
      }
      toast.error(
        e?.message ||
          tr(
            'admin.network.dhcp_static.toasts.load_router_dhcp_servers_failed',
            'Failed to load DHCP servers from router',
          ),
      );
    } finally {
      if (requestToken === dhcpServerLookupToken) {
        loadingRouterDhcpServers = false;
      }
    }
  }

  async function handleRouterChange(
    routerId: string,
    options?: { preserveServerName?: boolean },
  ) {
    formRouterId = routerId;
    await loadRouterDhcpServers(routerId, options);
  }

  async function openCreate() {
    if (!$can('manage', 'dhcp_static')) {
      toast.error($t('common.forbidden') || 'Forbidden');
      return;
    }

    resetForm();
    showCreate = true;
  }

  async function openEdit(row: DhcpStaticServicePublic) {
    if (!$can('manage', 'dhcp_static')) {
      toast.error($t('common.forbidden') || 'Forbidden');
      return;
    }

    resetForm();
    editRow = row;
    formCustomerId = row.customer_id;
    formLocationId = row.location_id;
    formSubscriptionId = row.subscription_id;
    formPackageId = row.package_id;
    formRouterId = row.router_id;
    formDhcpServerName = row.dhcp_server_name;
    formMacAddress = row.mac_address;
    formIpAddress = row.ip_address;
    formComment = row.comment || '';
    formDisabled = Boolean(row.disabled);
    formQueueMode = row.queue_mode === 'simple_queue' ? 'simple_queue' : 'none';
    formQueueRateLimit = row.queue_rate_limit || '';
    await loadCustomerScopedOptions(row.customer_id);
    await loadRouterDhcpServers(row.router_id, { preserveServerName: true });
    showEdit = true;
  }

  async function handleCustomerChange(customerId: string) {
    formCustomerId = customerId;
    formLocationId = '';
    formSubscriptionId = '';
    formPackageId = '';
    await handleRouterChange('', { preserveServerName: false });
    await loadCustomerScopedOptions(customerId);
    if (locations.length === 1) {
      formLocationId = locations[0].id;
    }
  }

  function handleLocationChange(locationId: string) {
    formLocationId = locationId;
    const currentSubscriptionStillMatches = customerSubscriptions.some(
      (subscription) =>
        subscription.id === formSubscriptionId && subscription.location_id === locationId,
    );
    if (!currentSubscriptionStillMatches) {
      formSubscriptionId = '';
      formPackageId = '';
      void handleRouterChange('', { preserveServerName: false });
    }
  }

  function handleSubscriptionChange(subscriptionId: string) {
    formSubscriptionId = subscriptionId;
    syncFormFromSubscription(subscriptionId);
    if (formQueueMode === 'simple_queue' && !formQueueRateLimit.trim() && queueRateLimitPresets[0]) {
      formQueueRateLimit = queueRateLimitPresets[0];
    }
  }

  function validateDhcpStaticFormInput(): {
    macAddress: string;
    ipAddress: string;
    queueRateLimit: string | null;
  } | null {
    if (!formRouterId) {
      formRouterIdError = tr(
        'admin.network.dhcp_static.validation.required_router',
        'Select router',
      );
      return null;
    }
    formRouterIdError = null;

    if (!formDhcpServerName.trim()) {
      formDhcpServerNameError = tr(
        'admin.network.dhcp_static.validation.required_dhcp_server_name',
        'Enter DHCP server name',
      );
      return null;
    }
    formDhcpServerNameError = null;

    const normalizedMac = normalizeDhcpStaticMacAddress(formMacAddress);
    if (normalizedMac.error || !normalizedMac.value) {
      formMacAddressError = tr(
        'admin.network.dhcp_static.validation.invalid_mac',
        'Enter a valid MAC address',
      );
      return null;
    }
    formMacAddressError = null;

    if (validateDhcpStaticIpv4Address(formIpAddress)) {
      formIpAddressError = tr(
        'admin.network.dhcp_static.validation.invalid_ip',
        'Enter a valid IPv4 address',
      );
      return null;
    }
    formIpAddressError = null;

    if (formQueueMode === 'simple_queue') {
      const queueRateLimit = formQueueRateLimit.trim();
      if (!queueRateLimit) {
        formQueueRateLimitError = tr(
          'admin.network.dhcp_static.validation.queue_rate_required',
          'Queue rate limit is required when Simple Queue is enabled',
        );
        return null;
      }
      if (validateDhcpStaticQueueRateLimit(queueRateLimit)) {
        formQueueRateLimitError = tr(
          'admin.network.dhcp_static.validation.invalid_queue_rate',
          'Queue rate limit must use format like 20M/20M',
        );
        return null;
      }
      formQueueRateLimitError = null;
      return {
        macAddress: normalizedMac.value,
        ipAddress: formIpAddress.trim(),
        queueRateLimit,
      };
    }

    formQueueRateLimitError = null;

    return {
      macAddress: normalizedMac.value,
      ipAddress: formIpAddress.trim(),
      queueRateLimit: null,
    };
  }

  async function submitCreate() {
    if (saving) return;
    if (
      !formCustomerId ||
      !formLocationId ||
      !formSubscriptionId ||
      !formPackageId ||
      !formRouterId ||
      !formDhcpServerName.trim() ||
      !formMacAddress.trim() ||
      !formIpAddress.trim()
    ) {
      toast.error(
        tr(
          'admin.network.dhcp_static.validation.required_fields',
          'Complete all required DHCP static fields first',
        ),
      );
      return;
    }

    const validated = validateDhcpStaticFormInput();
    if (!validated) return;

    formMacAddress = validated.macAddress;
    formIpAddress = validated.ipAddress;
    formQueueRateLimit = validated.queueRateLimit || '';

    saving = true;
    try {
      await api.dhcpStatic.services.create({
        subscription_id: formSubscriptionId,
        router_id: formRouterId,
        customer_id: formCustomerId,
        location_id: formLocationId,
        package_id: formPackageId,
        dhcp_server_name: formDhcpServerName.trim(),
        mac_address: validated.macAddress,
        ip_address: validated.ipAddress,
        comment: formComment.trim() || null,
        disabled: formDisabled,
        queue_mode: formQueueMode,
        queue_rate_limit: validated.queueRateLimit,
      });
      toast.success(
        tr('admin.network.dhcp_static.toasts.created', 'DHCP static service created'),
      );
      showCreate = false;
      await load();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr(
            'admin.network.dhcp_static.toasts.create_failed',
            'Failed to create DHCP static service',
          ),
      );
    } finally {
      saving = false;
    }
  }

  async function submitEdit() {
    if (saving || !editRow) return;
    if (
      !formRouterId ||
      !formDhcpServerName.trim() ||
      !formMacAddress.trim() ||
      !formIpAddress.trim()
    ) {
      toast.error(
        tr(
          'admin.network.dhcp_static.validation.required_fields',
          'Complete all required DHCP static fields first',
        ),
      );
      return;
    }

    const validated = validateDhcpStaticFormInput();
    if (!validated) return;

    formMacAddress = validated.macAddress;
    formIpAddress = validated.ipAddress;
    formQueueRateLimit = validated.queueRateLimit || '';

    saving = true;
    try {
      await api.dhcpStatic.services.update(editRow.id, {
        router_id: formRouterId,
        dhcp_server_name: formDhcpServerName.trim(),
        mac_address: validated.macAddress,
        ip_address: validated.ipAddress,
        comment: formComment.trim() || null,
        disabled: formDisabled,
        queue_mode: formQueueMode,
        queue_rate_limit: validated.queueRateLimit,
      });
      toast.success(
        tr('admin.network.dhcp_static.toasts.updated', 'DHCP static service updated'),
      );
      showEdit = false;
      await load();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr(
            'admin.network.dhcp_static.toasts.update_failed',
            'Failed to update DHCP static service',
          ),
      );
    } finally {
      saving = false;
    }
  }

  $effect(() => {
    if (!formSubscriptionId && visibleSubscriptions.length === 1) {
      const onlySubscription = visibleSubscriptions[0];
      formSubscriptionId = onlySubscription.id;
      syncFormFromSubscription(onlySubscription.id);
    }
  });

  $effect(() => {
    if (formRouterIdError && formRouterId) {
      formRouterIdError = null;
    }
  });

  $effect(() => {
    if (formDhcpServerNameError && formDhcpServerName.trim()) {
      formDhcpServerNameError = null;
    }
  });

  $effect(() => {
    if (formMacAddressError && normalizeDhcpStaticMacAddress(formMacAddress).value) {
      formMacAddressError = null;
    }
  });

  $effect(() => {
    if (formIpAddressError && !validateDhcpStaticIpv4Address(formIpAddress)) {
      formIpAddressError = null;
    }
  });

  $effect(() => {
    if (formQueueMode !== 'simple_queue') {
      formQueueRateLimitError = null;
      return;
    }
    if (!formQueueRateLimit.trim() && queueRateLimitPresets[0]) {
      formQueueRateLimit = queueRateLimitPresets[0];
    }
    if (
      formQueueRateLimitError &&
      formQueueRateLimit.trim() &&
      !validateDhcpStaticQueueRateLimit(formQueueRateLimit)
    ) {
      formQueueRateLimitError = null;
    }
  });
</script>

<div class="page-content">
  <NetworkPageHeader
    title={$t('admin.network.dhcp_static.title') || 'DHCP Static'}
    subtitle={$t('admin.network.dhcp_static.subtitle') || 'Monitor and re-apply static DHCP leases and optional simple queues.'}
  />

  <section class="stats-grid">
    <article class="stat-card"><span>{$t('admin.network.dhcp_static.stats.total') || 'Total'}</span><strong>{stats.total}</strong></article>
    <article class="stat-card"><span>{$t('admin.network.dhcp_static.stats.lease_ready') || 'Lease Ready'}</span><strong>{stats.leaseReady}</strong></article>
    <article class="stat-card"><span>{$t('admin.network.dhcp_static.stats.queue_issues') || 'Queue Issues'}</span><strong>{stats.queueIssues}</strong></article>
    <article class="stat-card"><span>{$t('admin.network.dhcp_static.stats.disabled') || 'Disabled'}</span><strong>{stats.disabled}</strong></article>
  </section>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <label>
        <span>{$t('common.search') || 'Search'}</span>
        <input
          class="input"
          bind:value={search}
          placeholder={$t('admin.network.dhcp_static.filters.search_placeholder') ||
            'MAC, IP, server, comment'}
        />
      </label>
      <label>
        <span>{$t('admin.network.dhcp_static.filters.router') || 'Router'}</span>
        <select class="input" bind:value={routerId}>
          <option value="">{$t('admin.network.dhcp_static.filters.all_routers') || 'All routers'}</option>
          {#each routerOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <div class="filter-actions">
        <button class="btn ghost" onclick={load} disabled={loading}>
          {$t('common.refresh') || 'Refresh'}
        </button>
        <button class="btn ghost" onclick={reconcileRouter} disabled={loading || !routerId || !$can('manage', 'dhcp_static')}>
          {$t('admin.network.dhcp_static.actions.reconcile') || 'Reconcile Router'}
        </button>
        <button class="btn" onclick={openCreate} disabled={!$can('manage', 'dhcp_static')}>
          {$t('admin.network.dhcp_static.actions.create') || 'Create DHCP Static'}
        </button>
      </div>
    </NetworkFilterPanel>
  </div>

  <section class="table-card">
    {#if loading}
      <div class="empty-state">{$t('common.loading') || 'Loading...'}</div>
    {:else if filteredRows.length === 0}
      <div class="empty-state">{$t('admin.network.dhcp_static.empty') || 'No DHCP static services.'}</div>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{$t('admin.network.dhcp_static.columns.server') || 'Server'}</th>
              <th>{$t('admin.network.dhcp_static.columns.mac') || 'MAC'}</th>
              <th>{$t('admin.network.dhcp_static.columns.ip') || 'IP'}</th>
              <th>{$t('admin.network.dhcp_static.columns.lease') || 'Lease'}</th>
              <th>{$t('admin.network.dhcp_static.columns.queue') || 'Queue'}</th>
              <th>{$t('admin.network.dhcp_static.columns.actions') || 'Actions'}</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredRows as row}
              <tr>
                <td>
                  <div class="stack">
                    <strong>{row.dhcp_server_name}</strong>
                    <span class="muted">
                      {customerNameById.get(row.customer_id) || row.customer_id}
                      •
                      {packageNameById.get(row.package_id) || row.package_id}
                    </span>
                  </div>
                </td>
                <td>{row.mac_address}</td>
                <td>{row.ip_address}</td>
                <td><span class:ok={row.lease_present} class="status-pill">{row.lease_present ? ($t('admin.network.dhcp_static.sync.present') || 'Present') : ($t('admin.network.dhcp_static.sync.missing') || 'Missing')}</span></td>
                <td><span class:ok={row.queue_mode === 'none' || row.queue_present} class="status-pill">{row.queue_mode === 'none' ? ($t('admin.network.dhcp_static.sync.none') || 'None') : row.queue_present ? ($t('admin.network.dhcp_static.sync.present') || 'Present') : ($t('admin.network.dhcp_static.sync.missing') || 'Missing')}</span></td>
                <td>
                  <div class="actions">
                    <button class="btn ghost" onclick={() => applyRow(row)} disabled={!$can('manage', 'dhcp_static')}>{$t('admin.network.dhcp_static.actions.apply') || 'Apply'}</button>
                    <button class="btn ghost" onclick={() => openEdit(row)} disabled={!$can('manage', 'dhcp_static')}>{$t('common.edit') || 'Edit'}</button>
                    <button class="btn ghost" onclick={() => goto(`${tenantPrefix}/admin/network/installations`)} disabled={!$can('manage', 'work_orders')}>{$t('admin.network.dhcp_static.actions.install') || 'Install'}</button>
                    <button class="btn danger" onclick={() => deleteRow(row)} disabled={!$can('manage', 'dhcp_static')}>{$t('common.delete') || 'Delete'}</button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>

<Modal
  show={showCreate || showEdit}
  title={showEdit
    ? $t('admin.network.dhcp_static.modal.edit_title') || 'Edit DHCP Static'
    : $t('admin.network.dhcp_static.modal.create_title') || 'Create DHCP Static'}
  width="720px"
  onclose={() => {
    showCreate = false;
    showEdit = false;
  }}
>
  <div class="modal-form">
    <section class="form-section">
      <div class="section-head">
        <div>
          <h3>{$t('admin.network.dhcp_static.fields.subscription_context') || 'Subscription context'}</h3>
          <p>
            {$t('admin.network.dhcp_static.modal.context_help') ||
              'Choose the customer and subscription first, then continue with lease provisioning.'}
          </p>
        </div>
      </div>

      <div class="form-grid">
        <label>
          <span>{$t('common.customer') || 'Customer'}</span>
          <select
            class="input"
            bind:value={formCustomerId}
            onchange={(event) => handleCustomerChange((event.currentTarget as HTMLSelectElement).value)}
            disabled={saving || loadingFormOptions || showEdit}
          >
            <option value="">{$t('admin.network.dhcp_static.fields.select_customer') || 'Select customer'}</option>
            {#each customers as customer}
              <option value={customer.id}>{customer.name}</option>
            {/each}
          </select>
        </label>

        <label>
          <span>{$t('common.location') || 'Location'}</span>
          <select
            class="input"
            bind:value={formLocationId}
            onchange={(event) =>
              handleLocationChange((event.currentTarget as HTMLSelectElement).value)}
            disabled={saving || loadingFormOptions || !formCustomerId}
          >
            <option value="">
              {$t('admin.network.dhcp_static.fields.select_location') || 'Select location'}
            </option>
            {#each locations as location}
              <option value={location.id}>{location.label}</option>
            {/each}
          </select>
        </label>

        <label>
          <span>{$t('admin.network.dhcp_static.fields.subscription') || 'Subscription'}</span>
          <select
            class="input"
            bind:value={formSubscriptionId}
            onchange={(event) =>
              handleSubscriptionChange((event.currentTarget as HTMLSelectElement).value)}
            disabled={saving || loadingFormOptions || !formCustomerId || !formLocationId || showEdit}
          >
            <option value="">{$t('admin.network.dhcp_static.fields.select_subscription') || 'Select DHCP static subscription'}</option>
            {#each visibleSubscriptions as subscription}
              <option value={subscription.id}>
                {(subscription.package_name || selectedPackageName) + ' • ' + subscription.status}
              </option>
            {/each}
          </select>
          {#if formCustomerId && formLocationId && visibleSubscriptions.length === 0}
            <div class="field-hint">
              {$t('admin.network.dhcp_static.fields.no_subscription_for_location') ||
                'No DHCP static subscription is available for this location yet.'}
            </div>
          {/if}
        </label>
      </div>

      <div class="context-grid">
        <div class="context-card">
          <span class="context-label">{$t('common.location') || 'Location'}</span>
          <strong>{selectedLocation?.label || selectedLocationName}</strong>
        </div>
        <div class="context-card">
          <span class="context-label">{$t('common.package') || 'Package'}</span>
          <strong>{selectedPackageName}</strong>
        </div>
      </div>

      <div class="existing-services-panel">
        <div class="existing-services-head">
          <strong>
            {$t('admin.network.dhcp_static.fields.existing_services_title') ||
              'Existing DHCP Static'}
          </strong>
          <span>
            {tr(
              'admin.network.dhcp_static.fields.existing_services_count',
              '{count} item(s)',
              { count: existingCustomerDhcpServices.length },
            )}
          </span>
        </div>
        {#if !formCustomerId}
          <p class="field-hint">
            {$t('admin.network.dhcp_static.fields.choose_customer_for_existing_services') ||
              'Choose customer first to inspect existing DHCP static services.'}
          </p>
        {:else if !formLocationId}
          <p class="field-hint">
            {$t('admin.network.dhcp_static.fields.choose_location_for_existing_services') ||
              'Choose location to narrow existing DHCP static services.'}
          </p>
        {:else if existingCustomerDhcpServices.length === 0}
          <p class="field-hint">
            {$t('admin.network.dhcp_static.fields.no_existing_service_for_location') ||
              'No existing DHCP static service for this location.'}
          </p>
        {:else}
          <div class="existing-service-list">
            {#each existingCustomerDhcpServices as service}
              <div class="existing-service-item">
                <strong>{service.ip_address}</strong>
                <span>{service.mac_address}</span>
                <span>{service.dhcp_server_name}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </section>

    <section class="form-section">
      <div class="section-head">
        <div>
          <h3>
            {$t('admin.network.dhcp_static.modal.lease_title') || 'Lease Provisioning'}
          </h3>
          <p>
            {$t('admin.network.dhcp_static.modal.lease_help') ||
              'Fill router, DHCP server, customer MAC, and static IP assignment.'}
          </p>
        </div>
      </div>

      <div class="form-grid">
        <label>
          <span>{$t('admin.network.dhcp_static.filters.router') || 'Router'}</span>
          <select
            class:error={!!formRouterIdError}
            class="input"
            bind:value={formRouterId}
            onchange={(event) =>
              handleRouterChange((event.currentTarget as HTMLSelectElement).value, {
                preserveServerName: false,
              })}
            disabled={saving || loadingFormOptions}
          >
            <option value="">{$t('admin.network.dhcp_static.fields.select_router') || 'Select router'}</option>
            {#each routers as router}
              <option value={router.id}>{router.name}</option>
            {/each}
          </select>
          {#if formRouterIdError}
            <div class="field-error">{formRouterIdError}</div>
          {:else}
            <div class="field-hint">
              {$t('admin.network.dhcp_static.fields.selected_router') || 'Selected'}:
              <strong>{selectedRouterName}</strong>
            </div>
          {/if}
        </label>

        <label>
          <span>{$t('admin.network.dhcp_static.fields.dhcp_server_name') || 'DHCP Server Name'}</span>
          <select
            class:error={!!formDhcpServerNameError}
            class="input"
            bind:value={formDhcpServerName}
            disabled={saving || loadingRouterDhcpServers || !formRouterId}
          >
            <option value="">
              {$t('admin.network.dhcp_static.fields.select_dhcp_server') ||
                'Select DHCP server'}
            </option>
            {#each routerDhcpServerOptions as option}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
          {#if formDhcpServerNameError}
            <div class="field-error">{formDhcpServerNameError}</div>
          {:else if loadingRouterDhcpServers}
            <div class="field-hint">
              {$t('admin.network.dhcp_static.fields.loading_dhcp_servers') ||
                'Loading DHCP servers from router...'}
            </div>
          {:else if !formRouterId}
            <div class="field-hint">
              {$t('admin.network.dhcp_static.fields.pick_router_first') ||
                'Choose router first to load DHCP server names.'}
            </div>
          {:else if selectedRouterDhcpServer}
            <div class="field-hint">
              {selectedRouterDhcpServer.interface
                ? tr(
                    'admin.network.dhcp_static.fields.selected_dhcp_server_with_interface',
                    'Interface: {interface}',
                    { interface: selectedRouterDhcpServer.interface },
                  )
                : $t('admin.network.dhcp_static.fields.selected_dhcp_server_detected') ||
                  'Detected from router'}
            </div>
          {:else if routerDhcpServers.length > 0}
            <div class="field-hint">
              {tr(
                'admin.network.dhcp_static.fields.detected_dhcp_servers',
                '{count} DHCP server(s) detected from router',
                { count: routerDhcpServers.length },
              )}
            </div>
          {:else}
            <div class="field-hint">
              {$t('admin.network.dhcp_static.fields.no_dhcp_servers_available') ||
                'No DHCP server available from the selected router.'}
            </div>
          {/if}
        </label>

        <label>
          <span>{$t('admin.network.dhcp_static.fields.mac_address') || 'MAC Address'}</span>
          <input
            class:error={!!formMacAddressError}
            class="input mono"
            value={formMacAddress}
            oninput={(event) =>
              (formMacAddress = formatDhcpStaticMacAddressInput(
                (event.currentTarget as HTMLInputElement).value,
              ))}
            placeholder="AA:BB:CC:DD:EE:FF"
          />
          {#if formMacAddressError}
            <div class="field-error">{formMacAddressError}</div>
          {/if}
        </label>

        <label>
          <span>{$t('admin.network.dhcp_static.fields.ip_address') || 'IP Address'}</span>
          <input
            class:error={!!formIpAddressError}
            class="input mono"
            bind:value={formIpAddress}
            placeholder="192.168.1.10"
          />
          {#if formIpAddressError}
            <div class="field-error">{formIpAddressError}</div>
          {/if}
        </label>

        <label class="full">
          <span>{$t('common.notes') || 'Notes'}</span>
          <input
            class="input"
            bind:value={formComment}
            placeholder={$t('admin.network.dhcp_static.fields.comment_placeholder') ||
              'Optional note for static lease'}
          />
        </label>
      </div>
    </section>

    <section class="form-section">
      <div class="section-head">
        <div>
          <h3>
            {$t('admin.network.dhcp_static.modal.traffic_title') || 'Traffic Control'}
          </h3>
          <p>
            {$t('admin.network.dhcp_static.modal.traffic_help') ||
              'Optional queue settings for bandwidth shaping on the router.'}
          </p>
        </div>
      </div>

      <div class="form-grid queue-grid">
        <label>
          <span>{$t('admin.network.dhcp_static.fields.queue_mode') || 'Queue Mode'}</span>
          <select class="input" bind:value={formQueueMode}>
            <option value="none">{$t('admin.network.dhcp_static.sync.none') || 'None'}</option>
            <option value="simple_queue">{$t('admin.network.dhcp_static.fields.simple_queue') || 'Simple Queue'}</option>
          </select>
        </label>

        <label>
          <span>{$t('admin.network.dhcp_static.fields.queue_rate_limit') || 'Queue Rate Limit'}</span>
          <input
            class:error={!!formQueueRateLimitError}
            class="input mono"
            bind:value={formQueueRateLimit}
            placeholder="20M/20M"
            disabled={formQueueMode !== 'simple_queue'}
          />
          <div class="field-hint">
            {$t('admin.network.dhcp_static.fields.queue_rate_limit_hint') ||
              'Use format like 20M/20M for download/upload.'}
          </div>
          {#if formQueueRateLimitError}
            <div class="field-error">{formQueueRateLimitError}</div>
          {/if}
        </label>

        {#if formQueueMode === 'simple_queue'}
          <div class="preset-panel">
            <span class="preset-label">
              {$t('admin.network.dhcp_static.fields.queue_presets') || 'Quick presets'}
            </span>
            <div class="preset-chips">
              {#each queueRateLimitPresets as preset}
                <button
                  type="button"
                  class="preset-chip"
                  onclick={() => (formQueueRateLimit = preset)}
                >
                  {preset}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <label class="checkbox full checkbox-row">
          <input type="checkbox" bind:checked={formDisabled} />
          <span>{$t('admin.network.dhcp_static.fields.disabled') || 'Disabled on router'}</span>
        </label>
      </div>
    </section>

    {#if selectedSubscription}
      <div class="subscription-card">
        <strong>{$t('admin.network.dhcp_static.fields.subscription_context') || 'Subscription context'}</strong>
        <p>
          {customerNameById.get(selectedSubscription.customer_id) || selectedSubscription.customer_id}
          • {selectedSubscription.package_name || selectedPackageName}
          • {selectedSubscription.status}
        </p>
      </div>
    {/if}
  </div>

  <div class="modal-actions sticky-actions">
    <button
      class="btn ghost"
      onclick={() => {
        showCreate = false;
        showEdit = false;
      }}
      disabled={saving}
    >
      {$t('common.cancel') || 'Cancel'}
    </button>
    <button class="btn" onclick={showEdit ? submitEdit : submitCreate} disabled={saving}>
      {saving
        ? $t('common.saving') || 'Saving...'
        : showEdit
          ? $t('admin.network.dhcp_static.actions.update') || 'Update DHCP Static'
          : $t('admin.network.dhcp_static.actions.create') || 'Create DHCP Static'}
    </button>
  </div>
</Modal>

<style>
  .page-content {
    padding: 28px;
    max-width: 1400px;
    margin: 0 auto;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }

  .stat-card,
  .table-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }

  .stat-card {
    padding: 16px;
    display: grid;
    gap: 8px;
  }

  .stat-card span,
  .muted {
    color: var(--text-muted);
  }

  .stat-card strong {
    font-size: 1.5rem;
  }

  .filters-wrap {
    margin-bottom: 16px;
  }

  .filter-actions {
    display: flex;
    gap: 8px;
    align-items: end;
    flex-wrap: wrap;
  }

  .modal-form {
    display: grid;
    gap: 16px;
  }

  .form-section {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface) 82%, black);
    padding: 16px;
    display: grid;
    gap: 14px;
  }

  .section-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
  }

  .section-head h3 {
    margin: 0;
    font-size: 1rem;
  }

  .section-head p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px 14px;
  }

  .form-grid label {
    display: grid;
    gap: 7px;
  }

  .form-grid .full {
    grid-column: 1 / -1;
  }

  .context-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .existing-services-panel {
    border-top: 1px solid var(--border-subtle);
    padding-top: 14px;
    display: grid;
    gap: 10px;
  }

  .existing-services-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
  }

  .existing-services-head strong {
    font-size: 0.9rem;
  }

  .existing-services-head span {
    color: var(--text-muted);
    font-size: 0.82rem;
  }

  .existing-service-list {
    display: grid;
    gap: 8px;
  }

  .existing-service-item {
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--bg-primary);
    padding: 10px 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
  }

  .existing-service-item strong {
    font-size: 0.9rem;
  }

  .existing-service-item span {
    color: var(--text-muted);
    font-size: 0.82rem;
  }

  .context-card {
    border: 1px solid var(--border-subtle);
    border-radius: 14px;
    background: var(--bg-primary);
    padding: 12px 14px;
    display: grid;
    gap: 6px;
    min-height: 76px;
  }

  .context-card strong {
    font-size: 0.96rem;
    color: var(--text-primary);
    line-height: 1.35;
    word-break: break-word;
  }

  .context-label {
    color: var(--text-muted);
    font-size: 0.76rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .queue-grid {
    align-items: start;
  }

  .checkbox {
    display: flex !important;
    align-items: center;
    gap: 10px;
  }

  .checkbox-row {
    margin-top: 2px;
  }

  .field-hint {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .field-error {
    font-size: 0.82rem;
    color: var(--color-danger-700, #b91c1c);
  }

  .preset-panel {
    grid-column: 1 / -1;
    display: grid;
    gap: 8px;
    margin-top: -4px;
  }

  .preset-label {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .preset-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .preset-chip {
    border: 1px solid var(--border-subtle);
    border-radius: 999px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 5px 10px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .input.error {
    border-color: var(--color-danger-500, #ef4444);
  }

  .subscription-card {
    padding: 14px 16px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-subtle, rgba(148, 163, 184, 0.08));
  }

  .subscription-card p {
    margin: 6px 0 0;
    color: var(--text-muted);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }

  .sticky-actions {
    margin-top: 0;
    padding-top: 14px;
    border-top: 1px solid var(--border-subtle);
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-subtle);
    text-align: left;
  }

  .stack {
    display: grid;
    gap: 4px;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .status-pill.ok {
    color: var(--color-success-700, #166534);
  }

  .empty-state {
    padding: 28px;
    color: var(--text-muted);
  }

  @media (max-width: 640px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }

    .page-content {
      padding: 16px;
    }

    .context-grid,
    .form-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
