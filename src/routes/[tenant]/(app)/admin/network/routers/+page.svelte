<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Table from '$lib/components/ui/Table.svelte';

  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    username: string;
    enabled: boolean;
    identity?: string | null;
    ros_version?: string | null;
    is_online: boolean;
    last_seen_at?: string | null;
    latency_ms?: number | null;
    last_error?: string | null;
    updated_at?: string;
  };

  let loading = $state(true);
  let routers = $state<RouterRow[]>([]);
  let search = $state('');
  let refreshing = $state(false);
  let lastRefreshAt = $state<number | null>(null);

  let showModal = $state(false);
  let editing: RouterRow | null = $state(null);

  let formName = $state('');
  let formHost = $state('');
  let formPort = $state(8728);
  let formUsername = $state('');
  let formPassword = $state('');
  let formEnabled = $state(true);

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return routers;
    return routers.filter((r) => {
      const hay = `${r.name} ${r.host} ${r.identity || ''}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const stats = $derived.by(() => {
    const total = routers.length;
    const online = routers.filter((r) => r.is_online).length;
    const offline = total - online;
    return { total, online, offline };
  });

  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.network.routers.columns.name') || 'Name' },
    { key: 'host', label: $t('admin.network.routers.columns.host') || 'Host' },
    { key: 'status', label: $t('admin.network.routers.columns.status') || 'Status' },
    { key: 'latency', label: $t('admin.network.routers.columns.latency') || 'Latency' },
    { key: 'seen', label: $t('admin.network.routers.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '220px' },
  ]);

  let refreshHandle: any = null;

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }
    void load();

    // Keep status reasonably fresh without requiring manual refresh.
    // Note: server also runs a background poller; this is just UI sync.
    const intervalMs = 5000;
    refreshHandle = setInterval(() => {
      void refreshSilent();
    }, intervalMs);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

  async function load() {
    loading = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      lastRefreshAt = Date.now();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    if (refreshing || showModal) return;
    refreshing = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      lastRefreshAt = Date.now();
    } catch {
      // ignore (avoid noisy toasts for background refresh)
    } finally {
      refreshing = false;
    }
  }

  function openCreate() {
    editing = null;
    formName = '';
    formHost = '';
    formPort = 8728;
    formUsername = '';
    formPassword = '';
    formEnabled = true;
    showModal = true;
  }

  function openEdit(r: RouterRow) {
    editing = r;
    formName = r.name || '';
    formHost = r.host || '';
    formPort = Number(r.port || 8728);
    formUsername = r.username || '';
    formPassword = '';
    formEnabled = r.enabled ?? true;
    showModal = true;
  }

  async function save() {
    const name = formName.trim();
    const host = formHost.trim();
    if (!name || !host || !formUsername.trim()) {
      toast.error($t('common.validation_error') || 'Please fill required fields.');
      return;
    }
    if (!editing && !formPassword.trim()) {
      toast.error($t('admin.network.routers.form.password') || 'Password is required.');
      return;
    }

    try {
      if (editing) {
        await api.mikrotik.routers.update(editing.id, {
          name,
          host,
          port: formPort,
          username: formUsername.trim(),
          password: formPassword.trim() ? formPassword : undefined,
          enabled: formEnabled,
        });
        toast.success($t('admin.network.routers.toasts.updated') || 'Router updated');
      } else {
        await api.mikrotik.routers.create({
          name,
          host,
          port: formPort,
          username: formUsername.trim(),
          password: formPassword,
          enabled: formEnabled,
        });
        toast.success($t('admin.network.routers.toasts.created') || 'Router created');
      }
      showModal = false;
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function test(r: RouterRow) {
    try {
      const res = await api.mikrotik.routers.test(r.id);
      if (res?.ok) {
        toast.success(
          `${res.identity || r.name} • RouterOS ${res.ros_version || ''} • ${res.latency_ms ?? ''}ms`,
        );
      } else {
        toast.error(res?.error || 'Failed to connect');
      }
      await refreshSilent();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function remove(r: RouterRow) {
    const ok = confirm(`${$t('common.delete') || 'Delete'}: ${r.name}?`);
    if (!ok) return;
    try {
      await api.mikrotik.routers.delete(r.id);
      toast.success($t('admin.network.routers.toasts.deleted') || 'Router deleted');
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openDetail(r: RouterRow) {
    goto(`${$page.url.pathname}/${r.id}`);
  }

  function statusLabel(r: RouterRow) {
    if (r.is_online) return $t('admin.network.routers.badges.online') || 'Online';
    return $t('admin.network.routers.badges.offline') || 'Offline';
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.network.routers.title') || 'Routers'}</h1>
      <p class="sub">{$t('admin.network.routers.subtitle') || 'Manage MikroTik routers and monitoring'}</p>
    </div>

    <div class="head-actions">
      <button class="btn ghost" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('admin.network.routers.actions.refresh') || $t('common.refresh') || 'Refresh'}
      </button>

      {#if $can('manage', 'network_routers')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          {$t('admin.network.routers.actions.add') || 'Add Router'}
        </button>
      {/if}
    </div>
  </div>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span class="stat-label">Total</span>
        <Icon name="list" size={14} />
      </div>
      <div class="stat-value">{stats.total}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span class="stat-label">Online</span>
        <Icon name="check-circle" size={14} />
      </div>
      <div class="stat-value">{stats.online}</div>
    </div>
    <div class="stat-card tone-bad">
      <div class="stat-top">
        <span class="stat-label">Offline</span>
        <Icon name="alert-circle" size={14} />
      </div>
      <div class="stat-value">{stats.offline}</div>
    </div>
  </div>

  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={16} />
      <input
        class="search-input"
        bind:value={search}
        placeholder={$t('admin.network.routers.search') || 'Search routers...'}
      />
      {#if search}
        <button class="clear" type="button" onclick={() => (search = '')}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </div>
  </div>

  <div class="table-wrap">
    <Table
      {columns}
      data={filtered}
      loading={loading}
      emptyText={$t('admin.network.routers.empty') || 'No routers yet'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'name'}
          <div class="name-cell">
            <div class="name-top">
              <span class="name">{item.name}</span>
              {#if item.identity}
                <span class="chip">{item.identity}</span>
              {/if}
            </div>
            <div class="muted">{item.username}@{item.host}:{item.port}</div>
            {#if item.last_error}
              <div class="error">{item.last_error}</div>
            {/if}
          </div>
        {:else if key === 'host'}
          <span class="mono">{item.host}:{item.port}</span>
        {:else if key === 'status'}
          <span class="badge" class:online={item.is_online} class:offline={!item.is_online}>
            {statusLabel(item)}
          </span>
        {:else if key === 'latency'}
          {#if item.latency_ms != null}
            <span class="mono">{item.latency_ms} ms</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'seen'}
          <span class="muted">{item.last_seen_at || '—'}</span>
        {:else if key === 'actions'}
          <div class="actions">
            <button class="icon-btn" type="button" onclick={() => openDetail(item)} title={$t('admin.network.routers.actions.open') || 'Open'}>
              <Icon name="arrow-right" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => test(item)} title={$t('admin.network.routers.actions.test') || 'Test Connection'}>
              <Icon name="zap" size={16} />
            </button>
            {#if $can('manage', 'network_routers')}
              <button class="icon-btn" type="button" onclick={() => openEdit(item)} title={$t('admin.network.routers.actions.edit') || 'Edit'}>
                <Icon name="edit" size={16} />
              </button>
              <button class="icon-btn danger" type="button" onclick={() => remove(item)} title={$t('admin.network.routers.actions.delete') || 'Delete'}>
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

<Modal
  show={showModal}
  title={editing
    ? `${$t('admin.network.routers.actions.edit') || 'Edit'}: ${editing.name}`
    : $t('admin.network.routers.actions.add') || 'Add Router'}
  width="520px"
  onclose={() => (showModal = false)}
>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <label>
      <span>{$t('admin.network.routers.form.name') || 'Name'}</span>
      <input bind:value={formName} placeholder="e.g. POP Router 1" />
    </label>
    <label>
      <span>{$t('admin.network.routers.form.host') || 'Host'}</span>
      <input bind:value={formHost} placeholder="192.168.88.1" />
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.network.routers.form.port') || 'Port'}</span>
        <input type="number" bind:value={formPort} min="1" max="65535" />
      </label>
      <label>
        <span>{$t('admin.network.routers.form.username') || 'Username'}</span>
        <input bind:value={formUsername} placeholder="admin" />
      </label>
    </div>

    <label>
      <span>{$t('admin.network.routers.form.password') || 'Password'}</span>
      <input
        type="password"
        bind:value={formPassword}
        placeholder={editing ? 'Leave blank to keep current password' : ''}
      />
    </label>

    <label class="check">
      <input type="checkbox" bind:checked={formEnabled} />
      <span>{$t('admin.network.routers.form.enabled') || 'Enabled'}</span>
    </label>

    <div class="modal-actions">
      <button class="btn ghost" type="button" onclick={() => (showModal = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" type="submit">
        <Icon name="save" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </form>
</Modal>

<style>
  .page-content {
    padding: 28px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 18px;
  }

  .head-actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .title {
    margin: 0;
    font-size: 1.6rem;
    color: var(--text-primary);
  }

  .sub {
    margin: 6px 0 0;
    color: var(--text-secondary);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 700;
    cursor: pointer;
    transition: transform 0.12s ease, filter 0.12s ease;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:hover {
    filter: brightness(1.05);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px 14px 12px;
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 10px;
    font-size: 1.6rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .tone-ok {
    box-shadow: 0 0 0 1px rgba(34, 197, 94, 0.15) inset;
  }

  .tone-bad {
    box-shadow: 0 0 0 1px rgba(239, 68, 68, 0.16) inset;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px 12px;
    min-width: min(560px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.95rem;
  }

  .clear {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
  }

  .table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
  }

  .name-cell .name-top {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .name {
    font-weight: 900;
    color: var(--text-primary);
  }

  .chip {
    font-size: 0.72rem;
    font-weight: 800;
    padding: 3px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-hover), transparent 20%);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    color: var(--text-primary);
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .error {
    margin-top: 6px;
    color: color-mix(in srgb, #ef4444, var(--text-primary) 15%);
    font-size: 0.85rem;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.78rem;
    border: 1px solid var(--border-color);
  }

  .badge.online {
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
    border-color: rgba(34, 197, 94, 0.28);
  }

  .badge.offline {
    background: rgba(239, 68, 68, 0.12);
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
  }

  .actions {
    display: inline-flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .icon-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 12px;
    padding: 8px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .icon-btn:hover {
    background: var(--bg-hover);
  }

  .icon-btn.danger {
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-secondary);
    font-weight: 700;
  }

  input[type='password'],
  input[type='number'],
  input {
    background: var(--bg-input, color-mix(in srgb, var(--bg-card), transparent 8%));
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 10px 12px;
    color: var(--text-primary);
    outline: none;
  }

  input:focus {
    border-color: rgba(99, 102, 241, 0.55);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.18);
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 10px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 12px;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 18px;
    }

    .head {
      flex-direction: column;
      align-items: stretch;
    }

    .head-actions {
      justify-content: flex-start;
      flex-wrap: wrap;
    }

    .stats {
      grid-template-columns: 1fr;
    }

    .search {
      min-width: 0;
      width: 100%;
    }
  }
</style>
