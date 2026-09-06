<script lang="ts">
  /*
    Profil PPP v2 — gelombang 24d (batch C kecil 1/2).

    Versi lama: (app)/admin/network/ppp-profiles/+page.svelte (643 baris).
    Perilaku identik: pilih router → daftar + sinkron → CRUD dialog →
    hapus berguard dependensi. Helper pppProfileCrud + pppProfileRemotePool
    dipakai ulang (sudah ber-tes); dialog PppProfileFormDialog reuse.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import PppProfileFormDialog, {
    type PppProfileFormModel,
  } from '$lib/components/network/PppProfileFormDialog.svelte';
  import {
    getPppProfileCrudGateState,
    getPppProfileDeleteState,
    getPppProfileMutationErrorState,
    getPppProfileOnlyOneState,
    isPppProfileStaleTargetConflict,
    normalizePppProfilePayload,
  } from '$lib/utils/pppProfileCrud';
  import {
    getPppProfileRemotePoolOptions,
    getPppProfileRemotePoolValue,
  } from '$lib/utils/pppProfileRemotePool';
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
  type PppProfileRow = {
    id: string;
    name: string;
    local_address?: string | null;
    remote_address?: string | null;
    rate_limit?: string | null;
    dns_server?: string | null;
    comment?: string | null;
    only_one?: boolean | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };
  type IpPoolRow = { id: string; name: string };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let rows = $state<PppProfileRow[]>([]);
  let ipPools = $state<IpPoolRow[]>([]);
  let showForm = $state(false);
  let editing = $state<PppProfileRow | null>(null);
  let form = $state<PppProfileFormModel>({
    name: '', local_address: '', remote_address: '', rate_limit: '', dns_server: '', comment: '', only_one: false,
  });
  let showDelete = $state(false);
  let deleteTarget = $state<PppProfileRow | null>(null);
  let deleteMessage = $state('');
  let deleteKeyword = $state('');
  let deleteBlocked = $state(false);

  const remotePoolOptions = $derived.by(() => getPppProfileRemotePoolOptions(ipPools));

  const columns: Column[] = [
    { key: 'name', label: 'Nama' },
    { key: 'local', label: 'Lokal' },
    { key: 'remote', label: 'Remote' },
    { key: 'rate', label: 'Rate limit' },
    { key: 'dns', label: 'DNS' },
    { key: 'only_one', label: 'Only-one' },
    { key: 'state', label: 'State' },
    { key: 'synced', label: 'Sinkron' },
    { key: 'actions', label: 'Aksi' },
  ];

  const canManage = $derived($can('manage', 'ppp_profiles'));

  onMount(async () => {
    if (!$can('read', 'ppp_profiles') && !$can('manage', 'ppp_profiles')) {
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
      const [profileRows, poolRows] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      rows = profileRows as any;
      ipPools = poolRows as any;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal memuat profil.');
    } finally {
      loading = false;
    }
  }

  async function sync() {
    if (!routerId || loading) return;
    loading = true;
    try {
      rows = (await api.mikrotik.routers.syncPppProfiles(routerId)) as any;
      ipPools = (await api.mikrotik.routers.ipPools(routerId)) as any;
      toast.success('Profil PPP tersinkron.');
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal sinkron.');
    } finally {
      loading = false;
    }
  }

  function resetForm() {
    form = { name: '', local_address: '', remote_address: '', rate_limit: '', dns_server: '', comment: '', only_one: false };
  }

  function openCreate() {
    if (getPppProfileCrudGateState(routerId).blocked) {
      toast.error('Pilih router dulu.');
      return;
    }
    editing = null;
    resetForm();
    showForm = true;
  }

  function openEdit(row: PppProfileRow) {
    if (getPppProfileCrudGateState(routerId).blocked) {
      toast.error('Pilih router dulu.');
      return;
    }
    editing = row;
    form = {
      name: row.name || '',
      local_address: row.local_address || '',
      remote_address: getPppProfileRemotePoolValue(remotePoolOptions, row.remote_address),
      rate_limit: row.rate_limit || '',
      dns_server: row.dns_server || '',
      comment: row.comment || '',
      only_one: Boolean(row.only_one),
    };
    showForm = true;
  }

  async function save() {
    if (!routerId) {
      toast.error('Pilih router dulu.');
      return;
    }
    saving = true;
    try {
      const payload = normalizePppProfilePayload(form);
      if (!payload.name && !editing) throw new Error('Nama profil wajib diisi.');
      if (editing) {
        await api.mikrotik.routers.updatePppProfile(routerId, editing.id, payload);
        toast.success('Profil PPP diperbarui.');
      } else {
        await api.mikrotik.routers.createPppProfile(routerId, payload as any);
        toast.success('Profil PPP dibuat.');
      }
      showForm = false;
      editing = null;
      await load();
    } catch (error) {
      const state = getPppProfileMutationErrorState(
        String((error as any)?.message || '').includes('mirror') ? 'mirror_sync_failed' : 'router_write_failed',
      );
      toast.error(extractApiErrorMessage(error, state.message || ''));
    } finally {
      saving = false;
    }
  }

  async function openDelete(row: PppProfileRow) {
    if (!routerId) {
      toast.error('Pilih router dulu.');
      return;
    }
    try {
      const dependency = await api.mikrotik.routers.pppProfileDependencies(routerId, row.id);
      const counts = Object.fromEntries((dependency.dependencies || []).map((item: any) => [item.type, item.count]));
      const state = getPppProfileDeleteState(counts as any);
      deleteTarget = row;
      deleteBlocked = state.blocked;
      deleteKeyword = state.blocked ? '__blocked__' : row.name;
      deleteMessage = state.blocked
        ? `Tidak bisa hapus ${row.name} — masih dipakai ${state.totalDependencies} data.`
        : `Hapus profil PPP ${row.name} dari router?`;
      showDelete = true;
    } catch (error) {
      toast.error(extractApiErrorMessage(error) || 'Gagal cek dependensi.');
    }
  }

  async function confirmDelete() {
    if (!routerId || !deleteTarget) return;
    if (deleteBlocked) {
      toast.error(deleteMessage);
      return;
    }
    deleting = true;
    try {
      await api.mikrotik.routers.deletePppProfile(routerId, deleteTarget.id);
      toast.success('Profil PPP dihapus.');
      showDelete = false;
      deleteTarget = null;
      deleteBlocked = false;
      await load();
    } catch (error) {
      const message = extractApiErrorMessage(error) || 'Gagal hapus.';
      if (isPppProfileStaleTargetConflict(message)) {
        try {
          rows = (await api.mikrotik.routers.syncPppProfiles(routerId)) as any;
          ipPools = (await api.mikrotik.routers.ipPools(routerId)) as any;
          showDelete = false;
          deleteTarget = null;
          deleteMessage = '';
          deleteKeyword = '';
          deleteBlocked = false;
          toast.warning('Profil sudah hilang di router — daftar disegarkan.');
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

</script>
<AppShell title="Profil PPP">
  <PageHeader
    title="Profil PPP"
    eyebrow="Jaringan"
    desc="Template kecepatan & alamat untuk akun PPPoE per router."
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
      <Field stacked id="pp-router" label="Router" type="select" value={routerId} options={[{ value: '', label: 'Pilih router…' }, ...routers.map((r) => ({ value: r.id, label: r.name }))]} onchange={(v) => { routerId = v; void load(); }} />
    </div>
  </Card>

  {#if routerId}
    {#if rows.length === 0 && !loading}
      <Card><p class="py-10 text-center text-sm text-ink-500">Router ini belum punya profil PPP tersinkron.</p></Card>
    {:else}
    <Card title={`Profil — ${rows.length} item`}>
      <DataTable
        {columns}
        rows={rows.map((r, idx) => ({
          id: r.id || `${r.name}:${idx}`,
          name: r.name,
          local: r.local_address || '—',
          remote: r.remote_address || '—',
          rate: r.rate_limit || '—',
          dns: r.dns_server || '—',
          only_one: getPppProfileOnlyOneState(r.only_one).enabled,
          state: Boolean(r.router_present),
          synced: r.last_sync_at,
        }))}
        emptyTitle="Belum ada profil"
      >
        {#snippet cell(row, col)}
          {@const cellVal = (row as unknown as Record<string, unknown>)[col.key] as string}
          {#if col.key === 'state'}
            <Badge tone={row.state ? 'positive' : 'warning'} label={row.state ? 'Ada' : 'Hilang'} />
          {:else if col.key === 'only_one'}
            <Badge tone={row.only_one ? 'positive' : 'neutral'} label={row.only_one ? 'Ya' : 'Tidak'} />
          {:else if col.key === 'synced'}
            {#if row.synced}
              <span class="font-mono text-xs">{new Date(row.synced).toLocaleString('id-ID')}</span>
            {:else}
              <span class="text-ink-300">—</span>
            {/if}
          {:else if col.key === 'actions'}
            {#if canManage}
              <div class="flex gap-1">
                <Button variant="ghost" onclick={() => openEdit(rows.find((r) => (r.id || r.name) === row.id) ?? rows[0])}>Ubah</Button>
                <Button variant="ghost" onclick={() => void openDelete(rows.find((r) => (r.id || r.name) === row.id) ?? rows[0])}>Hapus</Button>
              </div>
            {/if}
          {:else if col.key === 'local' || col.key === 'remote' || col.key === 'rate' || col.key === 'dns'}
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

<PppProfileFormDialog
  bind:show={showForm}
  loading={saving}
  isEditing={Boolean(editing)}
  bind:profile={form}
  remotePoolOptions={remotePoolOptions}
  onSubmit={() => void save()}
/>

<ConfirmDialog
  bind:show={showDelete}
  title="Hapus profil PPP"
  message={deleteMessage}
  confirmText="Hapus"
  confirmationKeyword={deleteKeyword}
  loading={deleting}
  onconfirm={() => void confirmDelete()}
  oncancel={() => {
    deleteTarget = null;
    deleteMessage = '';
    deleteBlocked = false;
  }}
/>
