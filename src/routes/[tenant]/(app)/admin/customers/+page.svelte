<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type Customer, type PaginatedResponse } from '$lib/api/client';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.customers.columns.customer') || 'Customer' },
    { key: 'contact', label: $t('admin.customers.columns.contact') || 'Contact' },
    { key: 'status', label: $t('admin.customers.columns.status') || 'Status' },
    { key: 'updated_at', label: $t('admin.customers.columns.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  let customers = $state<Customer[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let error = $state('');

  let q = $state('');
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

  let stats = $derived({
    total,
    active: customers.filter((c) => c.is_active).length,
    inactive: customers.filter((c) => !c.is_active).length,
  });

  onMount(async () => {
    if (!$can('read', 'customers') && !$can('manage', 'customers')) {
      goto('/unauthorized');
      return;
    }
    await load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      const res: PaginatedResponse<Customer> = await api.customers.list({
        q,
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

  function openCustomer(c: Customer) {
    goto(`./${c.id}`);
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
      await load();
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
      await load();
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
      <button class="btn btn-secondary" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if $can('manage', 'customers')}
        <button class="btn btn-primary" onclick={() => (showCreate = true)}>
          <Icon name="plus" size={16} />
          {$t('admin.customers.actions.new') || 'New customer'}
        </button>
      {/if}
    </div>
  </div>

  <div class="stats-grid">
    <StatsCard
      title={$t('admin.customers.stats.total') || 'Total'}
      value={stats.total}
      icon="users"
      color="blue"
    />
    <StatsCard
      title={$t('admin.customers.stats.active') || 'Active'}
      value={stats.active}
      icon="check-circle"
      color="green"
    />
    <StatsCard
      title={$t('admin.customers.stats.inactive') || 'Inactive'}
      value={stats.inactive}
      icon="x-circle"
      color="orange"
    />
  </div>

  <div class="card table-card">
    <TableToolbar
      bind:searchQuery={q}
      placeholder={$t('admin.customers.search') || 'Search customers...'}
      onsearch={() => {
        page = 0;
        load();
      }}
    >
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

    <Table
      {columns}
      data={customers}
      keyField="id"
      loading={loading}
      emptyText={$t('admin.customers.empty') || 'No customers yet.'}
      pagination
      serverSide
      pageSize={perPage}
      count={total}
      onchange={(p) => {
        page = p;
        load();
      }}
      onpageSizeChange={(s) => {
        perPage = s;
        page = 0;
        load();
      }}
    >
      {#snippet cell({ item, key })}
        {@const c = item as Customer}
        {#if key === 'name'}
          <button class="linkish" onclick={() => openCustomer(c)}>
            <div class="name">{c.name}</div>
            <div class="sub">{c.email || c.phone || ''}</div>
          </button>
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
        {:else if key === 'updated_at'}
          <span class="mono">{new Date(c.updated_at).toLocaleString()}</span>
        {:else if key === 'actions'}
          <div class="row-actions">
            <button class="btn-icon" title={$t('common.open') || 'Open'} onclick={() => openCustomer(c)}>
              <Icon name="arrow-right" size={16} />
            </button>
            {#if $can('manage', 'customers')}
              <button
                class="btn-icon danger"
                title={$t('common.delete') || 'Delete'}
                onclick={() => confirmDelete(c)}
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
        disabled={
          creating ||
          !createName.trim() ||
          !createEmail.trim() ||
          !createPortalPassword ||
          !createPortalPasswordConfirm ||
          createPortalPassword !== createPortalPasswordConfirm
        }
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
      </button>
    </div>
  </div>
</Modal>

<ConfirmDialog
  show={showDelete}
  title={$t('admin.customers.delete.title') || 'Delete customer'}
  message={$t('admin.customers.delete.message') || 'This will remove the customer and all related data.'}
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

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .table-card {
    padding: 1.25rem;
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
    .grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
