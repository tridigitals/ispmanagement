<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { api, type PppoeAccountPublic } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import { timeAgo } from '$lib/utils/date';

  type RouterRow = { id: string; name: string; host?: string; port?: number };
  type CustomerRow = { id: string; name: string };

  let loading = $state(true);
  let error = $state('');
  let accounts = $state<PppoeAccountPublic[]>([]);
  let routers = $state<RouterRow[]>([]);
  let customers = $state<CustomerRow[]>([]);
  let refreshing = $state(false);

  let q = $state('');
  let routerId = $state('');
  let status = $state<'any' | 'present' | 'missing'>('any');
  let disabled = $state<'any' | 'enabled' | 'disabled'>('any');

  const tenantPrefix = $derived.by(() => {
    const tid = String($page.params.tenant || '');
    return tid ? `/${tid}` : '';
  });

  const routerName = (id: string) => routers.find((r) => r.id === id)?.name || '-';
  const customerName = (id: string) => customers.find((c) => c.id === id)?.name || '-';
  const routerHost = (id: string) => routers.find((r) => r.id === id)?.host || '';
  const routerPort = (id: string) => routers.find((r) => r.id === id)?.port || 0;

  const viewRows = $derived.by(() =>
    accounts.filter((a) => {
      if (status === 'present' && !a.router_present) return false;
      if (status === 'missing' && a.router_present) return false;
      if (disabled === 'enabled' && a.disabled) return false;
      if (disabled === 'disabled' && !a.disabled) return false;
      return true;
    }),
  );

  const statusChip = (v: 'any' | 'present' | 'missing') => status === v;
  const stateChip = (v: 'any' | 'enabled' | 'disabled') => disabled === v;

  function clearFilters() {
    q = '';
    routerId = '';
    status = 'any';
    disabled = 'any';
    void loadAccounts();
  }

  const stats = $derived.by(() => {
    const total = viewRows.length;
    const present = viewRows.filter((a) => a.router_present).length;
    const missing = total - present;
    const disabledCount = viewRows.filter((a) => a.disabled).length;
    return { total, present, missing, disabled: disabledCount };
  });

  const columns = $derived.by(() => [
    { key: 'username', label: $t('admin.network.pppoe.columns.username') || 'Username' },
    { key: 'customer', label: $t('admin.network.pppoe.columns.customer') || 'Customer' },
    { key: 'router', label: $t('admin.network.pppoe.columns.router') || 'Router' },
    { key: 'sync', label: $t('admin.network.pppoe.columns.sync') || 'Sync' },
    { key: 'actions', label: '', align: 'right' as const, width: '220px' },
  ]);

  onMount(() => {
    if (!$can('read', 'pppoe') && !$can('manage', 'pppoe')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      await Promise.all([loadRouters(), loadCustomers(), loadAccounts()]);
    } catch (e: any) {
      error = String(e?.message || e || '');
    } finally {
      loading = false;
    }
  }

  async function loadRouters() {
    try {
      routers = (await api.mikrotik.routers.list()) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function loadCustomers() {
    try {
      // We only need id+name for mapping. For now, load first 1000.
      const res = await api.customers.list({ page: 1, perPage: 1000 });
      customers = (res.data || []).map((c) => ({ id: c.id, name: c.name }));
    } catch (e: any) {
      // Non-critical; list can still show ids.
    }
  }

  async function loadAccounts() {
    refreshing = true;
    try {
      // Fetch in chunks so large imports (hundreds/thousands) show fully even if the backend enforces paging.
      const requestedPerPage = 500;
      const first = await api.pppoe.accounts.list({
        q: q.trim() || undefined,
        router_id: routerId || undefined,
        page: 1,
        per_page: requestedPerPage,
      });

      let all = first.data || [];
      const total = Number(first.total || all.length);
      // Use the server's effective page size (it may clamp/ignore our requested per_page).
      const effectivePerPage =
        Number((first as any).per_page || all.length || requestedPerPage) || requestedPerPage;

      // If more pages exist, fetch the rest sequentially (keeps logic simple and avoids hammering the server).
      if (all.length < total) {
        const maxPages = Math.ceil(total / effectivePerPage);
        for (let p = 2; p <= maxPages; p++) {
          const next = await api.pppoe.accounts.list({
            q: q.trim() || undefined,
            router_id: routerId || undefined,
            page: p,
            per_page: effectivePerPage,
          });
          const chunk = next.data || [];
          if (chunk.length === 0) break;
          all = [...all, ...chunk];
          if (all.length >= total) break;
        }
      }

      accounts = all;
    } catch (e: any) {
      const msg = String(e?.message || e || 'Failed to load PPPoE accounts');
      error = msg;
      toast.error(msg);
    } finally {
      refreshing = false;
    }
  }

  async function reconcileAll() {
    const routerIds = Array.from(new Set(viewRows.map((a) => a.router_id).filter(Boolean)));
    if (routerIds.length === 0) return;
    try {
      for (const rid of routerIds) {
        await api.pppoe.accounts.reconcileRouter(rid);
      }
      toast.success($t('admin.network.pppoe.toasts.reconciled') || 'Reconciled router state');
      await loadAccounts();
    } catch (e: any) {
      toast.error(
        $t('admin.network.pppoe.toasts.reconcile_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }

  async function apply(row: PppoeAccountPublic) {
    try {
      await api.pppoe.accounts.apply(row.id);
      toast.success($t('admin.network.pppoe.toasts.applied') || 'Applied to router');
      await loadAccounts();
    } catch (e: any) {
      toast.error(
        $t('admin.network.pppoe.toasts.apply_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }
</script>

<div class="page-content fade-in">
  <section class="hero">
    <div class="hero-bg"></div>
    <div class="hero-inner">
      <div class="hgroup">
        <div class="kicker">
          <span class="dot"></span>
          {$t('sidebar.sections.network') || 'Network'}
        </div>
        <h1 class="h1">{$t('admin.network.pppoe.title') || 'PPPoE'}</h1>
        <div class="sub">
          {$t('admin.network.pppoe.subtitle') || 'Tenant-wide view of PPPoE accounts across routers.'}
        </div>
      </div>

      <div class="hero-actions">
        <div class="search">
          <Icon name="search" size={16} />
          <input
            class="search-input"
            value={q}
            oninput={(e) => {
              q = (e.currentTarget as HTMLInputElement).value;
              void loadAccounts();
            }}
            placeholder={$t('admin.network.pppoe.search') || 'Search PPPoE...'}
          />
          {#if q.trim()}
            <button class="clear" type="button" onclick={() => (q = '')}>
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>

        <div class="hero-buttons">
          <button class="btn btn-secondary" onclick={loadAccounts} disabled={refreshing}>
            <Icon name="refresh-cw" size={16} />
            {$t('common.refresh') || 'Refresh'}
          </button>
          {#if $can('manage', 'pppoe')}
            <button
              class="btn btn-secondary"
              onclick={() => goto(`${tenantPrefix}/admin/network/pppoe/import`)}
              title={$t('admin.network.pppoe.import.title') || 'Import PPPoE from MikroTik'}
            >
              <Icon name="download" size={16} />
              {$t('admin.network.pppoe.import.cta') || 'Import'}
            </button>
          {/if}
          {#if $can('manage', 'pppoe')}
            <button
              class="btn btn-secondary"
              onclick={reconcileAll}
              disabled={refreshing || viewRows.length === 0}
              title={$t('admin.network.pppoe.actions.reconcile') || 'Reconcile'}
            >
              <Icon name="refresh-cw" size={16} />
              {$t('admin.network.pppoe.actions.reconcile') || 'Reconcile'}
            </button>
          {/if}
          {#if q.trim() || routerId || status !== 'any' || disabled !== 'any'}
            <button class="btn btn-secondary" onclick={clearFilters}>
              <Icon name="eraser" size={16} />
              {$t('common.clear') || 'Clear'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </section>

  <div class="stats-grid">
    <StatsCard
      title={$t('admin.network.pppoe.stats.total') || 'Total'}
      value={stats.total}
      icon="key"
      color="blue"
    />
    <StatsCard
      title={$t('admin.network.pppoe.stats.present') || 'On router'}
      value={stats.present}
      icon="check-circle"
      color="green"
    />
    <StatsCard
      title={$t('admin.network.pppoe.stats.missing') || 'Missing'}
      value={stats.missing}
      icon="alert-triangle"
      color="orange"
    />
    <StatsCard
      title={$t('admin.network.pppoe.stats.disabled') || 'Disabled'}
      value={stats.disabled}
      icon="pause"
      color="warning"
    />
  </div>

  <div class="card table-card">
    <TableToolbar
      showSearch={false}
    >
      {#snippet filters()}
        <div class="filters">
          <div class="chips">
            <button class="chip {statusChip('any') ? 'active' : ''}" type="button" onclick={() => (status = 'any')}>
              {$t('admin.network.pppoe.filters.any') || 'Any'}
            </button>
            <button
              class="chip {statusChip('present') ? 'active' : ''}"
              type="button"
              onclick={() => (status = 'present')}
              title={$t('admin.network.pppoe.filters.present') || 'On router'}
            >
              {$t('admin.network.pppoe.filters.present') || 'On router'}
            </button>
            <button
              class="chip {statusChip('missing') ? 'active' : ''}"
              type="button"
              onclick={() => (status = 'missing')}
              title={$t('admin.network.pppoe.filters.missing') || 'Missing'}
            >
              {$t('admin.network.pppoe.filters.missing') || 'Missing'}
            </button>
          </div>

          <div class="chips">
            <button class="chip {stateChip('any') ? 'active' : ''}" type="button" onclick={() => (disabled = 'any')}>
              {$t('admin.network.pppoe.filters.any') || 'Any'}
            </button>
            <button
              class="chip {stateChip('enabled') ? 'active' : ''}"
              type="button"
              onclick={() => (disabled = 'enabled')}
            >
              {$t('admin.network.pppoe.filters.enabled') || 'Enabled'}
            </button>
            <button
              class="chip {stateChip('disabled') ? 'active' : ''}"
              type="button"
              onclick={() => (disabled = 'disabled')}
            >
              {$t('admin.network.pppoe.filters.disabled_only') || 'Disabled'}
            </button>
          </div>

          <label class="router-filter">
            <span class="label">{$t('admin.network.pppoe.filters.router') || 'Router'}</span>
            <select class="select" bind:value={routerId} onchange={() => void loadAccounts()}>
              <option value="">{($t('admin.network.pppoe.filters.all') || 'All') + '...'}</option>
              {#each routers as r}
                <option value={r.id}>{r.name}</option>
              {/each}
            </select>
          </label>
        </div>
      {/snippet}
      {#snippet actions()}
        <span class="muted">{viewRows.length} {$t('common.results') || 'results'}</span>
      {/snippet}
    </TableToolbar>

    {#if error}
      <div class="error-banner">
        <Icon name="alert-triangle" size={18} />
        <span>{error}</span>
      </div>
    {/if}

    <Table
      columns={columns}
      data={viewRows}
      loading={loading}
      emptyText={$t('admin.network.pppoe.empty') || 'No PPPoE accounts.'}
      pagination
    >
      {#snippet cell({ item, key })}
        {@const row = item as PppoeAccountPublic}
        {#if key === 'username'}
          <div class="stack">
            <div class="name">{row.username}</div>
            <div class="meta">
              {#if row.disabled}
                <span class="badge warn">{$t('common.disabled') || 'Disabled'}</span>
              {:else}
                <span class="badge ok">{$t('common.active') || 'Active'}</span>
              {/if}
              <span class="pill">
                {$t('admin.customers.pppoe.fields.profile') || 'Profile'}:
                {row.router_profile_name || '-'}
              </span>
              <span class="pill">
                {$t('admin.customers.pppoe.fields.remote_address') || 'Remote'}:
                {row.remote_address || row.address_pool || '-'}
              </span>
            </div>
          </div>
        {:else if key === 'customer'}
          <div class="stack">
            <div class="name">{customerName(row.customer_id)}</div>
            <div class="meta">
              <span class="pill mono" title={row.customer_id}>{row.customer_id.slice(0, 8)}…</span>
              <span class="pill mono" title={row.location_id}>
                {$t('sidebar.locations') || 'Locations'}: {row.location_id.slice(0, 8)}…
              </span>
            </div>
          </div>
        {:else if key === 'router'}
          <div class="stack">
            <div class="name">{routerName(row.router_id)}</div>
            <div class="meta">
              {#if routerHost(row.router_id)}
                <span class="pill mono">{routerHost(row.router_id)}:{routerPort(row.router_id) || ''}</span>
              {/if}
              <span class="pill mono" title={row.router_id}>{row.router_id.slice(0, 8)}…</span>
            </div>
          </div>
        {:else if key === 'sync'}
          <div class="stack">
            <div class="meta">
              {#if row.router_present}
                <span class="badge ok">{$t('admin.network.pppoe.sync.present') || 'On router'}</span>
              {:else}
                <span class="badge warn">{$t('admin.network.pppoe.sync.missing') || 'Missing'}</span>
              {/if}
              <span class="pill mono">{row.last_sync_at ? timeAgo(row.last_sync_at) : '-'}</span>
            </div>
            {#if row.last_error}
              <div class="error-line" title={row.last_error}>
                <Icon name="alert-triangle" size={14} />
                <span class="error-text">{row.last_error}</span>
              </div>
            {/if}
          </div>
        {:else if key === 'actions'}
          <div class="row-actions">
            <button
              class="btn-icon"
              title={$t('admin.network.pppoe.actions.open_customer') || 'Open customer'}
              onclick={() => goto(`${tenantPrefix}/admin/customers/${row.customer_id}`)}
            >
              <Icon name="external-link" size={16} />
            </button>
            {#if $can('manage', 'pppoe')}
              <button
                class="btn-icon"
                title={$t('admin.network.pppoe.actions.apply') || 'Apply to router'}
                onclick={() => apply(row)}
              >
                <Icon name="send" size={16} />
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

<style>
  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 22px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.28), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.12), transparent 60%),
      radial-gradient(900px 220px at 50% 110%, rgba(245, 158, 11, 0.14), transparent 60%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
    filter: saturate(1.1);
  }

  :global([data-theme='light']) .hero-bg {
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.18), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.08), transparent 60%),
      radial-gradient(900px 220px at 50% 110%, rgba(245, 158, 11, 0.1), transparent 60%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.02), rgba(0, 0, 0, 0.01));
  }

  .hero-inner {
    position: relative;
    padding: 1.15rem 1.2rem 1.2rem;
    display: grid;
    gap: 1rem;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.78rem;
  }

  .kicker .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .h1 {
    margin-top: 0.25rem;
    font-size: clamp(1.55rem, 2.2vw, 2rem);
    font-weight: 1000;
    letter-spacing: 0.01em;
    color: var(--text-primary);
    line-height: 1.12;
  }

  .sub {
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-weight: 650;
    max-width: 70ch;
  }

  .hero-actions {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .hero-buttons {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.18);
    padding: 0.55rem 0.7rem;
    border-radius: 14px;
    min-width: min(520px, 100%);
    color: rgba(255, 255, 255, 0.85);
  }

  :global([data-theme='light']) .search {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(255, 255, 255, 0.8);
    color: rgba(0, 0, 0, 0.75);
  }

  .search-input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font-weight: 750;
    font-size: 0.95rem;
    min-height: 0;
  }

  .clear {
    border: none;
    background: transparent;
    color: inherit;
    opacity: 0.8;
    cursor: pointer;
    padding: 0.2rem;
    border-radius: 10px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .clear:hover {
    opacity: 1;
    background: rgba(255, 255, 255, 0.06);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .filters {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    flex-wrap: wrap;
    padding: 0.2rem 0.1rem 0.6rem;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .chip {
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    padding: 0.5rem 0.75rem;
    border-radius: 999px;
    font-weight: 850;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    user-select: none;
  }

  :global([data-theme='light']) .chip {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
  }

  .chip:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .chip.active {
    border-color: rgba(99, 102, 241, 0.55);
    background: rgba(99, 102, 241, 0.18);
    color: var(--text-primary);
  }

  .router-filter {
    display: flex;
    gap: 0.45rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .label {
    font-size: 12px;
    opacity: 0.75;
    font-weight: 800;
  }

  .select {
    padding: 0.55rem 0.7rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
  }

  .stack {
    display: grid;
    gap: 0.35rem;
  }

  .name {
    font-weight: 850;
    letter-spacing: 0.01em;
  }

  .meta {
    opacity: 0.9;
    font-size: 12px;
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }
  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn-icon {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    color: var(--text-primary);
    border-radius: 12px;
    width: 36px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .btn-icon:hover {
    background: var(--bg-hover);
  }
  .badge {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    font-weight: 700;
  }
  .badge.ok {
    border-color: color-mix(in srgb, #16a34a 45%, var(--border-color));
    color: #16a34a;
  }
  .badge.warn {
    border-color: color-mix(in srgb, #f59e0b 45%, var(--border-color));
    color: #f59e0b;
  }
  .pill {
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    padding: 0.25rem 0.55rem;
    border-radius: 999px;
    font-weight: 800;
  }

  :global([data-theme='light']) .pill {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
  }

  .error-line {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: #ef4444;
    font-weight: 750;
    font-size: 12px;
    opacity: 0.95;
    max-width: 520px;
  }

  .error-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 768px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }

    .hero-buttons {
      width: 100%;
    }
    .hero-buttons :global(.btn) {
      width: 100%;
      justify-content: center;
    }
    .router-filter {
      width: 100%;
    }
    .select {
      width: 100%;
    }
    .error-line {
      max-width: 100%;
    }
  }

  @media (max-width: 1100px) {
    .stats-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
