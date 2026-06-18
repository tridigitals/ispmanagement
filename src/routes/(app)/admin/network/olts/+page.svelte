<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import type { Olt } from '$lib/api/olt';
  import { toast } from '$lib/stores/toast';
  import { appendBackParam } from '$lib/utils/backNavigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { appSettings } from '$lib/stores/settings';

  const OLT_TYPE_MAP: Record<string, string> = {
    hioso_ha7302cst: 'HIOSO HA-7302CST (EPON)',
    vsol_epon: 'VSOL (EPON)',
  };

  function friendlyOltType(t: string): string {
    return OLT_TYPE_MAP[t] || t;
  }

  let loading = $state(true);
  let olts = $state<Olt[]>([]);
  let search = $state('');
  let isMobile = $state(false);

  let showModal = $state(false);
  let editing: Olt | null = $state(null);
  let showDeleteConfirm = $state(false);
  let deleteTarget = $state<Olt | null>(null);
  let testingId = $state<string | null>(null);
  let saving = $state(false);

  // Form fields
  let formName = $state('');
  let formDescription = $state('');
  let formOltType = $state('hioso_ha7302cst');
  let formHost = $state('');
  let formPort = $state(161);
  let formUsername = $state('');
  let formPassword = $state('');

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return olts;
    return olts.filter((o) => {
      const hay = `${o.name} ${o.host} ${o.description || ''} ${friendlyOltType(o.olt_type)}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const stats = $derived.by(() => {
    const total = olts.length;
    const online = olts.filter((o) => o.is_online).length;
    const offline = total - online;
    return { total, online, offline };
  });

  const columns = $derived.by(() => [
    { key: 'name', label: 'Nama' },
    { key: 'type', label: 'Tipe' },
    { key: 'host', label: 'Host' },
    { key: 'status', label: 'Status' },
    { key: 'seen', label: 'Terakhir Dilihat' },
    { key: 'actions', label: '', align: 'right' as const, width: '200px' },
  ]);

  let refreshHandle: any = null;

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }
    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 1024px)');
      const sync = () => (isMobile = mq.matches);
      sync();
      try {
        mq.addEventListener('change', sync);
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
      }
    }
    void load();
    refreshHandle = setInterval(() => {
      void loadSilent();
    }, 10000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

  async function load() {
    loading = true;
    try {
      olts = (await api.olt.list()) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function loadSilent() {
    if (showModal) return;
    try {
      olts = (await api.olt.list()) as any;
    } catch {
      // silent
    }
  }

  function openCreate() {
    editing = null;
    formName = '';
    formDescription = '';
    formOltType = 'hioso_ha7302cst';
    formHost = '';
    formPort = 161;
    formUsername = '';
    formPassword = '';
    showModal = true;
  }

  function openEdit(o: Olt) {
    editing = o;
    formName = o.name || '';
    formDescription = o.description || '';
    formOltType = o.olt_type || 'hioso_ha7302cst';
    formHost = o.host || '';
    formPort = o.port || 161;
    formUsername = o.username || '';
    formPassword = '';
    showModal = true;
  }

  async function save() {
    const name = formName.trim();
    const host = formHost.trim();
    const username = formUsername.trim();
    if (!name || !host || !username) {
      toast.error('Harap isi semua kolom wajib.');
      return;
    }
    if (!editing && !formPassword.trim()) {
      toast.error('Password wajib diisi.');
      return;
    }
    saving = true;
    try {
      if (editing) {
        await api.olt.update(editing.id, {
          name,
          description: formDescription.trim() || null,
          host,
          port: formPort,
          username,
          password: formPassword.trim() ? formPassword : undefined,
        });
        toast.success('OLT berhasil diperbarui.');
      } else {
        await api.olt.create({
          name,
          description: formDescription.trim() || null,
          olt_type: formOltType,
          host,
          port: formPort,
          username,
          password: formPassword,
        });
        toast.success('OLT berhasil ditambahkan.');
      }
      showModal = false;
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  async function testConnection(o: Olt) {
    testingId = o.id;
    try {
      const result = await api.olt.test({
        host: o.host,
        port: o.port,
        username: o.username,
        password: '', // Backend uses stored credentials
        olt_type: o.olt_type,
      });
      if (result?.success) {
        toast.success(`Koneksi berhasil! ${result.info?.model || ''} ${result.info?.version ? `v${result.info.version}` : ''}`);
      } else {
        toast.error(result?.error || 'Gagal terhubung.');
      }
      await loadSilent();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      testingId = null;
    }
  }

  function remove(o: Olt) {
    deleteTarget = o;
    showDeleteConfirm = true;
  }

  async function handleConfirmDelete() {
    if (!deleteTarget) return;
    const o = deleteTarget;
    deleteTarget = null;
    try {
      await api.olt.delete(o.id);
      toast.success('OLT berhasil dihapus.');
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openDetail(o: Olt) {
    goto(appendBackParam(`${$page.url.pathname}/${o.id}`, $page.url));
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader title="OLT Monitoring" subtitle="Kelola perangkat OLT dan pantau status ONU.">
    {#snippet actions()}
      <button class="btn ghost" type="button" onclick={load} title="Refresh">
        <Icon name="refresh-cw" size={16} />
        Refresh
      </button>
      {#if $can('manage', 'router_inventory')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          Tambah OLT
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

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
      <input class="search-input" bind:value={search} placeholder="Cari OLT..." />
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
      emptyText="Belum ada OLT"
      mobileView={isMobile ? 'card' : 'scroll'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'name'}
          <div class="name-cell">
            <div class="name-top">
              <span class="name">{item.name}</span>
              <span class="chip">{friendlyOltType(item.olt_type)}</span>
            </div>
            {#if item.description}
              <div class="muted">{item.description}</div>
            {/if}
            <div class="muted">{item.username}@{item.host}:{item.port}</div>
            {#if item.last_error}
              <div class="error">{item.last_error}</div>
            {/if}
          </div>
        {:else if key === 'type'}
          <span class="chip">{friendlyOltType(item.olt_type)}</span>
        {:else if key === 'host'}
          <span class="mono">{item.host}:{item.port}</span>
        {:else if key === 'status'}
          <span class="badge" class:online={item.is_online} class:offline={!item.is_online}>
            {item.is_online ? 'Online' : 'Offline'}
          </span>
        {:else if key === 'seen'}
          {#if item.last_polled_at}
            <span class="muted" title={formatDateTime(item.last_polled_at, { timeZone: $appSettings.app_timezone })}>
              {timeAgo(item.last_polled_at)}
            </span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'actions'}
          <div class="actions">
            <button class="icon-btn" type="button" onclick={() => openDetail(item)} title="Buka">
              <Icon name="arrow-right" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => testConnection(item)} disabled={testingId === item.id} title="Test Koneksi">
              <Icon name="zap" size={16} />
            </button>
            {#if $can('manage', 'router_inventory')}
              <button class="icon-btn" type="button" onclick={() => openEdit(item)} title="Edit">
                <Icon name="edit" size={16} />
              </button>
              <button class="icon-btn danger" type="button" onclick={() => remove(item)} title="Hapus">
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

{#if showModal}
  <div class="modal-backdrop" onclick={() => (showModal = false)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>{editing ? 'Edit OLT' : 'Tambah OLT'}</h3>
        <button class="icon-btn" type="button" onclick={() => (showModal = false)}>
          <Icon name="x" size={18} />
        </button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label for="olt-name">Nama <span class="req">*</span></label>
          <input id="olt-name" type="text" bind:value={formName} placeholder="OLT Gedung A" />
        </div>
        <div class="form-group">
          <label for="olt-desc">Deskripsi</label>
          <input id="olt-desc" type="text" bind:value={formDescription} placeholder="OLT untuk area gedung A" />
        </div>
        <div class="form-group">
          <label for="olt-type">Tipe OLT <span class="req">*</span></label>
          <select id="olt-type" bind:value={formOltType} disabled={!!editing}>
            <option value="hioso_ha7302cst">HIOSO HA-7302CST (EPON)</option>
            <option value="vsol_epon">VSOL (EPON)</option>
          </select>
        </div>
        <div class="form-row">
          <div class="form-group flex-2">
            <label for="olt-host">Host <span class="req">*</span></label>
            <input id="olt-host" type="text" bind:value={formHost} placeholder="192.168.1.1" />
          </div>
          <div class="form-group flex-1">
            <label for="olt-port">Port <span class="req">*</span></label>
            <input id="olt-port" type="number" bind:value={formPort} min="1" max="65535" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group flex-1">
            <label for="olt-user">Username <span class="req">*</span></label>
            <input id="olt-user" type="text" bind:value={formUsername} placeholder="admin" />
          </div>
          <div class="form-group flex-1">
            <label for="olt-pass">Password {editing ? '' : '<span class="req">*</span>'}</label>
            <input id="olt-pass" type="password" bind:value={formPassword} placeholder={editing ? 'Kosongkan jika tidak diubah' : '••••••'} />
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn ghost" type="button" onclick={() => (showModal = false)}>Batal</button>
        <button class="btn" type="button" onclick={save} disabled={saving}>
          {#if saving}
            <Icon name="loader" size={16} />
            Menyimpan...
          {:else}
            {editing ? 'Perbarui' : 'Simpan'}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title="Konfirmasi Hapus"
  message="Yakin ingin menghapus OLT ini? Tindakan ini tidak dapat dibatalkan."
  confirmText="Hapus"
  cancelText="Batal"
  type="danger"
  onconfirm={handleConfirmDelete}
  oncancel={() => { deleteTarget = null; }}
/>

<style>
  .page-content {
    padding: 24px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 13px;
    border-radius: 10px;
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

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 12px;
  }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 12px 13px 11px;
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
    margin-top: 8px;
    font-size: 1.42rem;
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
    margin-bottom: 10px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 9px 11px;
    min-width: min(500px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.9rem;
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
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
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
    font-size: 0.7rem;
    font-weight: 800;
    padding: 2px 7px;
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
    font-size: 0.86rem;
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
    padding: 5px 9px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.74rem;
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
    border-radius: 10px;
    padding: 7px;
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

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: grid;
    place-items: center;
    z-index: 1000;
    padding: 24px;
  }

  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    width: 100%;
    max-width: 520px;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .form-group label {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .form-group input,
  .form-group select {
    padding: 9px 11px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
  }

  .form-group input:focus,
  .form-group select:focus {
    border-color: var(--color-primary);
  }

  .form-row {
    display: flex;
    gap: 12px;
  }

  .flex-1 {
    flex: 1;
  }

  .flex-2 {
    flex: 2;
  }

  .req {
    color: #ef4444;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 18px;
    }
    .stats {
      grid-template-columns: 1fr;
    }
    .search {
      min-width: 0;
      width: 100%;
    }
    .form-row {
      flex-direction: column;
    }
  }

  @media (max-width: 640px) {
    .stats {
      grid-template-columns: 1fr;
    }
  }
</style>
