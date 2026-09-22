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
    EmptyState,
    PageHeader,
    TableSkeleton,
  } from '$lib/components/ds';
  import { t } from 'svelte-i18n';

  type RouterRow = { id: string; name: string };
  type IpPoolRow = {
    id: string;
    name: string;
    ranges?: string | null;
    next_pool?: string | null;
    comment?: string | null;
    router_id?: string | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');
  /* Default all-router (paritas PPPoE v2): '' = semua router tampil. */
  let routerFilter = $state('');

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let syncingAll = $state(false);
  let allRows = $state<IpPoolRow[]>([]);
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

  const columns = $derived<Column[]>([
    { key: 'name', label: $t('network.ip_pools.v2.col_name') },
    { key: 'ranges', label: $t('network.ip_pools.v2.col_ranges') },
    { key: 'next', label: $t('network.ip_pools.v2.col_next') },
    { key: 'router', label: $t('network.ip_pools.v2.col_router'), hideSm: true },
    { key: 'state', label: $t('network.ip_pools.v2.col_state') },
    { key: 'synced', label: $t('network.ip_pools.v2.col_synced') },
    { key: 'actions', label: $t('network.ip_pools.v2.col_actions') },
  ]);

  const canManage = $derived($can('manage', 'ip_pools'));

  /* Nama router per id — untuk kolom Router saat mode all-router. */
  const routerNameById = $derived(new Map(routers.map((r) => [r.id, r.name])));

  /* Filter client-side ala PPPoE v2: '' = semua router. */
  const rows = $derived(
    routerFilter ? allRows.filter((r) => r.router_id === routerFilter) : allRows,
  );

  /* Hitung pool per router untuk option count di dropdown filter. */
  const routerCountById = $derived.by(() => {
    const m = new Map<string, number>();
    for (const r of allRows) {
      if (r.router_id) m.set(r.router_id, (m.get(r.router_id) ?? 0) + 1);
    }
    return m;
  });

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
      await loadAll();
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || $t('network.ip_pools.v2.t_load_routers'));
    } finally {
      loadingRouters = false;
    }
  }

  /* Tarik mirror pool dari SEMUA router sekaligus (paritas PPPoE v2:
     fetchAllPages-ish; di sini mirror DB per router, 1 request/router). */
  async function loadAll() {
    if (loading || routers.length === 0) {
      if (routers.length === 0) allRows = [];
      return;
    }
    loading = true;
    try {
      const results = await Promise.allSettled(
        routers.map((r) => api.mikrotik.routers.ipPools(r.id)),
      );
      const merged: IpPoolRow[] = [];
      let failed = 0;
      for (let i = 0; i < results.length; i++) {
        const res = results[i];
        if (res.status === 'fulfilled') {
          for (const row of res.value as IpPoolRow[]) {
            merged.push({ ...row, router_id: row.router_id || routers[i].id });
          }
        } else {
          failed++;
        }
      }
      allRows = merged;
      if (failed > 0) {
        toast.warning(
          $t('network.ip_pools.v2.t_load_partial', { values: { ok: routers.length - failed, failed } }),
        );
      }
    } finally {
      loading = false;
    }
  }

  async function load() {
    /* Kompatibilitas panggilan lama (setelah create/edit/delete) —
       sekarang selalu muat ulang seluruh tenant. */
    await loadAll();
  }

  async function sync() {
    /* Sinkron router terpilih; di mode all-router tanpa filter, sinkron semua. */
    const targets = routerFilter ? [routerFilter] : routers.map((r) => r.id);
    if (targets.length === 0 || loading) return;
    loading = true;
    try {
      if (targets.length === 1) {
        await api.mikrotik.routers.syncIpPools(targets[0]);
        toast.success($t('network.ip_pools.v2.t_synced'));
      } else {
        syncingAll = true;
        const results = await Promise.allSettled(
          targets.map((rid) => api.mikrotik.routers.syncIpPools(rid)),
        );
        const failed = results.filter((r) => r.status === 'rejected').length;
        if (failed === 0) toast.success($t('network.ip_pools.v2.t_synced'));
        else
          toast.warning(
            $t('network.ip_pools.v2.t_sync_partial', { values: { ok: targets.length - failed, failed } }),
          );
      }
      await loadAll();
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || $t('network.ip_pools.v2.t_sync_fail'));
    } finally {
      syncingAll = false;
      loading = false;
    }
  }

  function resetForm() {
    form = { name: '', ranges: '', next_pool: '', comment: '' };
  }

  /* Router target CRUD: mode all-router → pakai router milik row yang diedit
     / row pertama untuk create. CRUD tetap per-router di backend. */
  function targetRouterId(row?: IpPoolRow | null): string {
    return routerFilter || row?.router_id || routerId || '';
  }

  function openCreate() {
    if (routers.length === 0) {
      toast.error($t('network.ip_pools.v2.pick_router_first'));
      return;
    }
    /* Default router: filter aktif → router terakhir dipakai → router pertama.
       Bisa diganti di dalam dialog. */
    if (!routerId) routerId = routerFilter || routers[0].id;
    editing = null;
    resetForm();
    showForm = true;
  }

  function openEdit(row: IpPoolRow) {
    const target = targetRouterId(row);
    if (!target) {
      toast.error($t('network.ip_pools.v2.pick_router_first'));
      return;
    }
    routerId = target;
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
      toast.error($t('network.ip_pools.v2.pick_router_first'));
      return;
    }
    saving = true;
    try {
      const payload = normalizedPayload();
      if (!payload.name && !editing) throw new Error('Nama pool wajib diisi.');
      if (editing) {
        await api.mikrotik.routers.updateIpPool(routerId, editing.id, payload);
        toast.success($t('network.ip_pools.v2.t_updated'));
      } else {
        await api.mikrotik.routers.createIpPool(routerId, payload as any);
        toast.success($t('network.ip_pools.v2.t_created'));
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
    const target = targetRouterId(row);
    if (!target) {
      toast.error($t('network.ip_pools.v2.pick_router_first'));
      return;
    }
    routerId = target;
    try {
      const dependency = await api.mikrotik.routers.ipPoolDependencies(target, row.id);
      const counts = Object.fromEntries((dependency.dependencies || []).map((item: any) => [item.type, item.count]));
      const state = getIpPoolDeleteState(counts as any);
      deleteTarget = row;
      deleteDialogType = state.warning ? 'warning' : 'danger';
      deleteWarningCount = state.totalDependencies;
      deleteKeyword = row.name;
      deleteMessage = state.warning
        ? $t('network.ip_pools.v2.del_dep', { values: { n: row.name, d: state.totalDependencies } })
        : $t('network.ip_pools.v2.del_msg', { values: { n: row.name } });
      showDelete = true;
    } catch (error) {
      toast.error(extractApiErrorMessage(error) || $t('network.ip_pools.v2.t_dep_fail'));
    }
  }

  async function confirmDelete() {
    if (!routerId || !deleteTarget) return;
    deleting = true;
    try {
      const result = await api.mikrotik.routers.deleteIpPool(routerId, deleteTarget.id);
      showDelete = false;
      if ((result.warnings || []).some((item: any) => Number(item.count || 0) > 0) && typeof toast.warning === 'function') {
        toast.warning($t('network.ip_pools.v2.t_del_warn', { values: { n: deleteTarget.name, c: deleteWarningCount } }));
      } else {
        toast.success($t('network.ip_pools.v2.t_deleted'));
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
          await api.mikrotik.routers.syncIpPools(routerId);
          showDelete = false;
          deleteTarget = null;
          deleteMessage = '';
          deleteKeyword = '';
          deleteWarningCount = 0;
          toast.warning($t('network.ip_pools.v2.t_gone'));
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
<AppShell title={ $t('network.ip_pools.v2.title') }>
  <PageHeader
    title={ $t('network.ip_pools.v2.title') }
    eyebrow={ $t('admin.eyebrows.network') }
    desc={ $t('network.ip_pools.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void loadAll()} disabled={loading}>{ $t('common.refresh') }</Button>
      <Button variant="ghost" icon="download" onclick={() => void sync()} disabled={loading} loading={syncingAll}>{ $t('network.ip_pools.v2.col_synced') }</Button>
      {#if canManage}
        <Button variant="primary" icon="plus" onclick={openCreate} disabled={loading || routers.length === 0}>{ $t('network.ip_pools.v2.add') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if routers.length > 0}
    <div class="mb-4 flex flex-wrap items-center gap-2">
      <select
        bind:value={routerFilter}
        onchange={undefined}
        aria-label={ $t('network.ip_pools.v2.router') }
        class="h-9 rounded-lg bg-white px-3 text-sm text-ink-900 ring-1 ring-inset ring-ink-200 focus:ring-brand-600 focus:outline-none"
      >
        <option value="">{ $t('network.ip_pools.v2.all_routers', { values: { n: allRows.length } }) }</option>
        {#each routers as r (r.id)}
          <option value={r.id}>{r.name} ({routerCountById.get(r.id) ?? 0})</option>
        {/each}
      </select>
    </div>
  {/if}

  {#if loadingRouters}
    <Card padded={false}><TableSkeleton rows={5} cols={6} /></Card>
  {:else if routers.length === 0}
    <Card>
      <EmptyState
        icon="server"
        title={ $t('network.ip_pools.v2.no_routers_title') }
        hint={ $t('network.ip_pools.v2.no_routers_hint') }
      />
      {#if canManage}
        <div class="flex justify-center pb-6">
          <Button variant="primary" icon="plus" href="/v2/admin/network/routers">{ $t('network.ip_pools.v2.go_routers') }</Button>
        </div>
      {/if}
    </Card>
  {:else if !loading && rows.length === 0}
    <Card>
      <EmptyState
        icon="database"
        title={ $t('network.ip_pools.v2.empty_no_pool') }
        hint={ $t('network.ip_pools.v2.empty_pool_hint') }
      />
      {#if canManage}
        <div class="flex justify-center pb-6">
          <Button variant="primary" icon="plus" onclick={openCreate}>{ $t('network.ip_pools.v2.add') }</Button>
        </div>
      {/if}
    </Card>
  {:else}
    <Card title={ $t('network.ip_pools.v2.card_pool', { values: { n: rows.length } }) }>
      <DataTable pageSize={25}
          {columns}
          rows={rows.map((r, idx) => ({
            id: r.id || `${r.name}:${idx}`,
            name: r.name,
            ranges: r.ranges || '—',
            next: r.next_pool || '—',
            router: r.router_id ? (routerNameById.get(r.router_id) ?? r.router_id) : '—',
            state: Boolean(r.router_present),
            synced: r.last_sync_at,
          }))}
          emptyTitle="Belum ada pool"
        >
          {#snippet cell(row, col)}
            {@const cellVal = (row as unknown as Record<string, unknown>)[col.key] as string}
            {#if col.key === 'state'}
              <Badge tone={row.state ? 'positive' : 'warning'} label={row.state ? $t('network.ip_pools.v2.state_present') : $t('network.ip_pools.v2.state_missing')} />
            {:else if col.key === 'router'}
              <span class="text-xs text-ink-500">{cellVal}</span>
            {:else if col.key === 'synced'}
              {#if row.synced}
                <span class="font-mono text-xs">{new Date(row.synced).toLocaleString('id-ID')}</span>
              {:else}
                <span class="text-ink-300">—</span>
              {/if}
            {:else if col.key === 'actions'}
              {#if canManage}
                <div class="flex gap-1">
                  <Button variant="ghost" onclick={() => openEdit(rowById(row.id))}>{ $t('network.ip_pools.v2.edit') }</Button>
                  <Button variant="ghost" onclick={() => void openDelete(rowById(row.id))}>{ $t('network.ip_pools.v2.delete') }</Button>
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
</AppShell>

<IpPoolFormDialog
  bind:show={showForm}
  loading={saving}
  isEditing={Boolean(editing)}
  bind:pool={form}
  nextPoolOptions={nextPoolOptions}
  routers={routers}
  bind:routerId
  onSubmit={() => void save()}
/>

<ConfirmDialog
  bind:show={showDelete}
  type={deleteDialogType}
  title={ $t('network.ip_pools.v2.del_title') }
  message={deleteMessage}
  confirmText={ $t('network.ip_pools.v2.delete') }
  confirmationKeyword={deleteKeyword}
  loading={deleting}
  onconfirm={() => void confirmDelete()}
  oncancel={() => {
    deleteTarget = null;
    deleteMessage = '';
    deleteWarningCount = 0;
  }}
/>
