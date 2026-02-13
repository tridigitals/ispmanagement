<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type Customer, type CustomerLocation, type CustomerPortalUser } from '$lib/api/client';
  import type { PppoeAccountPublic } from '$lib/api/client';
  import { timeAgo } from '$lib/utils/date';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Table from '$lib/components/ui/Table.svelte';

  const customerId = $derived(String($page.params.id || ''));

  let activeTab = $state<'overview' | 'locations' | 'portal' | 'pppoe'>('overview');

  let customer = $state<Customer | null>(null);
  let loadingCustomer = $state(true);

  let locations = $state<CustomerLocation[]>([]);
  let loadingLocations = $state(false);

  let portalUsers = $state<CustomerPortalUser[]>([]);
  let loadingPortalUsers = $state(false);

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

  // Router-scoped inventory (used as suggestions for profile/pool fields)
  let pppoeProfiles = $state<any[]>([]);
  let pppoePools = $state<any[]>([]);
  let pppoeInventoryLoading = $state(false);
  let pppoeInventoryRouter = $state<string | null>(null);

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

  // Location modal
  let showAddLocation = $state(false);
  let creatingLocation = $state(false);
  let locLabel = $state('');
  let locAddress1 = $state('');
  let locAddress2 = $state('');
  let locCity = $state('');
  let locState = $state('');
  let locPostal = $state('');
  let locCountry = $state('');
  let locNotes = $state('');

  // Portal user modal
  let showAddPortalUser = $state(false);
  let creatingPortalUser = $state(false);
  let puEmail = $state('');
  let puName = $state('');
  let puPassword = $state('');

  // Deletes
  let showDeleteCustomer = $state(false);
  let deletingCustomer = $state(false);

  let showRemovePortalUser = $state(false);
  let removingPortalUser = $state(false);
  let portalUserToRemove = $state<CustomerPortalUser | null>(null);

  const locColumns = $derived.by(() => [
    { key: 'label', label: $t('admin.customers.locations.columns.label') || 'Label' },
    { key: 'address', label: $t('admin.customers.locations.columns.address') || 'Address' },
    { key: 'updated_at', label: $t('admin.customers.locations.columns.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  const portalColumns = $derived.by(() => [
    { key: 'user', label: $t('admin.customers.portal.columns.user') || 'User' },
    { key: 'created_at', label: $t('admin.customers.portal.columns.added') || 'Added' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  onMount(async () => {
    if (!$can('read', 'customers') && !$can('manage', 'customers')) {
      goto('/unauthorized');
      return;
    }
    await loadCustomer();
    await Promise.all([loadLocations(), loadPortalUsers()]);
    if ($can('read', 'pppoe') || $can('manage', 'pppoe')) {
      await loadPppoeRouters();
    }
  });

  $effect(() => {
    if (activeTab !== 'pppoe') return;
    if (!$can('read', 'pppoe') && !$can('manage', 'pppoe')) return;
    void loadPppoeAccounts();
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

  $effect(() => {
    if (!showAddPppoe && !showEditPppoe) return;
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) return;

    const rid = pppoeRouterId;
    if (!rid) {
      pppoeProfiles = [];
      pppoePools = [];
      pppoeInventoryRouter = null;
      return;
    }

    if (pppoeInventoryRouter === rid) return;
    void loadPppoeInventory(rid, { silent: true });
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

  async function loadPortalUsers() {
    if (!$can('read', 'customers') && !$can('manage', 'customers')) return;
    loadingPortalUsers = true;
    try {
      portalUsers = await api.customers.portalUsers.list(customerId);
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.portal.toasts.load_failed') ||
          `Failed to load portal users: ${e?.message || e}`,
      );
    } finally {
      loadingPortalUsers = false;
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
    pppoeRouterProfileName = '';
    pppoeRemoteAddress = '';
    pppoeAddressPool = '';
    pppoeDisabled = false;
    pppoeComment = '';
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

  async function createPortalUser() {
    if (!puEmail.trim() || !puName.trim() || !puPassword) return;
    creatingPortalUser = true;
    try {
      await api.customers.portalUsers.createNew({
        customer_id: customerId,
        email: puEmail.trim(),
        name: puName.trim(),
        password: puPassword,
      });
      showAddPortalUser = false;
      puEmail = '';
      puName = '';
      puPassword = '';
      await loadPortalUsers();
      toast.success(get(t)('admin.customers.portal.toasts.created') || 'Portal user added');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.portal.toasts.create_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    } finally {
      creatingPortalUser = false;
    }
  }

  function confirmRemovePortalUser(row: CustomerPortalUser) {
    portalUserToRemove = row;
    showRemovePortalUser = true;
  }

  async function doRemovePortalUser() {
    const row = portalUserToRemove;
    if (!row) return;
    removingPortalUser = true;
    try {
      await api.customers.portalUsers.remove(row.customer_user_id);
      showRemovePortalUser = false;
      portalUserToRemove = null;
      await loadPortalUsers();
      toast.success(get(t)('admin.customers.portal.toasts.removed') || 'Portal user removed');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.portal.toasts.remove_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    } finally {
      removingPortalUser = false;
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
  <div class="page-header">
    <div class="title">
      <button class="btn btn-secondary" onclick={() => goto('..')}>
        <Icon name="arrow-left" size={16} />
        {$t('common.back') || 'Back'}
      </button>
      <div class="meta">
        <h1>{customer?.name || $t('admin.customers.detail.title') || 'Customer'}</h1>
        <p class="subtitle">
          {customer?.email || customer?.phone || ($t('admin.customers.detail.subtitle') || 'Customer details')}
        </p>
      </div>
    </div>
    <div class="header-actions">
      <button
        class="btn btn-secondary"
        onclick={() => Promise.all([loadCustomer(), loadLocations(), loadPortalUsers()])}
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

  <div class="tabs">
    <button class:active={activeTab === 'overview'} onclick={() => (activeTab = 'overview')}>
      {$t('admin.customers.tabs.overview') || 'Overview'}
    </button>
    <button class:active={activeTab === 'locations'} onclick={() => (activeTab = 'locations')}>
      {$t('admin.customers.tabs.locations') || 'Locations'}
    </button>
    <button class:active={activeTab === 'portal'} onclick={() => (activeTab = 'portal')}>
      {$t('admin.customers.tabs.portal') || 'Portal users'}
    </button>
    {#if $can('read', 'pppoe') || $can('manage', 'pppoe')}
      <button class:active={activeTab === 'pppoe'} onclick={() => (activeTab = 'pppoe')}>
        {$t('admin.customers.tabs.pppoe') || 'PPPoE'}
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
          <h3>{$t('admin.customers.overview.title') || 'Customer profile'}</h3>
          {#if $can('manage', 'customers')}
            <button class="btn btn-primary" onclick={saveOverview} disabled={saving || !name.trim()}>
              <Icon name="check-circle" size={16} />
              {$t('common.save') || 'Save'}
            </button>
          {/if}
        </div>

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
              rows="4"
              bind:value={notes}
              disabled={!$can('manage', 'customers')}
            ></textarea>
          </label>
          <label class="row">
            <input type="checkbox" bind:checked={isActive} disabled={!$can('manage', 'customers')} />
            <span>{$t('admin.customers.fields.active') || 'Active'}</span>
          </label>
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
            <button class="btn btn-primary" onclick={() => (showAddLocation = true)}>
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
              </div>
            {:else}
              {item[key] ?? ''}
            {/if}
          {/snippet}
        </Table>
      </div>
    {:else if activeTab === 'portal'}
      <div class="card section">
        <div class="section-head">
          <div>
            <h3>{$t('admin.customers.portal.title') || 'Portal users'}</h3>
            <p class="subtitle">
              {$t('admin.customers.portal.subtitle') || 'Users who can login to the customer portal.'}
            </p>
          </div>
          {#if $can('manage', 'customers')}
            <button class="btn btn-primary" onclick={() => (showAddPortalUser = true)}>
              <Icon name="plus" size={16} />
              {$t('admin.customers.portal.actions.add') || 'Add user'}
            </button>
          {/if}
        </div>

        <Table
          columns={portalColumns}
          data={portalUsers}
          loading={loadingPortalUsers}
          emptyText={$t('admin.customers.portal.empty') || 'No portal users yet.'}
          pagination
        >
          {#snippet cell({ item, key })}
            {@const row = item as CustomerPortalUser}
            {#if key === 'user'}
              <div class="name">{row.name}</div>
              <div class="sub">{row.email}</div>
            {:else if key === 'created_at'}
              <span class="mono">{new Date(row.created_at).toLocaleString()}</span>
            {:else if key === 'actions'}
              <div class="row-actions">
                {#if $can('manage', 'customers')}
                  <button class="btn-icon danger" title={$t('common.remove') || 'Remove'} onclick={() => confirmRemovePortalUser(row)}>
                    <Icon name="x" size={16} />
                  </button>
                {/if}
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
                    <Icon name="edit-3" size={16} />
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
        <select class="input" bind:value={pppoeRouterId} disabled={loadingPppoeRouters}>
          <option value="">{($t('common.select') || 'Select') + '...'}</option>
          {#each pppoeRouters as r}
            <option value={r.id}>{r.name}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.location') || 'Location'}</span>
        <select class="input" bind:value={pppoeLocationId}>
          <option value="">{($t('common.select') || 'Select') + '...'}</option>
          {#each locations as l}
            <option value={l.id}>{l.label}</option>
          {/each}
        </select>
      </label>
    </div>

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
        <input
          class="input"
          bind:value={pppoeRouterProfileName}
          list="pppoe-profile-suggestions"
          placeholder="default / paket-10mbps"
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.remote_address') || 'Remote IP / Pool'}</span>
        <input
          class="input"
          bind:value={pppoeRemoteAddress}
          list="pppoe-pool-suggestions"
          placeholder="10.10.10.10 / pool-pppoe"
        />
      </label>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={pppoeComment} />
    </label>

    <label class="row">
      <input type="checkbox" bind:checked={pppoeDisabled} />
      <span>{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</span>
    </label>

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
        <input class="input" bind:value={pppoeRouterProfileName} list="pppoe-profile-suggestions" />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.remote_address') || 'Remote IP / Pool'}</span>
        <input class="input" bind:value={pppoeRemoteAddress} list="pppoe-pool-suggestions" />
      </label>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={pppoeComment} />
    </label>

    <label class="row">
      <input type="checkbox" bind:checked={pppoeDisabled} />
      <span>{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</span>
    </label>

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

<datalist id="pppoe-profile-suggestions">
  {#each pppoeProfiles as p}
    <option value={p.name}></option>
  {/each}
</datalist>

<datalist id="pppoe-pool-suggestions">
  {#each pppoePools as p}
    <option value={p.name}></option>
  {/each}
</datalist>

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
  show={showAddPortalUser}
  title={$t('admin.customers.portal.new.title') || 'Add portal user'}
  onclose={() => (showAddPortalUser = false)}
>
  <div class="form">
    <div class="callout">
      <Icon name="info" size={18} />
      <span>
        {$t('admin.customers.portal.new.hint') ||
          'This user will be linked to the customer and can login to the dashboard.'}
      </span>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.portal.fields.email') || 'Email'}</span>
        <input class="input" bind:value={puEmail} placeholder="customer.user@example.com" />
      </label>
      <label>
        <span>{$t('admin.customers.portal.fields.name') || 'Name'}</span>
        <input class="input" bind:value={puName} placeholder="Customer User" />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.portal.fields.password') || 'Password'}</span>
      <input class="input" type="password" bind:value={puPassword} />
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showAddPortalUser = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={createPortalUser}
        disabled={creatingPortalUser || !puEmail.trim() || !puName.trim() || !puPassword}
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
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
  show={showRemovePortalUser}
  title={$t('admin.customers.portal.remove.title') || 'Remove portal user'}
  message={$t('admin.customers.portal.remove.message') || 'This will unlink the user from the customer.'}
  confirmText={$t('common.remove') || 'Remove'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={removingPortalUser}
  onconfirm={doRemovePortalUser}
  oncancel={() => (showRemovePortalUser = false)}
/>

<style>
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

  .meta h1 {
    margin: 0;
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
    padding: 1.25rem;
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

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
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

  .name {
    font-weight: 650;
  }

  .sub {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-top: 0.15rem;
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

  @keyframes spin {
    from {
      transform: rotate(0);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
    .grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
