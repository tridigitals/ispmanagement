<script lang="ts">
  /*
    Detail pelanggan v2 — gelombang 21.

    Versi lama: `(app)/admin/customers/[id]/+page.svelte` (3.865 baris).
    Halaman ini HUB: identitas, lokasi, langganan, tagihan, aset, PPPoE,
    DHCP statis, dan riwayat. Alur berat yang sudah punya halaman v2
    sendiri (pembuatan WO instalasi, provisioning PPPoE, designer kabel)
    ditautkan, tidak diduplikasi.

    Temuan backend yang dikunci gelombang ini (sudah dipatch):
    1. delete_customer tidak punya guard FK — pelanggan dengan langganan/
       WO/PPPoE/DHCP/lokasi membalas 500 FK-violation mentah. Kini 400
       deskriptif ("masih dipakai oleh N langganan, ...").
    2. delete_location sama persis celahnya. Kini guard + 400.
    3. delete_customer_subscription membiarkan dhcp_static_services /
       installation_work_orders (RESTRICT) meledak jadi 500. Kini guard.
  */
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import type {
    AuditLog,
    Customer,
    CustomerLocation,
    CustomerPortalUser,
    CustomerSubscriptionView,
    DhcpStaticServicePublic,
    Invoice,
    IspPackage,
    NetworkAssetListItem,
    PppoeAccountPublic,
  } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { fetchAllPages } from '$lib/utils/fetchAllPages';
  import { toast } from '$lib/stores/toast';
  import { formatDate, formatDateTime, timeAgo } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import {
    customerHealthChips,
    friendlyCustomerError,
    invoicesForSubscriptions,
    subStatusTone,
    subStatusLabel,
    subscriptionIdFromInvoice,
    formatLocationLine,
  } from '$lib/utils/customerDetailInsights';
  import {
    getVisibleCustomerDetailTabs,
    normalizeCustomerDetailTab,
    type CustomerDetailTab,
  } from '$lib/utils/customerDetailAccess';
  import {
    buildCustomerBillingStats,
    filterCustomerBillingRows,
    type CustomerBillingFilter,
  } from '../../../../../(app)/admin/customers/[id]/customerBillingState';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import EmptyState from '$lib/components/ds/EmptyState.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import type { Column } from '$lib/components/ds/table-types';

  import { t } from 'svelte-i18n';
  const customerId = $derived($page.params.id || '');

  const canManage = $derived($can('manage', 'customers'));
  const canReadLocations = $derived($can('read', 'customer_locations') || $can('manage', 'customer_locations'));
  const canReadBilling = $derived($can('read', 'subscriptions') || canManage);
  const canReadAssets = $derived($can('read', 'network_assets'));
  const canReadPppoe = $derived($can('read', 'pppoe'));
  const canReadDhcp = $derived($can('read', 'dhcp_static'));
  const canReadAudit = $derived($can('read', 'audit_logs'));

  const access = $derived({
    canReadCustomerLocations: canReadLocations,
    canReadBilling,
    canReadFtthAssets: canReadAssets,
    canReadPppoe,
    canReadDhcpStatic: canReadDhcp,
    canReadAudit,
  });
  const visibleTabs = $derived(getVisibleCustomerDetailTabs(access));

  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let customer = $state<Customer | null>(null);
  let activeTab = $state<CustomerDetailTab>('overview');

  let locations = $state<CustomerLocation[]>([]);
  let subscriptions = $state<CustomerSubscriptionView[]>([]);
  let invoices = $state<Invoice[]>([]);
  let portalUsers = $state<CustomerPortalUser[]>([]);
  let assets = $state<NetworkAssetListItem[]>([]);
  let pppoeAccounts = $state<PppoeAccountPublic[]>([]);
  let dhcpServices = $state<DhcpStaticServicePublic[]>([]);
  let timeline = $state<AuditLog[]>([]);
  let packages = $state<IspPackage[]>([]);

  const custActive = $derived(customer?.is_active ?? false);

  const healthChips = $derived(
    customer
      ? customerHealthChips({
          is_active: customer.is_active,
          subscriptions: subscriptions.map((s) => ({ status: s.status })),
          pendingInstallations: subscriptions.filter((s) => s.status === 'pending_installation').length,
        })
      : [],
  );

  const subscriptionById = $derived(new Map(subscriptions.map((s) => [s.id, s])));
  const billingFilter = $state<{ value: CustomerBillingFilter }>({ value: 'all' });
  const billingRows = $derived(
    filterCustomerBillingRows({
      invoices,
      subscriptionById,
      getSubscriptionIdFromInvoice: subscriptionIdFromInvoice,
      filter: billingFilter.value,
    }),
  );
  const billingStats = $derived(
    buildCustomerBillingStats({ invoices, subscriptionById, getSubscriptionIdFromInvoice: subscriptionIdFromInvoice }),
  );

  onMount(() => {
    const tabParam = $page.url.searchParams.get('tab');
    activeTab = normalizeCustomerDetailTab(tabParam, access);
    void loadAll();
  });

  async function loadAll() {
    loading = true;
    loadError = null;
    try {
      customer = await api.customers.get(customerId);
      const results = await Promise.allSettled([
        canReadLocations ? api.customers.locations.list(customerId) : Promise.resolve([]),
        canReadBilling
          ? Promise.all([
              // A-03: loop halaman ber-cap (500x10) — dulu bisu di baris ke-N.
              fetchAllPages((page, per_page) =>
                api.customers.subscriptions.list(customerId, { page, per_page }),
              ),
              fetchAllPages((page, per_page) =>
                api.payment.listCustomerPackageInvoices({ page, per_page }),
              ),
            ])
          : Promise.resolve([null, null] as const),
        canManage ? api.customers.portalUsers.list(customerId).catch(() => []) : Promise.resolve([]),
        canReadAssets ? api.networkAssets.listCustomerAssets(customerId).catch(() => []) : Promise.resolve([]),
        canReadPppoe
          ? api.pppoe.accounts.list({ customer_id: customerId, page: 1, per_page: 200 }).catch(() => ({ data: [] }))
          : Promise.resolve({ data: [] }),
        canReadDhcp
          ? api.dhcpStatic.services.list({ customer_id: customerId, page: 1, per_page: 200 }).catch(() => ({ data: [] }))
          : Promise.resolve({ data: [] }),
        canReadAudit
          ? api.audit.listTenant(1, 100, { customer_id: customerId }).catch(() => ({ data: [] }))
          : Promise.resolve({ data: [] }),
      ]);
      locations = (results[0].status === 'fulfilled' ? results[0].value : []) as CustomerLocation[];
      const subsRes = results[1].status === 'fulfilled' ? results[1].value : null;
      subscriptions = subsRes?.[0]?.rows || [];
      const allInvoices: Invoice[] = subsRes?.[1]?.rows || [];
      const subIds = new Set(subscriptions.map((s) => s.id));
      invoices = invoicesForSubscriptions(allInvoices, subIds);
      portalUsers = (results[2].status === 'fulfilled' ? results[2].value : []) as CustomerPortalUser[];
      assets = (results[3].status === 'fulfilled' ? results[3].value : []) as NetworkAssetListItem[];
      pppoeAccounts = (results[4].status === 'fulfilled' ? results[4].value.data : []) || [];
      dhcpServices = (results[5].status === 'fulfilled' ? results[5].value.data : []) || [];
      timeline = (results[6].status === 'fulfilled' ? results[6].value.data : []) || [];
      if (canManage) {
        packages = (await api.ispPackages.packages.list({ page: 1, per_page: 500, q: '' }).catch(() => ({ data: [] }))).data || [];
      }
    } catch (e) {
      loadError = friendlyCustomerError(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  // A-18: indikator bahwa tab bisa digeser (mobile) — fade + panah yang
  // hanya muncul bila konten overflow dan belum di-scroll ke ujung.
  let tabsEl: HTMLElement | undefined = $state();
  let tabsOverflow = $state(false);
  let tabsAtEnd = $state(false);
  function measureTabs() {
    const el = tabsEl;
    if (!el) return;
    tabsOverflow = el.scrollWidth - el.clientWidth > 4;
    tabsAtEnd = el.scrollLeft + el.clientWidth >= el.scrollWidth - 4;
  }
  $effect(() => {
    const el = tabsEl;
    if (!el) return;
    measureTabs();
    const ro = new ResizeObserver(measureTabs);
    ro.observe(el);
    return () => ro.disconnect();
  });

  function selectTab(tab: CustomerDetailTab) {
    activeTab = tab;
    void goto(`/v2/admin/customers/${customerId}?tab=${tab}`, { keepFocus: true, noScroll: true, replaceState: true });
  }

  let busy = $state(false);

  async function runAction(fn: () => Promise<unknown>, okMsg: string) {
    if (busy) return false;
    busy = true;
    try {
      await fn();
      toast.success(okMsg);
      await loadAll();
      return true;
    } catch (e) {
      toast.error(friendlyCustomerError(extractApiErrorMessage(e)));
      return false;
    } finally {
      busy = false;
    }
  }

  // ---- overview edit ----
  let showEdit = $state(false);
  let editName = $state('');
  let editEmail = $state('');
  let editPhone = $state('');
  let editNotes = $state('');

  function openEdit() {
    if (!customer) return;
    editName = customer.name;
    editEmail = customer.email || '';
    editPhone = customer.phone || '';
    editNotes = customer.notes || '';
    showEdit = true;
  }

  async function submitEdit() {
    if (!editName.trim()) {
      toast.error($t('admin.customers.detail.v2.t_name'));
      return;
    }
    if (await runAction(() => api.customers.update(customerId, {
      name: editName.trim(),
      email: editEmail.trim() || null,
      phone: editPhone.trim() || null,
      notes: editNotes.trim() || null,
    }), 'Profil tersimpan.')) showEdit = false;
  }

  async function toggleActive() {
    if (!customer) return;
    const next = !customer.is_active;
    if (await runAction(() => api.customers.update(customerId, { is_active: next }), next ? 'Pelanggan diaktifkan.' : 'Pelanggan dinonaktifkan.')) return;
  }

  // ---- delete customer ----
  let showDelete = $state(false);
  let deleteConfirmText = $state('');
  async function doDeleteCustomer() {
    if (deleteConfirmText.trim().toUpperCase() !== 'HAPUS') {
      toast.error($t('admin.customers.detail.v2.t_hapus'));
      return;
    }
    showDelete = false;
    if (await runAction(() => api.customers.delete(customerId), 'Pelanggan dihapus.')) {
      void goto('/v2/admin/customers');
    }
  }

  // ---- locations ----
  let showLocForm = $state(false);
  let locEditing = $state<CustomerLocation | null>(null);
  let locLabel = $state('');
  let locAddr1 = $state('');
  let locCity = $state('');
  let locNotes = $state('');
  let locError = $state<string | null>(null);

  function openLocCreate() {
    locEditing = null;
    locLabel = ''; locAddr1 = ''; locCity = ''; locNotes = '';
    locError = null;
    showLocForm = true;
  }
  function openLocEdit(row: CustomerLocation) {
    locEditing = row;
    locLabel = row.label; locAddr1 = row.address_line1 || ''; locCity = row.city || ''; locNotes = row.notes || '';
    locError = null;
    showLocForm = true;
  }
  async function submitLoc() {
    if (!locLabel.trim()) { locError = $t('admin.customers.detail.v2.label_required'); return; }
    const dto = {
      label: locLabel.trim(),
      address_line1: locAddr1.trim() || null,
      city: locCity.trim() || null,
      notes: locNotes.trim() || null,
    };
    const ok = locEditing
      ? await runAction(() => api.customers.locations.update(locEditing!.id, dto), 'Lokasi tersimpan.')
      : await runAction(() => api.customers.locations.create({ customer_id: customerId, ...dto }), 'Lokasi ditambahkan.');
    if (ok) showLocForm = false;
  }
  let locDeleteTarget = $state<CustomerLocation | null>(null);
  let showLocDelete = $state(false);
  function askLocDelete(row: CustomerLocation) {
    locDeleteTarget = row;
    showLocDelete = true;
  }
  async function doLocDelete() {
    if (!locDeleteTarget) return;
    const t = locDeleteTarget;
    locDeleteTarget = null;
    showLocDelete = false;
    await runAction(() => api.customers.locations.delete(t.id), 'Lokasi dihapus.');
  }

  // ---- subscriptions ----
  let showSubForm = $state(false);
  let subEditing = $state<CustomerSubscriptionView | null>(null);
  let subLocationId = $state('');
  let subPackageId = $state('');
  let subCycle = $state<'monthly' | 'yearly'>('monthly');
  let subPrice = $state('');
  let subStatus = $state('active');
  let subError = $state<string | null>(null);

  function openSubCreate() {
    subEditing = null;
    subLocationId = locations[0]?.id || '';
    subPackageId = ''; subCycle = 'monthly'; subPrice = ''; subStatus = 'pending_installation';
    subError = null;
    showSubForm = true;
  }
  function openSubEdit(row: CustomerSubscriptionView) {
    subEditing = row;
    subLocationId = row.location_id;
    subPackageId = row.package_id;
    subCycle = row.billing_cycle === 'yearly' ? 'yearly' : 'monthly';
    subPrice = String(row.price);
    subStatus = row.status;
    subError = null;
    showSubForm = true;
  }
  async function submitSub() {
    if (!subLocationId || !subPackageId) { subError = 'Lokasi dan paket wajib dipilih.'; return; }
    const price = Number(subPrice);
    if (!Number.isFinite(price) || price < 0) { subError = 'Harga tidak valid.'; return; }
    const dto = {
      location_id: subLocationId,
      package_id: subPackageId,
      billing_cycle: subCycle,
      price,
      status: subStatus,
    };
    const ok = subEditing
      ? await runAction(() => api.customers.subscriptions.update(subEditing!.id, dto), 'Langganan tersimpan.')
      : await runAction(() => api.customers.subscriptions.create(customerId, dto), 'Langganan dibuat.');
    if (ok) showSubForm = false;
  }
  async function setSubStatus(row: CustomerSubscriptionView, next: 'active' | 'suspended') {
    await runAction(() => api.customers.subscriptions.update(row.id, { status: next }), next === 'active' ? 'Ditangguhkan dicabut.' : 'Langganan ditangguhkan.');
  }
  let subDeleteTarget = $state<CustomerSubscriptionView | null>(null);
  let showSubDelete = $state(false);
  function askSubDelete(row: CustomerSubscriptionView) {
    subDeleteTarget = row;
    showSubDelete = true;
  }
  async function doSubDelete() {
    if (!subDeleteTarget) return;
    const t = subDeleteTarget;
    subDeleteTarget = null;
    showSubDelete = false;
    await runAction(() => api.customers.subscriptions.delete(t.id), 'Langganan dihapus.');
  }

  // ---- change package ----
  let showChangePkg = $state(false);
  let chgTarget = $state<CustomerSubscriptionView | null>(null);
  let chgNewPkg = $state('');
  async function submitChangePkg() {
    if (!chgTarget || !chgNewPkg) return;
    const ok = await runAction(
      () => api.payment.changePackage({ subscription_id: chgTarget!.id, new_package_id: chgNewPkg }),
      'Paket diganti.',
    );
    if (ok) { showChangePkg = false; chgTarget = null; chgNewPkg = ''; }
  }

  // ---- portal users ----
  let showPortalAdd = $state(false);
  let portalEmail = $state('');
  let portalName = $state('');
  let portalPass = $state('');
  async function addPortalUser() {
    if (!portalEmail.trim() || portalPass.length < 8) {
      toast.error($t('admin.customers.detail.v2.t_email_pass'));
      return;
    }
    const ok = await runAction(
      () => api.customers.portalUsers.createNew({
        customer_id: customerId,
        email: portalEmail.trim(),
        name: portalName.trim() || portalEmail.trim(),
        password: portalPass,
      }),
      'Akun portal dibuat.',
    );
    if (ok) { showPortalAdd = false; portalEmail = portalName = portalPass = ''; }
  }
  let portalRemoveTarget = $state<CustomerPortalUser | null>(null);
  let showPortalRemove = $state(false);
  function askPortalRemove(u: CustomerPortalUser) {
    portalRemoveTarget = u;
    showPortalRemove = true;
  }
  async function doPortalRemove() {
    if (!portalRemoveTarget) return;
    const t = portalRemoveTarget;
    portalRemoveTarget = null;
    showPortalRemove = false;
    await runAction(() => api.customers.portalUsers.remove(t.customer_user_id), 'Akses portal dicabut.');
  }

  function tz() {
    return $appSettings.app_timezone;
  }
</script>

<AppShell title={customer?.name || $t('common.customer')}>
  {#if loading}
    <div class="py-16 text-center text-ink-500">{ $t('admin.customers.detail.v2.loading') }</div>
  {:else if !customer}
    <div class="py-16 text-center">
      <div class="text-base font-medium text-ink-900">{loadError || $t('admin.customers.detail.v2.not_found')}</div>
      <Button variant="ghost" class="mt-3" onclick={() => goto('/v2/admin/customers')}>{ $t('admin.customers.detail.v2.back_list') }</Button>
    </div>
  {:else}
    <PageHeader title={customer.name} desc={(customer.customer_number || '') + (customer.email ? ` · ${customer.email}` : '') + (customer.phone ? ` · ${customer.phone}` : '')}>
      {#snippet actions()}
        {#if canManage}
          <Button variant="ghost" icon="clipboard" onclick={openEdit}>{ $t('admin.customers.detail.v2.act_edit') }</Button>
          <Button variant={custActive ? 'ghost' : 'primary'} onclick={() => void toggleActive()}>
            {custActive ? $t('admin.customers.detail.v2.act_disable') : $t('admin.customers.detail.v2.act_enable')}
          </Button>
          <Button variant="danger" onclick={() => (showDelete = true)}>{ $t('common.delete') }</Button>
        {/if}
      {/snippet}
    </PageHeader>

    <div class="mt-3 flex flex-wrap gap-2">
      {#each healthChips as chip (chip.key)}
        <Badge tone={chip.tone} label={chip.label} />
      {/each}
    </div>

    <div class="relative mt-5" aria-label={ $t('admin.customers.detail.v2.tab_aria') }>
    <nav
      class="flex gap-1 overflow-x-auto border-b border-ink-200"
      bind:this={tabsEl}
      onscroll={measureTabs}
    >
      {#each visibleTabs as tab (tab)}
        <button
          type="button"
          class="focus-ring whitespace-nowrap rounded-t-lg px-3 py-2 text-sm {activeTab === tab ? 'bg-white font-medium text-ink-900 shadow-[inset_0_-2px_0_0_var(--color-ink-900)]' : 'text-ink-500 hover:text-ink-900'}"
          aria-current={activeTab === tab ? 'page' : undefined}
          onclick={() => selectTab(tab)}
        >
          {tab === 'overview' ? $t('admin.customers.tabs.overview') : tab === 'locations' ? $t('admin.customers.tabs.locations') : tab === 'subscriptions' ? $t('admin.customers.tabs.subscriptions') : tab === 'billing' ? $t('admin.network.installations.billing') : tab === 'assets' ? $t('admin.customers.assets.columns.name') : tab === 'pppoe' ? 'PPPoE' : tab === 'dhcp_static' ? $t('admin.customers.detail.v2.tab_dhcp') : $t('admin.network.installations.v2.history')}
        </button>
      {/each}
    </nav>
    {#if tabsOverflow && !tabsAtEnd}
      <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center gap-1 bg-gradient-to-l from-white via-white/90 to-transparent pl-6 pr-1">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-ink-500 animate-pulse" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>
      </div>
    {/if}
    </div>

    {#if activeTab === 'overview'}
      <div class="mt-4 grid gap-4 lg:grid-cols-3">
        <div class="rounded-xl bg-white p-4 ring-1 ring-ink-200 lg:col-span-2">
          <div class="text-sm font-medium text-ink-900">{ $t('common.profile') }</div>
          <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-1.5 text-sm">
            <dt class="text-ink-500">{ $t('admin.customers.detail.v2.dt_number') }</dt><dd class="text-ink-900">{customer.customer_number || '—'}</dd>
            <dt class="text-ink-500">{ $t('admin.customers.fields.email') }</dt><dd class="text-ink-900">{customer.email || '—'}</dd>
            <dt class="text-ink-500">{ $t('admin.customers.fields.phone') }</dt><dd class="text-ink-900">{customer.phone || '—'}</dd>
            <dt class="text-ink-500">{ $t('admin.customers.detail.v2.dt_registered') }</dt><dd class="text-ink-900">{formatDate(customer.created_at, { timeZone: tz() })}</dd>
            <dt class="text-ink-500">{ $t('common.notes') }</dt><dd class="text-ink-900">{customer.notes || '—'}</dd>
          </dl>
        </div>
        <div class="rounded-xl bg-white p-4 ring-1 ring-ink-200">
          <div class="text-sm font-medium text-ink-900">{ $t('admin.customers.detail.v2.portal_access') }</div>
          {#if portalUsers.length === 0}
            <EmptyState title={ $t('admin.customers.detail.v2.portal_none') } hint={ $t('admin.customers.detail.v2.portal_none_hint') } />
          {:else}
            <ul class="mt-2 space-y-1.5 text-sm">
              {#each portalUsers as u (u.customer_user_id)}
                <li class="flex items-center justify-between gap-2">
                  <span class="min-w-0 truncate text-ink-900">{u.name} <span class="text-ink-500">· {u.email}</span></span>
                  {#if canManage}
                    <button type="button" class="focus-ring inline-flex min-h-[24px] shrink-0 items-center px-1 text-sm text-red-700 underline" onclick={() => askPortalRemove(u)}>{ $t('admin.customers.detail.v2.portal_revoke') }</button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
          {#if canManage}
            <Button variant="ghost" size="sm" class="mt-2" onclick={() => (showPortalAdd = true)}>{ $t('admin.customers.detail.v2.portal_add') }</Button>
          {/if}
        </div>
      </div>
      <div class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <StatTile label={ $t('admin.customers.tabs.subscriptions') } value={String(subscriptions.length)} hint={ $t('admin.customers.detail.v2.hint_incl_cancel') } />
        <StatTile label={ $t('admin.customers.tabs.locations') } value={String(locations.length)} hint={ $t('admin.customers.detail.v2.hint_install_addr') } />
        <StatTile label={ $t('admin.customers.detail.v2.ont') } value={String(assets.length)} hint={ $t('admin.customers.detail.v2.hint_bound') } />
        <StatTile label={ $t('admin.customers.detail.v2.unpaid') } value={String(billingStats.unpaid)} hint={ $t('admin.customers.detail.v2.hint_pkg_invoice') } tone={billingStats.unpaid ? 'negative' : 'positive'} />
      </div>
    {:else if activeTab === 'locations'}
      <div class="mt-4 flex items-center justify-between">
        <div class="text-sm text-ink-500">{locations.length} {$t('admin.customers.detail.v2.c_locations')}</div>
        {#if canManage}<Button variant="primary" size="sm" icon="plus" onclick={openLocCreate}>{ $t('admin.customers.detail.v2.loc_add') }</Button>{/if}
      </div>
      <div class="mt-3">
        <DataTable
          columns={[{ key: 'loc', label: $t('admin.customers.tabs.locations') }, { key: 'subs', label: $t('admin.customers.tabs.subscriptions'), width: '110px' }, { key: 'actions', label: '', width: '150px', align: 'right' }]}
          rows={locations}
          pageSize={25}
          emptyTitle={ $t('admin.customers.detail.v2.loc_empty') }
          emptyHint={ $t('admin.customers.detail.v2.loc_empty_hint') }
        >
          {#snippet cell(row: CustomerLocation, col: Column)}
            {#if col.key === 'loc'}
              <div class="text-sm text-ink-900">{formatLocationLine(row)}</div>
              {#if row.notes}<div class="text-sm text-ink-500">{row.notes}</div>{/if}
            {:else if col.key === 'subs'}
              <span class="text-sm text-ink-700">{subscriptions.filter((s) => s.location_id === row.id).length}</span>
            {:else if col.key === 'actions'}
              <RowActions
                primary={{ label: $t('admin.customers.detail.v2.act_edit'), onclick: () => openLocEdit(row), disabled: !canManage }}
                rest={canManage ? [{ label: $t('common.delete'), danger: true, onclick: () => askLocDelete(row) }] : []}
              />
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'subscriptions'}
      <div class="mt-4 flex items-center justify-between">
        <div class="text-sm text-ink-500">{subscriptions.length} {$t('admin.customers.detail.v2.c_subs')}</div>
        {#if canManage}<Button variant="primary" size="sm" icon="plus" onclick={openSubCreate}>{ $t('admin.customers.detail.v2.sub_new') }</Button>{/if}
      </div>
      <div class="mt-3">
        <DataTable
          columns={[
            { key: 'pkg', label: $t('common.package') },
            { key: 'status', label: $t('common.status'), width: '150px' },
            { key: 'price', label: $t('admin.customers.detail.v2.col_price'), width: '130px' },
            { key: 'period', label: $t('admin.customers.detail.v2.col_period'), width: '170px' },
            { key: 'actions', label: '', width: '190px', align: 'right' },
          ]}
          rows={subscriptions}
          pageSize={25}
          emptyTitle="Belum ada langganan"
          emptyHint="Buat langganan dari paket ISP yang tersedia."
        >
          {#snippet cell(row: CustomerSubscriptionView, col: Column)}
            {#if col.key === 'pkg'}
              <div class="min-w-0">
                <div class="truncate font-medium text-ink-900">{row.package_name || '—'}</div>
                <div class="truncate text-sm text-ink-500">{row.location_label || $t('admin.network.installations.v2.no_location')}{#if row.router_name} · {row.router_name}{/if}</div>
              </div>
            {:else if col.key === 'status'}
              <Badge tone={subStatusTone(row.status)} label={subStatusLabel(row.status)} />
            {:else if col.key === 'price'}
              <span class="text-sm text-ink-900">{formatMoney(row.price, { currency: row.currency_code })}<span class="text-ink-500">/{row.billing_cycle === 'yearly' ? $t('admin.customers.detail.v2.yearly_abbr') : $t('admin.customers.detail.v2.monthly_abbr')}</span></span>
            {:else if col.key === 'period'}
              <span class="text-sm text-ink-500">{row.starts_at ? formatDate(row.starts_at, { timeZone: tz() }) : '—'} → {row.ends_at ? formatDate(row.ends_at, { timeZone: tz() }) : '—'}</span>
            {:else if col.key === 'actions'}
              <RowActions
                primary={{ label: $t('admin.network.incidents.ui.row_detail'), icon: 'search', href: `/v2/admin/services?sub=${row.id}` }}
                rest={canManage
                  ? [
                      { label: $t('admin.customers.detail.v2.act_edit'), onclick: () => openSubEdit(row) },
                      { label: $t('admin.customers.detail.v2.chg_btn'), onclick: () => { chgTarget = row; chgNewPkg = ''; showChangePkg = true; } },
                      ...(row.status === 'active' ? [{ label: $t('admin.customers.detail.v2.act_suspend'), onclick: () => void setSubStatus(row, 'suspended') }] : []),
                      ...(row.status === 'suspended' ? [{ label: $t('admin.customers.detail.v2.act_enable'), onclick: () => void setSubStatus(row, 'active') }] : []),
                      { label: $t('common.delete'), danger: true, onclick: () => askSubDelete(row) },
                    ]
                  : []}
              />
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'billing'}
      <div class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
        {#each [['all', $t('admin.customers.billing.filters.all')], ['unpaid', $t('admin.customers.detail.v2.chip_unpaid')], ['paid', $t('admin.customers.billing.stats.paid')], ['overdue', $t('admin.customers.detail.v2.chip_overdue')]] as [key, label] (key)}
          <button
            type="button"
            class="focus-ring rounded-xl text-left {billingFilter.value === key ? 'ring-2 ring-ink-900' : ''}"
            aria-pressed={billingFilter.value === key}
            onclick={() => (billingFilter.value = key as CustomerBillingFilter)}
          >
            <StatTile label={label} value={String(billingStats[key as keyof typeof billingStats])} hint={ $t('admin.network.installations.v2.hint_filter') } tone={key === 'overdue' && billingStats.overdue ? 'negative' : 'neutral'} />
          </button>
        {/each}
      </div>
      <div class="mt-3">
        <DataTable
          columns={[
            { key: 'inv', label: $t('common.invoice') },
            { key: 'status', label: $t('common.status'), width: '130px' },
            { key: 'amount', label: $t('admin.customers.detail.v2.col_price'), width: '140px' },
            { key: 'due', label: $t('admin.customers.billing.columns.due_date'), width: '140px' },
            { key: 'actions', label: '', width: '110px', align: 'right' },
          ]}
          rows={billingRows}
          pageSize={25}
          emptyTitle={ $t('admin.customers.detail.v2.bill_empty') }
          emptyHint={ $t('admin.customers.detail.v2.bill_empty_hint') }
        >
          {#snippet cell(row: Invoice, col: Column)}
            {#if col.key === 'inv'}
              <div class="min-w-0">
                <div class="truncate font-medium text-ink-900">{row.invoice_number}</div>
                <div class="truncate text-sm text-ink-500">{row.description || '—'}</div>
              </div>
            {:else if col.key === 'status'}
              <Badge tone={row.status === 'paid' ? 'positive' : row.status === 'failed' ? 'negative' : 'warning'} label={row.status === 'paid' ? $t('admin.customers.billing.stats.paid') : row.status === 'failed' ? $t('admin.customers.detail.v2.inv_failed') : row.status === 'verification_pending' ? $t('admin.customers.detail.v2.inv_verif') : $t('admin.customers.detail.v2.chip_unpaid')} />
            {:else if col.key === 'amount'}
              <span class="text-sm text-ink-900">{formatMoney(row.amount, { currency: row.currency_code })}</span>
            {:else if col.key === 'due'}
              <span class="text-sm {row.status !== 'paid' && new Date(row.due_date).getTime() < Date.now() ? 'text-red-600' : 'text-ink-500'}">{formatDate(row.due_date, { timeZone: tz() })}</span>
            {:else if col.key === 'actions'}
              <RowActions primary={{ label: $t('admin.customers.detail.v2.act_open'), href: `/v2/admin/invoices/${row.id}` }} />
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'assets'}
      <div class="mt-4">
        <DataTable
          columns={[
            { key: 'name', label: $t('admin.customers.assets.columns.name') },
            { key: 'type', label: $t('admin.customers.assets.columns.type'), width: '120px' },
            { key: 'status', label: $t('common.status'), width: '120px' },
            { key: 'loc', label: $t('admin.customers.tabs.locations'), width: '180px' },
            { key: 'actions', label: '', width: '110px', align: 'right' },
          ]}
          rows={assets}
          pageSize={25}
          emptyTitle={ $t('admin.customers.detail.v2.asset_empty') }
          emptyHint={ $t('admin.customers.detail.v2.asset_empty_hint') }
        >
          {#snippet cell(row: NetworkAssetListItem, col: Column)}
            {#if col.key === 'name'}
              <div class="min-w-0">
                <div class="truncate font-medium text-ink-900">{row.name}</div>
                <div class="truncate text-sm text-ink-500">{row.serial_number || row.code || '—'}</div>
              </div>
            {:else if col.key === 'type'}
              <span class="text-sm text-ink-700">{row.asset_type}</span>
            {:else if col.key === 'status'}
              <Badge tone={row.status === 'active' ? 'positive' : row.status === 'decommissioned' ? 'neutral' : 'warning'} label={row.status} />
            {:else if col.key === 'loc'}
              <span class="text-sm text-ink-500">{row.location_label || '—'}</span>
            {:else if col.key === 'actions'}
              <RowActions primary={{ label: $t('admin.customers.assets.columns.name'), href: `/v2/admin/network/assets?q=${encodeURIComponent(row.name)}` }} />
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'pppoe'}
      <div class="mt-4 flex items-center justify-between">
        <div class="text-sm text-ink-500">{pppoeAccounts.length} {$t('admin.customers.detail.v2.c_pppoe')}</div>
        {#if canManage}<Button variant="ghost" size="sm" onclick={() => goto(`/v2/admin/network/pppoe?customer=${customerId}`)}>{ $t('admin.customers.detail.v2.pppoe_manage') }</Button>{/if}
      </div>
      <div class="mt-3">
        <DataTable
          columns={[
            { key: 'user', label: $t('common.username') },
            { key: 'profile', label: $t('common.profile'), width: '150px' },
            { key: 'ip', label: 'IP', width: '140px' },
            { key: 'online', label: $t('common.status'), width: '110px' },
          ]}
          rows={pppoeAccounts}
          pageSize={25}
          emptyTitle={ $t('admin.customers.detail.v2.pppoe_empty') }
          emptyHint={ $t('admin.customers.detail.v2.pppoe_empty_hint') }
        >
          {#snippet cell(row: PppoeAccountPublic, col: Column)}
            {#if col.key === 'user'}
              <div class="font-medium text-ink-900">{row.username}</div>
              <div class="text-sm text-ink-500">{row.account_source === 'managed_radius' ? $t('admin.customers.detail.v2.src_managed') : $t('admin.network.dhcp_static.filters.router')}</div>
            {:else if col.key === 'profile'}
              <span class="text-sm text-ink-700">{row.router_profile_name || '—'}</span>
            {:else if col.key === 'ip'}
              <span class="text-sm text-ink-700">{row.remote_address || '—'}</span>
            {:else if col.key === 'online'}
              <Badge tone={row.disabled ? 'neutral' : 'positive'} label={row.disabled ? $t('common.disabled') : $t('common.enabled')} />
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'dhcp_static'}
      <div class="mt-4 flex items-center justify-between">
        <div class="text-sm text-ink-500">{dhcpServices.length} {$t('admin.customers.detail.v2.c_dhcp')}</div>
        {#if canManage}<Button variant="ghost" size="sm" onclick={() => goto('/v2/admin/network/dhcp-static')}>{ $t('admin.customers.detail.v2.dhcp_manage') }</Button>{/if}
      </div>
      <div class="mt-3">
        <DataTable
          columns={[
            { key: 'mac', label: $t('admin.network.dhcp_static.columns.mac') },
            { key: 'ip', label: 'IP', width: '150px' },
            { key: 'server', label: $t('admin.customers.detail.v2.col_server'), width: '160px' },
            { key: 'sync', label: $t('admin.customers.pppoe.columns.sync'), width: '120px' },
          ]}
          rows={dhcpServices}
          pageSize={25}
          emptyTitle={ $t('admin.customers.detail.v2.dhcp_empty') }
          emptyHint={ $t('admin.customers.detail.v2.dhcp_empty_hint') }
        >
          {#snippet cell(row: DhcpStaticServicePublic, col: Column)}
            {#if col.key === 'mac'}
              <span class="font-mono text-sm text-ink-900">{row.mac_address}</span>
            {:else if col.key === 'ip'}
              <span class="font-mono text-sm text-ink-700">{row.ip_address}</span>
            {:else if col.key === 'server'}
              <span class="text-sm text-ink-700">{row.dhcp_server_name}</span>
            {:else if col.key === 'sync'}
              <Badge tone={row.lease_present ? 'positive' : 'warning'} label={row.lease_present ? $t('admin.customers.detail.v2.lease_present') : $t('admin.customers.detail.v2.lease_missing')} />
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'timeline'}
      <div class="mt-4">
        {#if timeline.length === 0}
          <div class="rounded-xl bg-white ring-1 ring-ink-200">
            <EmptyState icon="activity" title={ $t('admin.customers.detail.v2.no_activity') } hint={ $t('admin.customers.detail.v2.no_activity_hint') } />
          </div>
        {:else}
          <ul class="space-y-2">
            {#each timeline as log (log.id)}
              <li class="flex items-baseline justify-between gap-3 rounded-lg bg-white px-3 py-2 text-sm ring-1 ring-ink-200">
                <span class="min-w-0 text-ink-900">{log.action} <span class="text-ink-500">{log.details || ''}</span></span>
                <span class="shrink-0 text-xs text-ink-400">{timeAgo(log.created_at, (k) => $t(k))}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

  <Modal bind:show={showEdit} title={ $t('admin.customers.detail.v2.edit_title') }>
    <div class="space-y-3">
      <Field id="c-name" label={ $t('admin.customers.fields.name') } type="text" stacked value={editName} onchange={(v) => (editName = String(v ?? ''))} error={editName.trim() ? null : $t('admin.customers.detail.v2.required')} />
      <Field id="c-email" label={ $t('admin.customers.fields.email') } type="text" stacked value={editEmail} onchange={(v) => (editEmail = String(v ?? ''))} />
      <Field id="c-phone" label={ $t('admin.customers.fields.phone') } type="text" stacked value={editPhone} onchange={(v) => (editPhone = String(v ?? ''))} />
      <Field id="c-notes" label={ $t('common.notes') } type="textarea" stacked rows={3} value={editNotes} onchange={(v) => (editNotes = String(v ?? ''))} />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showEdit = false)}>{ $t('common.cancel') }</Button>
        <Button variant="primary" disabled={busy || !editName.trim()} onclick={() => void submitEdit()}>{ $t('common.save') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showDelete} title={ $t('admin.customers.detail.v2.delete_title') }>
    <div class="space-y-3 text-sm">
      <p class="text-ink-700">{ $t('admin.customers.detail.v2.delete_body') }</p>
      <Field id="del-confirm" label={ $t('admin.customers.detail.v2.type_hapus') } type="text" stacked value={deleteConfirmText} onchange={(v) => (deleteConfirmText = String(v ?? ''))} />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showDelete = false)}>{ $t('common.cancel') }</Button>
        <Button variant="danger" disabled={busy || deleteConfirmText.trim().toUpperCase() !== 'HAPUS'} onclick={() => void doDeleteCustomer()}>{ $t('admin.customers.detail.v2.delete_perm') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showLocForm} title={locEditing ? $t('admin.customers.detail.v2.loc_edit') : $t('admin.customers.detail.v2.loc_add')}>
    <div class="space-y-3">
      <Field id="l-label" label={ $t('admin.customers.locations.fields.label') } type="text" stacked value={locLabel} onchange={(v) => (locLabel = String(v ?? ''))} error={locError} />
      <Field id="l-addr" label={ $t('admin.customers.locations.columns.address') } type="text" stacked value={locAddr1} onchange={(v) => (locAddr1 = String(v ?? ''))} />
      <Field id="l-city" label={ $t('admin.customers.locations.fields.city') } type="text" stacked value={locCity} onchange={(v) => (locCity = String(v ?? ''))} />
      <Field id="l-notes" label={ $t('common.notes') } type="textarea" stacked rows={2} value={locNotes} onchange={(v) => (locNotes = String(v ?? ''))} />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showLocForm = false)}>{ $t('common.cancel') }</Button>
        <Button variant="primary" disabled={busy} onclick={() => void submitLoc()}>{ $t('common.save') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showLocDelete} title={ $t('admin.customers.detail.v2.loc_delete_title') }>
    <div class="space-y-3 text-sm">
      <p class="text-ink-700">{locDeleteTarget ? formatLocationLine(locDeleteTarget) : ''}</p>
      <p class="text-ink-500">{ $t('admin.customers.detail.v2.loc_reject_note') }</p>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showLocDelete = false)}>{ $t('common.cancel') }</Button>
        <Button variant="danger" disabled={busy} onclick={() => void doLocDelete()}>{ $t('common.delete') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showSubForm} title={subEditing ? $t('admin.customers.detail.v2.sub_edit') : $t('admin.customers.detail.v2.sub_new')}>
    <div class="space-y-3">
      <Field
        id="s-loc" label={ $t('admin.customers.tabs.locations') } type="select" stacked
        value={subLocationId}
        options={locations.map((l) => ({ value: l.id, label: l.label }))}
        onchange={(v) => (subLocationId = String(v ?? ''))}
        error={subError}
      />
      <Field
        id="s-pkg" label={ $t('common.package') } type="select" stacked
        value={subPackageId}
        options={[{ value: '', label: $t('admin.customers.detail.v2.pick_pkg') }, ...packages.map((p) => ({ value: p.id, label: `${p.name} · ${formatMoney(p.price_monthly ?? 0)}` }))]}
        onchange={(v) => (subPackageId = String(v ?? ''))}
      />
      <Field
        id="s-cycle" label={ $t('admin.customers.detail.v2.field_cycle') } type="select" stacked
        value={subCycle}
        options={[{ value: 'monthly', label: $t('admin.network.installations.monthly') }, { value: 'yearly', label: $t('admin.network.installations.yearly') }]}
        onchange={(v) => (subCycle = String(v ?? 'monthly') as 'monthly' | 'yearly')}
      />
      <Field id="s-price" label={ $t('admin.customers.detail.v2.col_price') } type="number" stacked value={subPrice} onchange={(v) => (subPrice = String(v ?? ''))} />
      {#if !subEditing}
        <Field
          id="s-status" label={ $t('admin.customers.detail.v2.initial_status') } type="select" stacked
          value={subStatus}
          options={[{ value: 'pending_installation', label: $t('admin.customers.detail.v2.waiting_install') }, { value: 'active', label: $t('admin.customers.detail.v2.activate_now') }]}
          onchange={(v) => (subStatus = String(v ?? 'pending_installation'))}
        />
      {/if}
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showSubForm = false)}>{ $t('common.cancel') }</Button>
        <Button variant="primary" disabled={busy} onclick={() => void submitSub()}>{ $t('common.save') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showSubDelete} title={ $t('admin.customers.detail.v2.sub_delete_title') }>
    <div class="space-y-3 text-sm">
      <p class="text-ink-700">{subDeleteTarget?.package_name || ''} — {subDeleteTarget ? subStatusLabel(subDeleteTarget.status) : ''}</p>
      <p class="text-ink-500">{ $t('admin.customers.detail.v2.sub_reject_note') }</p>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showSubDelete = false)}>{ $t('common.cancel') }</Button>
        <Button variant="danger" disabled={busy} onclick={() => void doSubDelete()}>{ $t('common.delete') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showChangePkg} title={ $t('admin.customers.detail.v2.chg_title') }>
    <div class="space-y-3">
      <p class="text-sm text-ink-500">{$t('admin.customers.detail.v2.chg_from', { values: { name: chgTarget?.package_name || '—' } })}</p>
      <Field
        id="chg-pkg" label={ $t('admin.customers.detail.v2.chg_new_pkg') } type="select" stacked
        value={chgNewPkg}
        options={[{ value: '', label: $t('admin.customers.detail.v2.pick_pkg') }, ...packages.filter((p) => p.id !== chgTarget?.package_id).map((p) => ({ value: p.id, label: p.name }))]}
        onchange={(v) => (chgNewPkg = String(v ?? ''))}
      />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showChangePkg = false)}>{ $t('common.cancel') }</Button>
        <Button variant="primary" disabled={busy || !chgNewPkg} onclick={() => void submitChangePkg()}>{ $t('admin.customers.detail.v2.chg_btn') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showPortalAdd} title={ $t('admin.customers.detail.v2.portal_add') }>
    <div class="space-y-3">
      <Field id="p-name" label={ $t('admin.customers.fields.name') } type="text" stacked value={portalName} onchange={(v) => (portalName = String(v ?? ''))} />
      <Field id="p-email" label={ $t('admin.customers.fields.email') } type="text" stacked value={portalEmail} onchange={(v) => (portalEmail = String(v ?? ''))} />
      <Field id="p-pass" label={ $t('admin.customers.detail.v2.portal_pass') } type="password" stacked value={portalPass} onchange={(v) => (portalPass = String(v ?? ''))} />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showPortalAdd = false)}>{ $t('common.cancel') }</Button>
        <Button variant="primary" disabled={busy} onclick={() => void addPortalUser()}>{ $t('admin.customers.detail.v2.portal_create') }</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showPortalRemove} title={ $t('admin.customers.detail.v2.portal_revoke_title') }>
    <div class="space-y-3 text-sm">
      <p class="text-ink-700">{portalRemoveTarget?.name} · {portalRemoveTarget?.email}</p>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showPortalRemove = false)}>{ $t('common.cancel') }</Button>
        <Button variant="danger" disabled={busy} onclick={() => void doPortalRemove()}>{ $t('admin.customers.invite.revoke') }</Button>
      </div>
    </div>
  </Modal>
{/if}
</AppShell>
