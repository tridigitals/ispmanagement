<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import {
    api,
    type Customer,
    type CustomerListItem,
    type CustomerRegistrationInvitePolicy,
    type CustomerRegistrationInviteSummary,
    type CustomerRegistrationInviteView,
    type CustomerSummary,
    type PaginatedResponse,
  } from '$lib/api/client';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  const IMPORT_PLACEHOLDER_CUSTOMER_NAME = 'Imported (Unassigned)';

  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.customers.columns.customer') || 'Customer' },
    { key: 'contact', label: $t('admin.customers.columns.contact') || 'Contact' },
    { key: 'status', label: $t('admin.customers.columns.status') || 'Status' },
    { key: 'health', label: 'Health' },
    { key: 'service', label: 'Service' },
    { key: 'updated_at', label: $t('admin.customers.columns.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  type CustomerStatusFilter = 'all' | 'active' | 'inactive';
  type CustomerServiceFilter = 'all' | 'active' | 'inactive' | 'none';

  let customers = $state<CustomerListItem[]>([]);
  let total = $state(0);
  let customerSummary = $state<CustomerSummary>({ total: 0, active: 0, inactive: 0 });
  let loading = $state(true);
  let error = $state('');

  let q = $state('');
  let statusFilter = $state<CustomerStatusFilter>('all');
  let serviceFilter = $state<CustomerServiceFilter>('all');
  let page = $state(0); // Table is 0-based
  let perPage = $state(10);

  let showCreate = $state(false);
  let creating = $state(false);
  let createName = $state('');
  let createEmail = $state('');
  let createPhone = $state('');
  let createNotes = $state('');
  let createPortalPassword = $state('');
  let createPortalPasswordConfirm = $state('');

  let showDelete = $state(false);
  let deleting = $state(false);
  let deleteTarget = $state<Customer | null>(null);

  let showInviteModal = $state(false);
  let inviteGenerating = $state(false);
  let inviteLoading = $state(false);
  let inviteRevokingId = $state<string | null>(null);
  let inviteExpiresInHours = $state(24);
  let inviteMaxUses = $state(1);
  let inviteNote = $state('');
  let inviteIncludeInactive = $state(true);
  let inviteRows = $state<CustomerRegistrationInviteView[]>([]);
  let generatedInviteUrl = $state('');
  let generatedInviteExpiresAt = $state('');
  let invitePolicyLoading = $state(false);
  let invitePolicySaving = $state(false);
  let inviteSummaryLoading = $state(false);
  let invitePolicyExpiresInHours = $state(24);
  let invitePolicyMaxUses = $state(1);
  let inviteSummary = $state<CustomerRegistrationInviteSummary | null>(null);
  const canReadCustomers = $derived($can('read', 'customers') || $can('manage', 'customers'));
  const canManageCustomers = $derived($can('manage', 'customers'));

  let stats = $derived({
    total: customerSummary.total,
    active: customerSummary.active,
    inactive: customerSummary.inactive,
  });
  let totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  onMount(async () => {
    if (!canReadCustomers) {
      goto('/unauthorized');
      return;
    }
    hydrateUrlState();
    await Promise.all([
      canManageCustomers ? loadInvites() : Promise.resolve(),
      load(),
      loadCustomerSummary(),
    ]);
  });

  function isCustomerStatusFilter(value: string | null): value is CustomerStatusFilter {
    return value === 'all' || value === 'active' || value === 'inactive';
  }

  function isCustomerServiceFilter(value: string | null): value is CustomerServiceFilter {
    return value === 'all' || value === 'active' || value === 'inactive' || value === 'none';
  }

  function hydrateUrlState() {
    if (typeof window === 'undefined') return;

    const params = new URLSearchParams(window.location.search);
    const nextStatus = params.get('status');
    const nextService = params.get('service');
    q = params.get('q') || '';
    statusFilter = isCustomerStatusFilter(nextStatus) ? nextStatus : 'all';
    serviceFilter = isCustomerServiceFilter(nextService) ? nextService : 'all';
    page = Math.max(0, Number(params.get('page') || '1') - 1);
    perPage = Math.max(1, Number(params.get('per_page') || perPage));
  }

  function syncUrlState() {
    if (typeof window === 'undefined') return;

    const url = new URL(window.location.href);
    if (q.trim()) url.searchParams.set('q', q.trim());
    else url.searchParams.delete('q');

    if (statusFilter !== 'all') url.searchParams.set('status', statusFilter);
    else url.searchParams.delete('status');

    if (serviceFilter !== 'all') url.searchParams.set('service', serviceFilter);
    else url.searchParams.delete('service');

    if (page > 0) url.searchParams.set('page', String(page + 1));
    else url.searchParams.delete('page');

    if (perPage !== 10) url.searchParams.set('per_page', String(perPage));
    else url.searchParams.delete('per_page');

    window.history.replaceState(window.history.state, '', url);
  }

  async function refreshCustomers(options: { sync?: boolean } = {}) {
    if (options.sync !== false) syncUrlState();
    await Promise.all([load(), loadCustomerSummary()]);
  }

  async function setStatusFilter(next: CustomerStatusFilter) {
    if (statusFilter === next) return;
    statusFilter = next;
    page = 0;
    await refreshCustomers();
  }

  async function setServiceFilter(next: CustomerServiceFilter) {
    if (serviceFilter === next) return;
    serviceFilter = next;
    page = 0;
    await refreshCustomers();
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const res: PaginatedResponse<CustomerListItem> = await api.customers.list({
        q,
        status: statusFilter,
        service: serviceFilter,
        page: page + 1,
        perPage,
      });
      customers = res.data;
      total = res.total;
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load customers');
      toast.error(get(t)('admin.customers.toasts.load_failed') || 'Failed to load customers');
    } finally {
      loading = false;
    }
  }

  async function loadCustomerSummary() {
    try {
      customerSummary = await api.customers.summary();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load customer summary');
    }
  }

  function isSystemImportPlaceholder(customer: Customer): boolean {
    return customer.name === IMPORT_PLACEHOLDER_CUSTOMER_NAME;
  }

  function openCustomer(c: Customer) {
    const base = $pageStore.url.pathname.replace(/\/$/, '');
    goto(`${base}/${c.id}`);
  }

  function serviceStatusLabel(c: CustomerListItem) {
    if (c.service_status === 'active') return `${c.active_subscriptions} active`;
    if (c.service_status === 'inactive') return `${c.subscription_count} inactive`;
    return 'No service';
  }

  function customerHealthLabel(c: CustomerListItem) {
    if (!c.is_active) return 'Inactive';
    if (c.service_status === 'none') return 'No service';
    if (c.service_status === 'inactive') return 'Service inactive';
    return 'Healthy';
  }

  function customerHealthTone(c: CustomerListItem) {
    if (!c.is_active) return 'muted';
    if (c.service_status === 'none' || c.service_status === 'inactive') return 'warning';
    return 'healthy';
  }

  async function goToMobilePage(nextPage: number) {
    page = Math.min(totalPages - 1, Math.max(0, nextPage));
    await refreshCustomers();
  }

  function adminBasePath() {
    return $pageStore.url.pathname.replace(/\/admin\/customers\/?$/, '/admin');
  }

  function openAddService(c: CustomerListItem) {
    const base = $pageStore.url.pathname.replace(/\/$/, '');
    goto(`${base}/${c.id}#subscriptions`);
  }

  function openCreateInvoice(c: CustomerListItem) {
    goto(`${adminBasePath()}/invoices?customer_id=${encodeURIComponent(c.id)}`);
  }

  function openWhatsApp(c: CustomerListItem) {
    if (!c.phone) {
      toast.error('Customer phone is not set');
      return;
    }
    const digits = c.phone.replace(/[^\d+]/g, '');
    window.open(`https://wa.me/${digits.replace(/^\+/, '')}`, '_blank', 'noopener,noreferrer');
  }

  async function createCustomer() {
    if (!createName.trim()) return;
    if (!createEmail.trim()) {
      toast.error(
        get(t)('admin.customers.new.portal.validation.email_required') ||
          'Email wajib diisi jika ingin membuat akun login.',
      );
      return;
    }
    if (!createPortalPassword || createPortalPassword.length < 6) {
      toast.error(
        get(t)('admin.customers.new.portal.validation.password_min') ||
          'Password minimal 6 karakter.',
      );
      return;
    }
    if (createPortalPassword !== createPortalPasswordConfirm) {
      toast.error(
        get(t)('admin.customers.new.portal.validation.password_mismatch') ||
          'Konfirmasi password tidak sama.',
      );
      return;
    }
    creating = true;
    try {
      await api.customers.createWithPortal({
        name: createName.trim(),
        email: createEmail.trim() || null,
        phone: createPhone.trim() || null,
        notes: createNotes.trim() || null,
        portal_email: createEmail.trim(),
        portal_name: createName.trim(),
        portal_password: createPortalPassword,
      });

      showCreate = false;
      createName = '';
      createEmail = '';
      createPhone = '';
      createNotes = '';
      createPortalPassword = '';
      createPortalPasswordConfirm = '';
      page = 0;
      await refreshCustomers();
      toast.success(get(t)('admin.customers.toasts.created') || 'Customer created');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.toasts.create_failed', { values: { message: e?.message || e } }) ||
          `Failed to create customer: ${e?.message || e}`,
      );
    } finally {
      creating = false;
    }
  }

  function confirmDelete(c: Customer) {
    deleteTarget = c;
    showDelete = true;
  }

  async function doDelete() {
    const c = deleteTarget;
    if (!c) return;
    deleting = true;
    try {
      await api.customers.delete(c.id);
      showDelete = false;
      deleteTarget = null;
      await refreshCustomers();
      toast.success(get(t)('admin.customers.toasts.deleted') || 'Customer deleted');
    } catch (e: any) {
      toast.error(
        get(t)('admin.customers.toasts.delete_failed', { values: { message: e?.message || e } }) ||
          `Failed to delete: ${e?.message || e}`,
      );
    } finally {
      deleting = false;
    }
  }

  function isInviteExpired(invite: CustomerRegistrationInviteView) {
    const ts = new Date(invite.expires_at).getTime();
    return Number.isFinite(ts) && ts <= Date.now();
  }

  function isInviteUsedOut(invite: CustomerRegistrationInviteView) {
    return invite.used_count >= invite.max_uses;
  }

  function inviteStatus(invite: CustomerRegistrationInviteView) {
    if (invite.is_revoked) return 'revoked';
    if (isInviteUsedOut(invite)) return 'used';
    if (isInviteExpired(invite)) return 'expired';
    return 'active';
  }

  function inviteStatusLabel(invite: CustomerRegistrationInviteView) {
    const s = inviteStatus(invite);
    if (s === 'revoked') return 'Revoked';
    if (s === 'used') return 'Used';
    if (s === 'expired') return 'Expired';
    return 'Active';
  }

  async function loadInvites() {
    if (!canManageCustomers) return;
    inviteLoading = true;
    try {
      inviteRows = await api.customers.invites.list({
        include_inactive: inviteIncludeInactive,
        limit: 50,
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load customer invite links');
    } finally {
      inviteLoading = false;
    }
  }

  async function loadInvitePolicy() {
    if (!canManageCustomers) return;
    invitePolicyLoading = true;
    try {
      const policy: CustomerRegistrationInvitePolicy = await api.customers.invites.getPolicy();
      invitePolicyExpiresInHours = policy.default_expires_in_hours || 24;
      invitePolicyMaxUses = policy.default_max_uses || 1;
      inviteExpiresInHours = invitePolicyExpiresInHours;
      inviteMaxUses = invitePolicyMaxUses;
    } catch (e: any) {
      const msg = String(e?.message || '');
      const isMissingEndpoint = msg.includes('404') || msg.toLowerCase().includes('not found');
      if (!isMissingEndpoint) {
        toast.error(msg || 'Failed to load invite defaults');
      }
    } finally {
      invitePolicyLoading = false;
    }
  }

  async function saveInvitePolicy() {
    if (invitePolicySaving) return;
    invitePolicySaving = true;
    try {
      const nextExpires = Math.min(720, Math.max(1, Math.trunc(invitePolicyExpiresInHours || 24)));
      const nextMaxUses = Math.min(100, Math.max(1, Math.trunc(invitePolicyMaxUses || 1)));
      const policy = await api.customers.invites.updatePolicy({
        default_expires_in_hours: nextExpires,
        default_max_uses: nextMaxUses,
      });
      invitePolicyExpiresInHours = policy.default_expires_in_hours;
      invitePolicyMaxUses = policy.default_max_uses;
      inviteExpiresInHours = policy.default_expires_in_hours;
      inviteMaxUses = policy.default_max_uses;
      toast.success('Invite defaults updated');
    } catch (e: any) {
      toast.error(e?.message || 'Failed to update invite defaults');
    } finally {
      invitePolicySaving = false;
    }
  }

  async function loadInviteSummary() {
    if (!canManageCustomers) return;
    inviteSummaryLoading = true;
    try {
      inviteSummary = await api.customers.invites.summary();
    } catch (e: any) {
      const msg = String(e?.message || '');
      const isMissingEndpoint = msg.includes('404') || msg.toLowerCase().includes('not found');
      if (!isMissingEndpoint) {
        toast.error(msg || 'Failed to load invite summary');
      }
    } finally {
      inviteSummaryLoading = false;
    }
  }

  function openInviteModal() {
    showInviteModal = true;
    generatedInviteUrl = '';
    generatedInviteExpiresAt = '';
    inviteExpiresInHours = invitePolicyExpiresInHours || 24;
    inviteMaxUses = invitePolicyMaxUses || 1;
    inviteNote = '';
    void Promise.all([loadInvitePolicy(), loadInviteSummary(), loadInvites()]);
  }

  async function generateInvite() {
    if (inviteGenerating) return;
    inviteGenerating = true;
    try {
      const nextExpires = Math.min(720, Math.max(1, Math.trunc(inviteExpiresInHours || 24)));
      const nextMaxUses = Math.min(100, Math.max(1, Math.trunc(inviteMaxUses || 1)));
      const res = await api.customers.invites.create({
        expires_in_hours: nextExpires,
        max_uses: nextMaxUses,
        note: inviteNote.trim() || null,
      });
      inviteExpiresInHours = nextExpires;
      inviteMaxUses = nextMaxUses;
      generatedInviteUrl = res.invite_url;
      generatedInviteExpiresAt = res.invite.expires_at;
      inviteNote = '';
      toast.success('Invite link generated');
      await Promise.all([loadInvites(), loadInviteSummary()]);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to generate invite link');
    } finally {
      inviteGenerating = false;
    }
  }

  async function revokeInvite(inviteId: string) {
    if (!inviteId || inviteRevokingId) return;
    inviteRevokingId = inviteId;
    try {
      await api.customers.invites.revoke(inviteId);
      toast.success('Invite link revoked');
      await Promise.all([loadInvites(), loadInviteSummary()]);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to revoke invite');
    } finally {
      inviteRevokingId = null;
    }
  }

  async function copyInviteLink(link: string) {
    if (!link) return;
    try {
      await navigator.clipboard.writeText(link);
      toast.success(get(t)('common.copied') || 'Copied');
    } catch {
      toast.error(get(t)('common.copy_failed') || 'Copy failed');
    }
  }
</script>

<div class="page-content fade-in">
  <div class="page-header">
    <div>
      <h1>{$t('admin.customers.title') || 'Customers'}</h1>
      <p class="subtitle">
        {$t('admin.customers.subtitle') || 'Manage customer accounts and service locations.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={() => refreshCustomers()} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if canManageCustomers}
        <button class="btn btn-secondary" onclick={openInviteModal}>
          <Icon name="link" size={16} />
          Invite Link
        </button>
        <button class="btn btn-primary" onclick={() => (showCreate = true)}>
          <Icon name="plus" size={16} />
          {$t('admin.customers.actions.new') || 'New customer'}
        </button>
      {/if}
    </div>
  </div>

  <div class="stats-grid">
    <button class="stat-filter" class:active={statusFilter === 'all'} onclick={() => setStatusFilter('all')}>
      <StatsCard
        title={$t('admin.customers.stats.total') || 'Total'}
        value={stats.total}
        icon="users"
        color="blue"
      />
    </button>
    <button
      class="stat-filter"
      class:active={statusFilter === 'active'}
      onclick={() => setStatusFilter('active')}
    >
      <StatsCard
        title={$t('admin.customers.stats.active') || 'Active'}
        value={stats.active}
        icon="check-circle"
        color="green"
      />
    </button>
    <button
      class="stat-filter"
      class:active={statusFilter === 'inactive'}
      onclick={() => setStatusFilter('inactive')}
    >
      <StatsCard
        title={$t('admin.customers.stats.inactive') || 'Inactive'}
        value={stats.inactive}
        icon="x-circle"
        color="orange"
      />
    </button>
  </div>

  <div class="card table-card">
    <TableToolbar
      bind:searchQuery={q}
      placeholder={$t('admin.customers.search') || 'Search customers...'}
      onsearch={() => {
        page = 0;
        refreshCustomers();
      }}
    >
      {#snippet filters()}
        <div class="toolbar-filters">
          <label class="customer-filter-field">
            <span>Status</span>
            <select
              class="customer-filter-select"
              aria-label="Customer status filter"
              value={statusFilter}
              onchange={(event) =>
                setStatusFilter((event.currentTarget as HTMLSelectElement).value as CustomerStatusFilter)}
            >
              <option value="all">All customers</option>
              <option value="active">Active</option>
              <option value="inactive">Inactive</option>
            </select>
          </label>
          <label class="customer-filter-field">
            <span>Service</span>
            <select
              class="customer-filter-select"
              aria-label="Customer service filter"
              value={serviceFilter}
              onchange={(event) =>
                setServiceFilter((event.currentTarget as HTMLSelectElement).value as CustomerServiceFilter)}
            >
              <option value="all">All services</option>
              <option value="active">Active service</option>
              <option value="inactive">Inactive service</option>
              <option value="none">No service</option>
            </select>
          </label>
        </div>
      {/snippet}
      {#snippet actions()}
        <span class="muted">
          {total}
          {$t('admin.customers.results') || 'results'}
        </span>
      {/snippet}
    </TableToolbar>

    {#if error}
      <div class="error-banner">
        <Icon name="alert-triangle" size={18} />
        <span>{error}</span>
      </div>
    {/if}

    <div class="mobile-customer-list">
      {#if loading}
        <div class="mobile-empty">{$t('common.loading') || 'Loading...'}</div>
      {:else if customers.length === 0}
        <div class="mobile-empty">{$t('admin.customers.empty') || 'No customers yet.'}</div>
      {:else}
        {#each customers as c (c.id)}
          <article class="mobile-customer-card">
            <div class="mobile-customer-head">
              <button class="linkish" onclick={() => openCustomer(c)} disabled={isSystemImportPlaceholder(c)}>
                <div class="name">{c.name}</div>
                <div class="sub">{c.email || c.phone || 'No contact'}</div>
              </button>
              <span
                class="pill"
                class:pill-green={customerHealthTone(c) === 'healthy'}
                class:pill-warning={customerHealthTone(c) === 'warning'}
                class:pill-gray={customerHealthTone(c) === 'muted'}
              >
                {customerHealthLabel(c)}
              </span>
            </div>
            <div class="mobile-customer-meta">
              <span>{c.email || '—'}</span>
              <span>{c.phone || '—'}</span>
              <span>{serviceStatusLabel(c)}</span>
            </div>
            <div class="mobile-customer-actions">
              {#if !isSystemImportPlaceholder(c)}
                <button class="btn btn-secondary" onclick={() => openCustomer(c)}>
                  <Icon name="arrow-right" size={15} />
                  Open
                </button>
              {/if}
              {#if canManageCustomers && !isSystemImportPlaceholder(c)}
                <button class="btn-icon" title="Add service" onclick={() => openAddService(c)}>
                  <Icon name="wifi" size={16} />
                </button>
                <button class="btn-icon" title="Create invoice" onclick={() => openCreateInvoice(c)}>
                  <Icon name="receipt" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title="Send WhatsApp"
                  disabled={!c.phone}
                  onclick={() => openWhatsApp(c)}
                >
                  <Icon name="message-circle" size={16} />
                </button>
                <button
                  class="btn-icon danger"
                  title={$t('common.delete') || 'Delete'}
                  onclick={() => confirmDelete(c)}
                >
                  <Icon name="trash-2" size={16} />
                </button>
              {/if}
            </div>
          </article>
        {/each}
        <div class="mobile-pager">
          <button class="btn btn-secondary" disabled={page === 0} onclick={() => goToMobilePage(page - 1)}>
            <Icon name="chevron-left" size={16} />
            Prev
          </button>
          <span class="mono">{page + 1} / {totalPages}</span>
          <button
            class="btn btn-secondary"
            disabled={page + 1 >= totalPages}
            onclick={() => goToMobilePage(page + 1)}
          >
            Next
            <Icon name="chevron-right" size={16} />
          </button>
        </div>
      {/if}
    </div>

    <div class="desktop-customer-table">
      <Table
        {columns}
        data={customers}
        keyField="id"
        {loading}
        emptyText={$t('admin.customers.empty') || 'No customers yet.'}
        pagination
        serverSide
        pageSize={perPage}
        count={total}
        onchange={(p) => {
          page = p;
          refreshCustomers();
        }}
        onpageSizeChange={(s) => {
          perPage = s;
          page = 0;
          refreshCustomers();
        }}
      >
        {#snippet cell({ item, key })}
          {@const c = item as CustomerListItem}
          {#if key === 'name'}
            {#if isSystemImportPlaceholder(c)}
              <div>
                <div class="name">{c.name}</div>
                <div class="sub">
                  {$t('admin.network.pppoe.import.fields.unassigned') || 'Unassigned'}
                </div>
              </div>
            {:else}
              <button class="linkish" onclick={() => openCustomer(c)}>
                <div class="name">{c.name}</div>
                <div class="sub">{c.email || c.phone || ''}</div>
              </button>
            {/if}
          {:else if key === 'contact'}
            <div class="contact">
              <div>{c.email || '—'}</div>
              <div class="sub">{c.phone || '—'}</div>
            </div>
          {:else if key === 'status'}
            {#if c.is_active}
              <span class="pill pill-green">{$t('common.active') || 'Active'}</span>
            {:else}
              <span class="pill pill-gray">{$t('common.inactive') || 'Inactive'}</span>
            {/if}
          {:else if key === 'health'}
            <span
              class="pill"
              class:pill-green={customerHealthTone(c) === 'healthy'}
              class:pill-warning={customerHealthTone(c) === 'warning'}
              class:pill-gray={customerHealthTone(c) === 'muted'}
            >
              {customerHealthLabel(c)}
            </span>
          {:else if key === 'service'}
            <div>
              <span
                class="pill"
                class:pill-green={c.service_status === 'active'}
                class:pill-gray={c.service_status === 'none'}
              >
                {serviceStatusLabel(c)}
              </span>
              {#if c.subscription_count > 0 && c.active_subscriptions !== c.subscription_count}
                <div class="sub">{c.subscription_count} total services</div>
              {/if}
            </div>
          {:else if key === 'updated_at'}
            <span class="mono">{new Date(c.updated_at).toLocaleString()}</span>
          {:else if key === 'actions'}
            <div class="row-actions">
              {#if !isSystemImportPlaceholder(c)}
                <button
                  class="btn-icon"
                  title={$t('common.open') || 'Open'}
                  onclick={() => openCustomer(c)}
                >
                  <Icon name="arrow-right" size={16} />
                </button>
              {/if}
              {#if canManageCustomers && !isSystemImportPlaceholder(c)}
                <button class="btn-icon" title="Add service" onclick={() => openAddService(c)}>
                  <Icon name="wifi" size={16} />
                </button>
                <button class="btn-icon" title="Create invoice" onclick={() => openCreateInvoice(c)}>
                  <Icon name="receipt" size={16} />
                </button>
                <button
                  class="btn-icon"
                  title="Send WhatsApp"
                  disabled={!c.phone}
                  onclick={() => openWhatsApp(c)}
                >
                  <Icon name="message-circle" size={16} />
                </button>
                <button
                  class="btn-icon danger"
                  title={$t('common.delete') || 'Delete'}
                  onclick={() => confirmDelete(c)}
                >
                  <Icon name="trash-2" size={16} />
                </button>
              {:else if isSystemImportPlaceholder(c)}
                <span class="mono">—</span>
              {/if}
            </div>
          {:else}
            {item[key] ?? ''}
          {/if}
        {/snippet}
      </Table>
    </div>
  </div>
</div>

<Modal
  show={showCreate}
  title={$t('admin.customers.new.title') || 'New customer'}
  onclose={() => (showCreate = false)}
>
  <div class="form">
    <label>
      <span>{$t('admin.customers.fields.name') || 'Name'}</span>
      <input class="input" bind:value={createName} placeholder="PT Example" />
    </label>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.fields.email') || 'Email'}</span>
        <input class="input" bind:value={createEmail} placeholder="customer@example.com" />
      </label>
      <label>
        <span>{$t('admin.customers.fields.phone') || 'Phone'}</span>
        <input class="input" bind:value={createPhone} placeholder="+62..." />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="4" bind:value={createNotes}></textarea>
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.new.portal.password') || 'Password'}</span>
        <input class="input" type="text" bind:value={createPortalPassword} />
      </label>
      <label>
        <span>{$t('admin.customers.new.portal.password_confirm') || 'Confirm password'}</span>
        <input class="input" type="text" bind:value={createPortalPasswordConfirm} />
      </label>
    </div>

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showCreate = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={createCustomer}
        disabled={creating ||
          !createName.trim() ||
          !createEmail.trim() ||
          !createPortalPassword ||
          !createPortalPasswordConfirm ||
          createPortalPassword !== createPortalPasswordConfirm}
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showInviteModal}
  title="Customer Invite Link"
  onclose={() => (showInviteModal = false)}
>
  <div class="form">
    <section class="invite-section">
      <div class="invite-section-head">
        <strong>Default policy (tenant)</strong>
        {#if invitePolicyLoading}
          <span class="muted">{$t('common.loading') || 'Loading...'}</span>
        {/if}
      </div>
      <div class="grid2">
        <label>
          <span>Default expiry (hours)</span>
          <input
            class="input"
            type="number"
            min="1"
            max="720"
            bind:value={invitePolicyExpiresInHours}
          />
        </label>
        <label>
          <span>Default max uses</span>
          <input class="input" type="number" min="1" max="100" bind:value={invitePolicyMaxUses} />
        </label>
      </div>
      <div class="actions actions-inline">
        <button class="btn btn-secondary" onclick={saveInvitePolicy} disabled={invitePolicySaving}>
          <Icon name="save" size={14} />
          {invitePolicySaving ? 'Saving...' : 'Save defaults'}
        </button>
      </div>
    </section>

    <section class="invite-section">
      <div class="invite-section-head">
        <strong>Invite summary</strong>
      </div>
      {#if inviteSummaryLoading}
        <div class="muted">{$t('common.loading') || 'Loading...'}</div>
      {:else if inviteSummary}
        <div class="invite-summary-grid">
          <div class="invite-summary-item">
            <small>Total</small>
            <strong>{inviteSummary.total}</strong>
          </div>
          <div class="invite-summary-item">
            <small>Active</small>
            <strong>{inviteSummary.active}</strong>
          </div>
          <div class="invite-summary-item">
            <small>Used up</small>
            <strong>{inviteSummary.used_up}</strong>
          </div>
          <div class="invite-summary-item">
            <small>Expired</small>
            <strong>{inviteSummary.expired}</strong>
          </div>
          <div class="invite-summary-item">
            <small>Revoked</small>
            <strong>{inviteSummary.revoked}</strong>
          </div>
          <div class="invite-summary-item">
            <small>Utilization</small>
            <strong>{inviteSummary.utilization_percent.toFixed(1)}%</strong>
          </div>
        </div>
      {/if}
    </section>

    <section class="invite-section">
      <div class="invite-section-head">
        <strong>Generate invite</strong>
      </div>
      <div class="grid2">
        <label>
          <span>Expire (hours)</span>
          <input class="input" type="number" min="1" max="720" bind:value={inviteExpiresInHours} />
        </label>
        <label>
          <span>Max uses</span>
          <input class="input" type="number" min="1" max="100" bind:value={inviteMaxUses} />
        </label>
      </div>
      <label>
        <span>Note (optional)</span>
        <input class="input" bind:value={inviteNote} placeholder="Campaign/remark" />
      </label>

      <div class="actions actions-inline">
        <button class="btn btn-primary" onclick={generateInvite} disabled={inviteGenerating}>
          <Icon name="plus" size={16} />
          {inviteGenerating ? 'Generating...' : 'Generate Invite Link'}
        </button>
      </div>
    </section>

    {#if generatedInviteUrl}
      <div class="invite-result">
        <div class="invite-result-head">
          <strong>Generated link</strong>
          <small class="sub">
            Expires: {new Date(generatedInviteExpiresAt).toLocaleString()}
          </small>
        </div>
        <div class="invite-copy-row">
          <input class="input mono" readonly value={generatedInviteUrl} />
          <button class="btn btn-secondary" onclick={() => copyInviteLink(generatedInviteUrl)}>
            <Icon name="link" size={16} />
            {$t('common.copy') || 'Copy'}
          </button>
        </div>
      </div>
    {/if}

    <div class="invite-list-head">
      <strong>Recent invite links</strong>
      <label class="inline-check">
        <input
          type="checkbox"
          bind:checked={inviteIncludeInactive}
          onchange={() => loadInvites()}
        />
        <span>Show inactive</span>
      </label>
    </div>

    {#if inviteLoading}
      <div class="muted">{$t('common.loading') || 'Loading...'}</div>
    {:else if inviteRows.length === 0}
      <div class="muted">No invite links yet.</div>
    {:else}
      <div class="invite-list">
        {#each inviteRows as invite}
          <div class="invite-item">
            <div>
              <div class="invite-meta">
                <span class="pill" class:pill-green={inviteStatus(invite) === 'active'}>
                  {inviteStatusLabel(invite)}
                </span>
                <span class="mono">
                  Uses: {invite.used_count}/{invite.max_uses}
                </span>
              </div>
              <div class="sub">
                Created: {new Date(invite.created_at).toLocaleString()} · Expires: {new Date(
                  invite.expires_at,
                ).toLocaleString()}
              </div>
              {#if invite.note}
                <div class="sub">{invite.note}</div>
              {/if}
            </div>
            {#if inviteStatus(invite) === 'active'}
              <button
                class="btn btn-secondary"
                onclick={() => revokeInvite(invite.id)}
                disabled={inviteRevokingId === invite.id}
              >
                <Icon name="x" size={14} />
                {inviteRevokingId === invite.id ? 'Revoking...' : 'Revoke'}
              </button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</Modal>

<ConfirmDialog
  show={showDelete}
  title={$t('admin.customers.delete.title') || 'Delete customer'}
  message={$t('admin.customers.delete.message') ||
    'This will remove the customer and all related data.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deleting}
  onconfirm={doDelete}
  oncancel={() => (showDelete = false)}
/>

<style>
  .page-content {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-top: 0.35rem;
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

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .stat-filter {
    display: block;
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-lg);
  }

  .stat-filter.active {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 42%, transparent);
    outline-offset: 2px;
  }

  .table-card {
    padding: 1.25rem;
  }

  .toolbar-filters {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .customer-filter-field {
    display: grid;
    gap: 0.25rem;
    min-width: 150px;
  }

  .customer-filter-field span {
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 700;
    line-height: 1;
  }

  .customer-filter-select {
    min-height: 38px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0.45rem 2rem 0.45rem 0.7rem;
    font-size: 0.86rem;
    font-weight: 700;
    outline: none;
    cursor: pointer;
  }

  .customer-filter-select:focus {
    border-color: color-mix(in srgb, var(--color-primary) 58%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 16%, transparent);
  }

  .customer-filter-select:hover {
    background: var(--bg-hover);
  }

  .customer-filter-select option {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .error-banner {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.08);
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .linkish {
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    padding: 0;
  }

  .linkish:disabled {
    cursor: default;
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
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
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

  .btn-icon:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 650;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .pill-green {
    border-color: rgba(34, 197, 94, 0.35);
    background: rgba(34, 197, 94, 0.12);
    color: rgb(34, 197, 94);
  }

  .pill-gray {
    border-color: rgba(148, 163, 184, 0.35);
    background: rgba(148, 163, 184, 0.12);
    color: rgba(148, 163, 184, 1);
  }

  .pill-warning {
    border-color: rgba(245, 158, 11, 0.34);
    background: rgba(245, 158, 11, 0.1);
    color: rgb(245, 158, 11);
  }

  .mobile-customer-list {
    display: none;
  }

  .mobile-customer-card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.85rem;
    display: grid;
    gap: 0.75rem;
  }

  .mobile-customer-head {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .mobile-customer-meta {
    display: grid;
    gap: 0.3rem;
    color: var(--text-secondary);
    font-size: 0.86rem;
    min-width: 0;
  }

  .mobile-customer-meta span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .mobile-customer-actions,
  .mobile-pager {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .mobile-pager {
    justify-content: space-between;
    padding-top: 0.15rem;
  }

  .mobile-empty {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    color: var(--text-secondary);
    padding: 1rem;
    text-align: center;
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

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .invite-section {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.75rem;
    background: var(--bg-surface);
    display: grid;
    gap: 0.65rem;
  }

  .invite-section-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
  }

  .actions-inline {
    margin-top: 0;
  }

  .invite-summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.55rem;
  }

  .invite-summary-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.55rem 0.6rem;
    background: rgba(99, 102, 241, 0.06);
    display: grid;
    gap: 0.2rem;
  }

  .invite-summary-item small {
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .invite-summary-item strong {
    font-size: 0.98rem;
  }

  .invite-result {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.75rem;
    background: var(--bg-surface);
    display: grid;
    gap: 0.6rem;
  }

  .invite-result-head {
    display: flex;
    justify-content: space-between;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .invite-copy-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.6rem;
    align-items: center;
  }

  .invite-list-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.4rem;
  }

  .inline-check {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .invite-list {
    display: grid;
    gap: 0.65rem;
    max-height: 280px;
    overflow: auto;
    padding-right: 0.25rem;
  }

  .invite-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.7rem;
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: center;
  }

  .invite-meta {
    display: inline-flex;
    gap: 0.55rem;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 1rem;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
    .header-actions .btn {
      flex: 1 1 100%;
      min-width: 0;
    }
    .toolbar-filters {
      width: 100%;
    }
    .customer-filter-field {
      flex: 1 1 180px;
      min-width: 0;
    }
    .customer-filter-select {
      width: 100%;
    }
    .grid2 {
      grid-template-columns: 1fr;
    }
    .invite-summary-grid {
      grid-template-columns: 1fr 1fr;
    }
    .invite-copy-row {
      grid-template-columns: 1fr;
    }
    .invite-item {
      flex-direction: column;
      align-items: stretch;
    }
  }

  @media (max-width: 720px) {
    .table-card {
      padding: 0.9rem;
    }
    .desktop-customer-table {
      display: none;
    }
    .mobile-customer-list {
      display: grid;
      gap: 0.75rem;
    }
    .mobile-customer-head {
      flex-direction: column;
      align-items: stretch;
    }
    .mobile-customer-actions .btn {
      flex: 1 1 auto;
    }
  }
</style>
