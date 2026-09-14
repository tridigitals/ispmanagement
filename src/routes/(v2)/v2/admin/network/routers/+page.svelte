<script lang="ts">
  /*
    Router v2.

    Versi lama: `(app)/admin/network/routers/+page.svelte` — 713 baris,
    320 script + 225 CSS scoped, dengan satu bug yang membuat layar berbohong:
    status diturunkan HANYA dari `is_online`, padahal poller backend memfilter
    `WHERE enabled = true` (`mikrotik_service.rs:2349`). Router yang
    dinonaktifkan berhenti diperbarui dan `is_online` membeku pada nilai
    terakhirnya.

    Terbukti pada data hidup: "Solikin" `enabled=false`, `is_online=true`,
    `latency_ms=65`, `last_seen_at` 28 hari lalu → layar lama menampilkan badge
    hijau $t('admin.network.routers.v2d.online') dan "65 ms", ringkasan "Online 3 dari 3".

    Semua logika status pindah ke `$lib/utils/routerStatus` (13 tes unit) supaya
    aturannya bisa diuji tanpa merender halaman.
  */
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { get as getStore } from 'svelte/store';
  import { page as pageStore } from '$app/stores';
  import { api } from '$lib/api/client';
  import RouterFormModal from '$lib/components/network/RouterFormModal.svelte';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import {
    AppShell,
    AttentionPanel,
    Badge,
    Button,
    Card,
    DataTable,
    Icon,
    PageHeader,
    RowActions,
    StatTile,
    type Column,
  } from '$lib/components/ds';
  import {
    POLL_INTERVAL_MS,
    humanAge,
    routerStatus,
    statusTone,
    summarize,
    type RouterLike,
  } from '$lib/utils/routerStatus';
  import { t } from 'svelte-i18n';

  type Row = RouterLike & {
    id: string;
    name: string;
    host: string;
    port: number;
    username: string;
    identity?: string | null;
    ros_version?: string | null;
    latitude?: number | string | null;
    longitude?: number | string | null;
  };

  let rows = $state<Row[]>([]);
  let loading = $state(true);
  let search = $state('');
  let now = $state(Date.now());
  let testing = $state<string | null>(null);
  let tick: ReturnType<typeof setInterval> | null = null;

  /* ── form tambah / ubah router ──────────────────────────────────────── */
  let formShow = $state(false);
  let formSaving = $state(false);
  let editing: Row | null = $state(null);
  let formName = $state('');
  let formHost = $state('');
  let formPort = $state<number>(8728);
  let formUsername = $state('');
  let formPassword = $state('');
  let formLatitude = $state('');
  let formLongitude = $state('');
  let formEnabled = $state(true);
  let formMaintenanceEnabled = $state(false);
  let formMaintenanceUntilLocal = $state('');
  let formMaintenanceReason = $state('');

  function isoToLocalInput(iso?: string | null) {
    if (!iso) return '';
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function localInputToIso(local: string): string | null {
    if (!local) return null;
    const d = new Date(local);
    return Number.isNaN(d.getTime()) ? null : d.toISOString();
  }

  function resetForm() {
    editing = null;
    formName = '';
    formHost = '';
    formPort = 8728;
    formUsername = '';
    formPassword = '';
    formLatitude = '';
    formLongitude = '';
    formEnabled = true;
    formMaintenanceEnabled = false;
    formMaintenanceUntilLocal = '';
    formMaintenanceReason = '';
  }

  function openCreate() {
    resetForm();
    formShow = true;
  }

  function openEdit(r: Row) {
    editing = r;
    formName = r.name || '';
    formHost = r.host || '';
    formPort = Number(r.port || 8728);
    formUsername = r.username || '';
    formPassword = '';
    formLatitude = r.latitude != null ? String(r.latitude) : '';
    formLongitude = r.longitude != null ? String(r.longitude) : '';
    formEnabled = r.enabled ?? true;
    const untilIso = r.maintenance_until ?? null;
    const untilMs = untilIso ? new Date(untilIso).getTime() : NaN;
    formMaintenanceEnabled = Number.isFinite(untilMs) ? untilMs > Date.now() : false;
    formMaintenanceUntilLocal = isoToLocalInput(untilIso);
    formMaintenanceReason = r.maintenance_reason || '';
    formShow = true;
  }

  async function saveForm() {
    const name = formName.trim();
    const host = formHost.trim();
    if (!name || !host || !formUsername.trim()) {
      toast.error($t('common.validation_error'));
      return;
    }
    if (!editing && !formPassword.trim()) {
      toast.error($t('admin.network.routers.form.password'));
      return;
    }
    const latRaw = formLatitude.trim();
    const lngRaw = formLongitude.trim();
    const parsedLat = latRaw ? Number(latRaw) : NaN;
    const parsedLng = lngRaw ? Number(lngRaw) : NaN;
    if (latRaw && (Number.isNaN(parsedLat) || parsedLat < -90 || parsedLat > 90)) {
      toast.error($t('network.map.latitude_range_error'));
      return;
    }
    if (lngRaw && (Number.isNaN(parsedLng) || parsedLng < -180 || parsedLng > 180)) {
      toast.error($t('network.map.longitude_range_error'));
      return;
    }
    const latitude = latRaw ? parsedLat : null;
    const longitude = lngRaw ? parsedLng : null;
    formSaving = true;
    try {
      const maintenance_until = formMaintenanceEnabled ? localInputToIso(formMaintenanceUntilLocal) : null;
      const maintenance_reason = formMaintenanceEnabled ? formMaintenanceReason.trim() || null : null;
      if (editing) {
        await api.mikrotik.routers.update(editing.id, {
          name,
          host,
          port: formPort,
          username: formUsername.trim(),
          password: formPassword.trim() ? formPassword : undefined,
          enabled: formEnabled,
          maintenance_until,
          maintenance_reason,
          latitude,
          longitude,
        });
        toast.success($t('admin.network.routers.toasts.updated'));
      } else {
        await api.mikrotik.routers.create({
          name,
          host,
          port: formPort,
          username: formUsername.trim(),
          password: formPassword,
          enabled: formEnabled,
          maintenance_until,
          maintenance_reason,
          latitude,
          longitude,
        });
        toast.success($t('admin.network.routers.toasts.created'));
      }
      formShow = false;
      await load();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      formSaving = false;
    }
  }

  /* Tautan ?new=1 / ?edit=<id> dari Beranda & menu baris. */
  function applyDeepLink() {
    const sp = getStore(pageStore).url.searchParams;
    if (!$can('manage', 'router_inventory')) return;
    if (sp.get('new') === '1') {
      openCreate();
      goto('/v2/admin/network/routers', { replaceState: true, keepFocus: true });
      return;
    }
    const editId = sp.get('edit');
    if (editId) {
      const r = rows.find((x) => x.id === editId);
      if (r) openEdit(r);
      goto('/v2/admin/network/routers', { replaceState: true, keepFocus: true });
    }
  }

  const stats = $derived(summarize(rows, now));

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((r) =>
      `${r.name} ${r.host} ${r.identity ?? ''}`.toLowerCase().includes(q),
    );
  });

  /* Router yang butuh perhatian: dinonaktifkan tapi masih mengaku online, atau
     aktif tapi datanya usang. Versi lama tidak punya cara menampilkan ini. */
  const perluPerhatian = $derived.by(() =>
    rows
      .map((r) => ({ row: r, status: routerStatus(r, now, tr) }))
      .filter((x) => x.status.state === 'stale' || x.status.state === 'disabled')
      .map((x) => ({
        icon: x.status.state === 'disabled' ? ('lock' as const) : ('clock' as const),
        title: `${x.row.name} — ${x.status.label.toLowerCase()}`,
        detail: x.status.reason,
        action: $t('common.open'),
        href: `/v2/admin/network/routers/${x.row.id}`,
        severity: (x.status.state === 'stale' ? 'high' : 'medium') as 'high' | 'medium',
      })),
  );

  const tr = (k: string, v?: Record<string, string | number>) => $t(k, v ? { values: v } : undefined);

  const columns = $derived<Column[]>([
    { key: 'name', label: $t('admin.network.routers.columns.name') },
    { key: 'status', label: $t('common.status') },
    { key: 'latency', label: $t('admin.network.routers.v2list.latency') },
    { key: 'seen', label: $t('common.last_data') },
    { key: 'actions', label: '', align: 'right', width: '150px' },
  ]);

  async function load() {
    loading = true;
    try {
      rows = (await api.mikrotik.routers.list()) as Row[];
      now = Date.now();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    try {
      rows = (await api.mikrotik.routers.list()) as Row[];
    } catch {
      /* diam: ini penyegaran latar, bukan aksi pengguna */
    }
    now = Date.now();
  }

  async function uji(r: Row) {
    testing = r.id;
    try {
      const res = (await api.mikrotik.routers.test(r.id)) as {
        ok?: boolean;
        identity?: string;
        ros_version?: string;
        latency_ms?: number;
        error?: string;
      };
      if (res?.ok) {
        toast.success(
          `${res.identity || r.name} · RouterOS ${res.ros_version ?? '?'} · ${res.latency_ms ?? '?'} ms`,
        );
      } else {
        toast.error(res?.error || 'Gagal terhubung');
      }
      await refreshSilent();
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      testing = null;
    }
  }

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }
    void load().then(() => applyDeepLink());

    /* Halaman lama memanggil API tiap 5 detik padahal poller backend hanya
       berjalan tiap 5 menit — 60 permintaan per siklus data yang sama.
       Di sini: muat ulang seirama poller, dan perbarui `now` tiap 30 detik
       supaya label umur ikut bergerak tanpa permintaan jaringan. */
    tick = setInterval(() => {
      now = Date.now();
      if (Date.now() % POLL_INTERVAL_MS < 30_000) void refreshSilent();
    }, 30_000);
  });

  onDestroy(() => {
    if (tick) clearInterval(tick);
  });
