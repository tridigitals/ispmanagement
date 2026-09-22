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
    EmptyState,
    PageHeader,
    TableSkeleton,
  } from '$lib/components/ds';
  import { t } from 'svelte-i18n';

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
    router_id?: string | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };
  type IpPoolRow = { id: string; name: string; router_id?: string | null };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');
  /* Default all-router (paritas PPPoE v2): '' = semua router tampil. */
  let routerFilter = $state('');

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let syncingAll = $state(false);
  let allRows = $state<PppProfileRow[]>([]);
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

  /* Nama router per id — untuk kolom Router saat mode all-router. */
  const routerNameById = $derived(new Map(routers.map((r) => [r.id, r.name])));

  /* Filter client-side ala PPPoE v2: '' = semua router. */
  const rows = $derived(
    routerFilter ? allRows.filter((r) => r.router_id === routerFilter) : allRows,
  );

  /* Hitung profil per router untuk option count di dropdown filter. */
  const routerCountById = $derived.by(() => {
    const m = new Map<string, number>();
    for (const r of allRows) {
      if (r.router_id) m.set(r.router_id, (m.get(r.router_id) ?? 0) + 1);
    }
    return m;
  });

  const columns = $derived<Column[]>([
    { key: 'name', label: $t('network.ppp_profiles.v2.col_name') },
    { key: 'rate', label: $t('network.ppp_profiles.v2.col_rate') },
    { key: 'remote', label: 'Remote', hideSm: true },
    { key: 'dns', label: 'DNS', hideSm: true },
    { key: 'router', label: $t('network.ppp_profiles.v2.col_router'), hideSm: true },
    { key: 'only_one', label: 'Only-one', hideSm: true },
    { key: 'state', label: 'State' },
    { key: 'actions', label: $t('network.ppp_profiles.v2.col_actions') },
  ]);

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
      await loadAll();
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || $t('network.ppp_profiles.v2.t_load_routers'));
    } finally {
      loadingRouters = false;
    }
  }

  /* Muat profil + pool dari SEMUA router (mirror DB, 1 request/router/router). */
  async function loadAll() {
    if (loading) return;
    if (routers.length === 0) {
      allRows = [];
      ipPools = [];
      return;
    }
    loading = true;
    try {
      const [profileResults, poolResults] = await Promise.all([
        Promise.allSettled(routers.map((r) => api.mikrotik.routers.pppProfiles(r.id))),
        Promise.allSettled(routers.map((r) => api.mikrotik.routers.ipPools(r.id))),
      ]);
      const merged: PppProfileRow[] = [];
      let failed = 0;
      for (let i = 0; i < profileResults.length; i++) {
        const res = profileResults[i];
        if (res.status === 'fulfilled') {
          for (const row of res.value as PppProfileRow[]) {
            merged.push({ ...row, router_id: row.router_id || routers[i].id });
          }
        } else {
          failed++;
        }
      }
      allRows = merged;
      const pools: IpPoolRow[] = [];
      for (let i = 0; i < poolResults.length; i++) {
        const res = poolResults[i];
        if (res.status === 'fulfilled') {
          for (const row of res.value as IpPoolRow[]) {
            pools.push({ ...row, id: row.id, router_id: (row as any).router_id || routers[i].id });
          }
        }
      }
      ipPools = pools;
      if (failed > 0) {
        toast.warning(
          $t('network.ppp_profiles.v2.t_load_partial', { values: { ok: routers.length - failed, failed } }),
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
        await api.mikrotik.routers.syncPppProfiles(targets[0]);
        toast.success($t('network.ppp_profiles.v2.t_synced'));
      } else {
        syncingAll = true;
        const results = await Promise.allSettled(
          targets.map((rid) => api.mikrotik.routers.syncPppProfiles(rid)),
        );
        const failed = results.filter((r) => r.status === 'rejected').length;
        if (failed === 0) toast.success($t('network.ppp_profiles.v2.t_synced'));
        else
          toast.warning(
            $t('network.ppp_profiles.v2.t_sync_partial', { values: { ok: targets.length - failed, failed } }),
          );
      }
      await loadAll();
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || $t('network.ppp_profiles.v2.t_sync_fail'));
    } finally {
      syncingAll = false;
      loading = false;
    }
  }

  function resetForm() {
    form = { name: '', local_address: '', remote_address: '', rate_limit: '', dns_server: '', comment: '', only_one: false };
  }

    /* Router target CRUD: mode all-router → pakai router milik row yang diedit.
     CRUD tetap per-router di backend. */
  function targetRouterId(row?: PppProfileRow | null): string {
    return routerFilter || row?.router_id || routerId || '';
  }

  function openCreate() {
    if (routers.length === 0) {
      toast.error($t('network.ppp_profiles.v2.pick_router_first'));
      return;
    }
    /* Default router: filter aktif → router terakhir dipakai → router pertama.
       Bisa diganti di dalam dialog. */
    if (!routerId) routerId = routerFilter || routers[0].id;
    editing = null;
    resetForm();
    showForm = true;
  }

  function openEdit(row: PppProfileRow) {
    const target = targetRouterId(row);
    if (!target) {
      toast.error($t('network.ppp_profiles.v2.pick_router_first'));
      return;
    }
    routerId = target;
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
      toast.error($t('network.ppp_profiles.v2.pick_router_first'));
      return;
    }
    saving = true;
    try {
      const payload = normalizePppProfilePayload(form);
      if (!payload.name && !editing) throw new Error('Nama profil wajib diisi.');
      if (editing) {
        await api.mikrotik.routers.updatePppProfile(routerId, editing.id, payload);
        toast.success($t('network.ppp_profiles.v2.t_updated'));
      } else {
        await api.mikrotik.routers.createPppProfile(routerId, payload as any);
        toast.success($t('network.ppp_profiles.v2.t_created'));
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
    const target = targetRouterId(row);
    if (!target) {
      toast.error($t('network.ppp_profiles.v2.pick_router_first'));
      return;
    }
    routerId = target;
    try {
      const dependency = await api.mikrotik.routers.pppProfileDependencies(target, row.id);
      const counts = Object.fromEntries((dependency.dependencies || []).map((item: any) => [item.type, item.count]));
      const state = getPppProfileDeleteState(counts as any);
      deleteTarget = row;
      deleteBlocked = state.blocked;
      deleteKeyword = state.blocked ? '__blocked__' : row.name;
      deleteMessage = state.blocked
        ? $t('network.ppp_profiles.v2.del_block', { values: { n: row.name, d: state.totalDependencies } })
        : $t('network.ppp_profiles.v2.del_msg', { values: { n: row.name } });
      showDelete = true;
    } catch (error) {
      toast.error(extractApiErrorMessage(error) || $t('network.ppp_profiles.v2.t_dep_fail'));
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
      toast.success($t('network.ppp_profiles.v2.t_deleted'));
      showDelete = false;
      deleteTarget = null;
      deleteBlocked = false;
      await load();
    } catch (error) {
      const message = extractApiErrorMessage(error) || $t('network.ppp_profiles.v2.t_del_fail');
      if (isPppProfileStaleTargetConflict(message)) {
        try {
          await api.mikrotik.routers.syncPppProfiles(routerId);
          await api.mikrotik.routers.ipPools(routerId);
          showDelete = false;
          deleteTarget = null;
          deleteMessage = '';
          deleteKeyword = '';
          deleteBlocked = false;
          toast.warning($t('network.ppp_profiles.v2.t_gone'));
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
<AppShell title={ $t('network.ppp_profiles.v2.title') }>
  <PageHeader
    title={ $t('network.ppp_profiles.v2.title') }
    eyebrow={ $t('admin.eyebrows.network') }
    desc={ $t('network.ppp_profiles.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void loadAll()} disabled={loading}>{ $t('common.refresh') }</Button>
      <Button variant="ghost" icon="download" onclick={() => void sync()} disabled={loading} loading={syncingAll}>{ $t('network.ppp_profiles.v2.sync') }</Button>
      {#if canManage}
        <Button variant="primary" icon="plus" onclick={openCreate} disabled={loading || routers.length === 0}>{ $t('network.ppp_profiles.v2.add') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if routers.length > 0}
    <div class="mb-4 flex flex-wrap items-center gap-2">
      <select
        bind:value={routerFilter}
        aria-label={ $t('network.ppp_profiles.v2.router') }
        class="h-9 rounded-lg bg-white px-3 text-sm text-ink-900 ring-1 ring-inset ring-ink-200 focus:ring-brand-600 focus:outline-none"
      >
        <option value="">{ $t('network.ppp_profiles.v2.all_routers', { values: { n: allRows.length } }) }</option>
        {#each routers as r (r.id)}
          <option value={r.id}>{r.name} ({routerCountById.get(r.id) ?? 0})</option>
        {/each}
      </select>
    </div>
  {/if}

  {#if loadingRouters}
    <Card padded={false}><TableSkeleton rows={5} cols={7} /></Card>
  {:else if routers.length === 0}
    <Card>
      <EmptyState
        icon="server"
        title={ $t('network.ppp_profiles.v2.no_routers_title') }
        hint={ $t('network.ppp_profiles.v2.no_routers_hint') }
      />
      {#if canManage}
        <div class="flex justify-center pb-6">
          <Button variant="primary" icon="plus" href="/v2/admin/network/routers">{ $t('network.ppp_profiles.v2.go_routers') }</Button>
        </div>
      {/if}
    </Card>
  {:else if !loading && rows.length === 0}
    <Card>
      <EmptyState
        icon="users"
        title={ $t('network.ppp_profiles.v2.empty') }
        hint={ $t('network.ppp_profiles.v2.empty_hint') }
      />
      {#if canManage}
        <div class="flex justify-center pb-6">
          <Button variant="primary" icon="plus" onclick={openCreate}>{ $t('network.ppp_profiles.v2.add') }</Button>
        </div>
      {/if}
    </Card>
  {:else}
    <Card title={ $t('network.ppp_profiles.v2.card_title', { values: { n: rows.length } }) }>
      <DataTable pageSize={25}
        {columns}
        rows={rows.map((r, idx) => ({
          id: r.id || `${r.name}:${idx}`,
          name: r.name,
          remote: r.remote_address || '—',
          rate: r.rate_limit || '—',
          dns: r.dns_server || '—',
          router: r.router_id ? (routerNameById.get(r.router_id) ?? r.router_id) : '—',
          only_one: getPppProfileOnlyOneState(r.only_one).enabled,
          state: Boolean(r.router_present),
          synced: r.last_sync_at,
        }))}
        emptyTitle="Belum ada profil"
      >
        {#snippet cell(row, col)}
          {@const cellVal = (row as unknown as Record<string, unknown>)[col.key] as string}
          {#if col.key === 'state'}
            <Badge tone={row.state ? 'positive' : 'warning'} label={row.state ? $t('network.ppp_profiles.v2.state_present') : $t('network.ppp_profiles.v2.state_missing')} />
          {:else if col.key === 'router'}
            <span class="text-xs text-ink-500">{cellVal}</span>
          {:else if col.key === 'only_one'}
            <Badge tone={row.only_one ? 'positive' : 'neutral'} label={row.only_one ? $t('common.yes') : $t('common.no')} />
          {:else if col.key === 'synced'}
            {#if row.synced}
              <span class="font-mono text-xs">{new Date(row.synced).toLocaleString('id-ID')}</span>
            {:else}
              <span class="text-ink-300">—</span>
            {/if}
          {:else if col.key === 'actions'}
            {#if canManage}
              <div class="flex gap-1">
                <Button variant="ghost" onclick={() => openEdit(rows.find((r) => (r.id || r.name) === row.id) ?? rows[0])}>{ $t('network.ppp_profiles.v2.edit') }</Button>
                <Button variant="ghost" onclick={() => void openDelete(rows.find((r) => (r.id || r.name) === row.id) ?? rows[0])}>{ $t('network.ppp_profiles.v2.delete') }</Button>
              </div>
            {/if}
          {:else if col.key === 'remote' || col.key === 'rate' || col.key === 'dns'}
            <span class="font-mono text-xs">{cellVal}</span>
          {:else}
            <span class="text-sm font-medium">{cellVal}</span>
          {/if}
        {/snippet}
      </DataTable>
    </Card>
  {/if}
</AppShell>

<PppProfileFormDialog
  bind:show={showForm}
  loading={saving}
  isEditing={Boolean(editing)}
  bind:profile={form}
  remotePoolOptions={remotePoolOptions}
  routers={routers}
  bind:routerId
  onSubmit={() => void save()}
/>

<ConfirmDialog
  bind:show={showDelete}
  title={ $t('network.ppp_profiles.v2.del_title') }
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
