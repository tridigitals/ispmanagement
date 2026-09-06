<script lang="ts">
  /*
    Pool IP v2 — gelombang 24d (batch C kecil 2/2).

    Versi lama: (app)/admin/network/ip-pools/+page.svelte (584 baris).
    Perilaku identik: pilih router → daftar + sinkron → CRUD dialog →
    hapus berguard dependensi (warning vs danger). Helper ipPoolCrud +
    ipPoolOptions dipakai (ber-tes); dialog IpPoolFormDialog reuse.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import IpPoolFormDialog, {
    type IpPoolFormModel,
  } from '$lib/components/network/IpPoolFormDialog.svelte';
  import {
    getIpPoolCrudGateState,
    getIpPoolDeleteState,
    getIpPoolMutationErrorState,
    isIpPoolStaleTargetConflict,
  } from '$lib/utils/ipPoolCrud';
  import { ipPoolNextPoolOptions } from '$lib/utils/ipPoolOptions';
  import type { Column } from '$lib/components/ds/table-types';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    PageHeader,
  } from '$lib/components/ds';

  type RouterRow = { id: string; name: string };
  type IpPoolRow = {
    id: string;
    name: string;
    ranges?: string | null;
    next_pool?: string | null;
    comment?: string | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let rows = $state<IpPoolRow[]>([]);
  let showForm = $state(false);
  let editing = $state<IpPoolRow | null>(null);
  let form = $state<IpPoolFormModel>({ name: '', ranges: '', next_pool: '', comment: '' });
  let showDelete = $state(false);
  let deleteTarget = $state<IpPoolRow | null>(null);
  let deleteMessage = $state('');
  let deleteKeyword = $state('');
  let deleteDialogType = $state<'danger' | 'warning'>('danger');
  let deleteWarningCount = $state(0);

  const nextPoolOptions = $derived.by(() =>
    ipPoolNextPoolOptions(
      rows.map((r) => r.name),
      editing?.name || form.name,
    ),
  );

  const columns: Column[] = [
    { key: 'name', label: 'Nama' },
    { key: 'ranges', label: 'Rentang' },
    { key: 'next', label: 'Next pool' },
    { key: 'state', label: 'State' },
    { key: 'synced', label: 'Sinkron' },
    { key: 'actions', label: 'Aksi' },
  ];

  const canManage = $derived($can('manage', 'ip_pools'));

  onMount(async () => {
    if (!$can('read', 'ip_pools') && !$can('manage', 'ip_pools')) {
      goto('/unauthorized');
      return;
    }
    await loadRouters();
  });

  async function loadRouters() {
    loadingRouters = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      if (routerId) await load();
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal memuat router.');
    } finally {
      loadingRouters = false;
    }
  }

  async function load() {
    if (!routerId || loading) return;
    loading = true;
    try {
      rows = (await api.mikrotik.routers.ipPools(routerId)) as any;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal memuat pool.');
    } finally {
      loading = false;
    }
  }

  async function sync() {
    if (!routerId || loading) return;
    loading = true;
    try {
      rows = (await api.mikrotik.routers.syncIpPools(routerId)) as any;
      toast.success('Pool IP tersinkron.');
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal sinkron.');
    } finally {
      loading = false;
    }
  }

  function resetForm() {
    form = { name: '', ranges: '', next_pool: '', comment: '' };
  }

  function openCreate() {
    if (getIpPoolCrudGateState(routerId).blocked) {
      toast.error('Pilih router dulu.');
      return;
    }
    editing = null;
    resetForm();
    showForm = true;
  }

  function openEdit(row: IpPoolRow) {
    if (getIpPoolCrudGateState(routerId).blocked) {
      toast.error('Pilih router dulu.');
      return;
    }
    editing = row;
    form = {
      name: row.name || '',
      ranges: row.ranges || '',
      next_pool: row.next_pool || '',
      comment: row.comment || '',
    };
    showForm = true;
  }

  function normalizedPayload() {
    const normalize = (value: string) => {
      const trimmed = value.trim();
      return trimmed ? trimmed : null;
    };
    return {
      name: form.name.trim(),
      ranges: normalize(form.ranges),
      next_pool: normalize(form.next_pool),
      comment: normalize(form.comment),
    };
  }

  async function save() {
    if (!routerId) {
      toast.error('Pilih router dulu.');
      return;
    }
    saving = true;
    try {
      const payload = normalizedPayload();
      if (!payload.name && !editing) throw new Error('Nama pool wajib diisi.');
      if (editing) {
        await api.mikrotik.routers.updateIpPool(routerId, editing.id, payload);
        toast.success('Pool IP diperbarui.');
      } else {
        await api.mikrotik.routers.createIpPool(routerId, payload as any);
        toast.success('Pool IP dibuat.');
      }
      showForm = false;
      editing = null;
      await load();
    } catch (error) {
      const state = getIpPoolMutationErrorState(
        String((error as any)?.message || '').includes('mirror refresh failed')
          ? 'mirror_sync_failed'
          : 'router_write_failed',
      );
      if (state.tone === 'warning' && typeof toast.warning === 'function') {
        toast.warning(extractApiErrorMessage(error, state.message || ''));
      } else {
        toast.error(extractApiErrorMessage(error, state.message || ''));
      }
    } finally {
      saving = false;
    }
  }

  async function openDelete(row: IpPoolRow) {
    if (!routerId) {
      toast.error('Pilih router dulu.');
      return;
    }
    try {
      const dependency = await api.mikrotik.routers.ipPoolDependencies(routerId, row.id);
      const counts = Object.fromEntries((dependency.dependencies || []).map((item: any) => [item.type, item.count]));
      const state = getIpPoolDeleteState(counts as any);
      deleteTarget = row;
      deleteDialogType = state.warning ? 'warning' : 'danger';
      deleteWarningCount = state.totalDependencies;
      deleteKeyword = row.name;
      deleteMessage = state.warning
        ? `Hapus ${row.name}? ${state.totalDependencies} data internal masih merujuk pool ini.`
        : `Hapus pool IP ${row.name} dari router?`;
      showDelete = true;
    } catch (error) {
      toast.error(extractApiErrorMessage(error) || 'Gagal cek dependensi.');
    }
  }

  async function confirmDelete() {
    if (!routerId || !deleteTarget) return;
    deleting = true;
    try {
      const result = await api.mikrotik.routers.deleteIpPool(routerId, deleteTarget.id);
      showDelete = false;
      if ((result.warnings || []).some((item: any) => Number(item.count || 0) > 0) && typeof toast.warning === 'function') {
        toast.warning(`Pool ${deleteTarget.name} dihapus dengan ${deleteWarningCount} referensi peringatan.`);
      } else {
        toast.success('Pool IP dihapus.');
      }
      deleteTarget = null;
      deleteMessage = '';
      deleteKeyword = '';
      deleteWarningCount = 0;
      await load();
    } catch (error) {
      const message = extractApiErrorMessage(error) || 'Gagal hapus.';
      if (isIpPoolStaleTargetConflict(message)) {
        try {
          rows = (await api.mikrotik.routers.syncIpPools(routerId)) as any;
          showDelete = false;
          deleteTarget = null;
          deleteMessage = '';
          deleteKeyword = '';
          deleteWarningCount = 0;
          toast.warning('Pool sudah hilang di router — daftar disegarkan.');
          return;
        } catch (syncError) {
          toast.error(extractApiErrorMessage(syncError) || message);
          return;
        }
      }
      toast.error(message);
    } finally {
      deleting = false;
    }
  }

  function rowById(id: string): IpPoolRow {
    return rows.find((r) => (r.id || r.name) === id) ?? rows[0];
  }
</script>
<AppShell title="Pool IP">
  <PageHeader
    title="Pool IP"
    eyebrow="Jaringan"
    desc="Rentang alamat IP per router untuk distribusi PPPoE/DHCP."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={!routerId || loading}>Segarkan</Button>
      <Button variant="ghost" icon="download" onclick={() => void sync()} disabled={!routerId || loading}>Sinkron</Button>
      {#if canManage}
        <Button variant="primary" icon="plus" onclick={openCreate} disabled={!routerId || loading}>Tambah</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card title="Router">
    <div class="max-w-md">
      <Field stacked id="ip-router" label="Router" type="select" value={routerId} options={[{ value: '', label: 'Pilih router…' }, ...routers.map((r) => ({ value: r.id, label: r.name }))]} onchange={(v) => { routerId = v; void load(); }} />
    </div>
  </Card>

  {#if routerId}
    {#if rows.length === 0 && !loading}
      <Card><p class="py-10 text-center text-sm text-ink-500">Router ini belum punya pool IP tersinkron.</p></Card>
    {:else}
      <Card title={`Pool — ${rows.length} item`}>
        <DataTable
          {columns}
          rows={rows.map((r, idx) => ({
            id: r.id || `${r.name}:${idx}`,
            name: r.name,
            ranges: r.ranges || '—',
            next: r.next_pool || '—',
            state: Boolean(r.router_present),
            synced: r.last_sync_at,
          }))}
          emptyTitle="Belum ada pool"
        >
          {#snippet cell(row, col)}
            {@const cellVal = (row as unknown as Record<string, unknown>)[col.key] as string}
            {#if col.key === 'state'}
              <Badge tone={row.state ? 'positive' : 'warning'} label={row.state ? 'Ada' : 'Hilang'} />
            {:else if col.key === 'synced'}
              {#if row.synced}
                <span class="font-mono text-xs">{new Date(row.synced).toLocaleString('id-ID')}</span>
              {:else}
                <span class="text-ink-300">—</span>
              {/if}
            {:else if col.key === 'actions'}
              {#if canManage}
                <div class="flex gap-1">
                  <Button variant="ghost" onclick={() => openEdit(rowById(row.id))}>Ubah</Button>
                  <Button variant="ghost" onclick={() => void openDelete(rowById(row.id))}>Hapus</Button>
                </div>
              {/if}
            {:else if col.key === 'ranges' || col.key === 'next'}
              <span class="font-mono text-xs">{cellVal}</span>
            {:else}
              <span class="text-sm font-medium">{cellVal}</span>
            {/if}
          {/snippet}
        </DataTable>
      </Card>
    {/if}
  {/if}
</AppShell>

<IpPoolFormDialog
  bind:show={showForm}
  loading={saving}
  isEditing={Boolean(editing)}
  bind:pool={form}
  nextPoolOptions={nextPoolOptions}
  onSubmit={() => void save()}
/>

<ConfirmDialog
  bind:show={showDelete}
  type={deleteDialogType}
  title="Hapus pool IP"
  message={deleteMessage}
  confirmText="Hapus"
  confirmationKeyword={deleteKeyword}
  loading={deleting}
  onconfirm={() => void confirmDelete()}
  oncancel={() => {
    deleteTarget = null;
    deleteMessage = '';
    deleteWarningCount = 0;
  }}
/>