</script>

<AppShell title={ $t('admin.network.routers.title') }>
  <PageHeader
    title={ $t('admin.network.routers.title') }
    eyebrow={ $t('admin.eyebrows.network') }
    desc={ $t('admin.network.routers.v2list.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={load}>{ $t('network.olt.refresh') }</Button>
      {#if $can('manage', 'router_inventory')}
        <Button icon="plus" onclick={openCreate}>{ $t('admin.network.routers.v2list.add') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile
        label={ $t('admin.network.routers.v2list.monitored') }
        value={String(stats.monitored)}
        hint={$t('admin.network.routers.v2list.hint_monitored', { values: { total: stats.total } })}
      />
      <StatTile
        label={ $t('admin.network.routers.v2d.online') }
        value={String(stats.online)}
        hint={stats.monitored ? $t('admin.network.routers.v2list.h_on_of', { values: { a: stats.online, b: stats.monitored } }) : $t('admin.network.routers.v2list.h_none_mon')}
        tone={stats.online === stats.monitored && stats.monitored > 0 ? 'positive' : 'neutral'}
      />
      <StatTile
        label={ $t('admin.network.routers.v2d.off') }
        value={String(stats.offline)}
        hint={stats.offline ? $t('admin.network.routers.v2list.h_no_answer') : $t('admin.network.routers.v2list.h_all_answer')}
        tone={stats.offline ? 'negative' : 'neutral'}
      />
      <StatTile
        label={ $t('admin.network.routers.v2list.unmonitored') }
        value={String(stats.disabled + stats.stale)}
        hint={$t('admin.network.routers.v2list.hint_disabled', { values: { d: stats.disabled, s: stats.stale } })}
        tone={stats.disabled + stats.stale ? 'warning' : 'neutral'}
      />
    </div>
  </Card>

  {#if perluPerhatian.length}
    <div class="mt-4">
      <AttentionPanel items={perluPerhatian} title={ $t('admin.network.routers.v2list.untrusted') } />
    </div>
  {/if}

  <div class="mt-4">
    <Card>
      <div class="mb-3 flex items-center gap-2">
        <div class="relative max-w-sm flex-1">
          <Icon
            name="search"
            size={15}
            class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400"
          />
          <input
            bind:value={search}
            placeholder={ $t('admin.network.routers.v2list.search_ph') }
            aria-label={ $t('admin.network.routers.v2list.search_aria') }
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>
      </div>

      <DataTable pageSize={25}
        {columns}
        rows={filtered}
        {loading}
        emptyTitle="Belum ada router"
        emptyHint="Tambahkan perangkat MikroTik untuk mulai memantau."
        footNote={$t('admin.network.routers.v2list.foot_poller', { values: { shown: filtered.length, total: rows.length, m: Math.round(POLL_INTERVAL_MS / 60000) } })}
      >
        {#snippet cell(r, c)}
          {@const s = routerStatus(r, now, tr)}

          {#if c.key === 'name'}
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-medium text-ink-900">{r.name}</span>
                {#if r.identity && r.identity !== r.name}
                  <span class="text-sm text-ink-400">{r.identity}</span>
                {/if}
              </div>
              <div class="num text-sm text-ink-500">{r.username}@{r.host}:{r.port}</div>
            </div>
          {:else if c.key === 'status'}
            <div class="min-w-0">
              <Badge tone={statusTone(s.state)} label={s.label} />
              <!-- Alasan selalu ikut. Badge sendiri tidak cukup: "Dinonaktifkan"
                   tanpa "data terakhir 28 hari lalu" masih menyisakan tebakan. -->
              <div class="mt-1 text-sm text-ink-500">{s.reason}</div>
            </div>
          {:else if c.key === 'latency'}
            {#if s.metricsTrustworthy && r.latency_ms != null}
              <span class="text-ink-700">{r.latency_ms} ms</span>
            {:else if r.latency_ms != null}
              <!-- Angka basi tidak ditampilkan sebagai pengukuran. -->
              <span class="text-ink-400" title={ $t('admin.network.routers.v2list.stale_hint') }>
                {r.latency_ms} ms lama
              </span>
            {:else}
              <span class="text-ink-400">—</span>
            {/if}
          {:else if c.key === 'seen'}
            <span class="text-sm {s.metricsTrustworthy ? 'text-ink-500' : 'text-amber-700'}">
              {s.ageMs == null ? $t('network.olt.v2.never') : $t('common.time.ago', { values: { t: humanAge(s.ageMs, tr) } })}
            </span>
          {:else if c.key === 'actions'}
            <RowActions
              primary={{ label: $t('common.open'), icon: 'chevronRight', href: `/v2/admin/network/routers/${r.id}` }}
              rest={[
                {
                  label: testing === r.id ? $t('admin.network.routers.v2list.testing') : $t('admin.network.routers.v2list.test_conn'),
                  icon: 'zap',
                  onclick: () => void uji(r),
                },
                ...($can('manage', 'router_inventory')
                  ? [
                      {
                        label: $t('common.edit'),
                        icon: 'cog' as const,
                        onclick: () => openEdit(r),
                      },
                    ]
                  : []),
              ]}
            />
          {/if}
        {/snippet}
      </DataTable>
    </Card>
  </div>
</AppShell>

<RouterFormModal
  bind:show={formShow}
  editing={editing}
  bind:formName
  bind:formHost
  bind:formPort
  bind:formUsername
  bind:formPassword
  bind:formLatitude
  bind:formLongitude
  bind:formEnabled
  bind:formMaintenanceEnabled
  bind:formMaintenanceUntilLocal
  bind:formMaintenanceReason
  onSubmit={() => void saveForm()}
/>
