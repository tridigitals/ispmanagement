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
    hijau "Online" dan "65 ms", ringkasan "Online 3 dari 3".

    Semua logika status pindah ke `$lib/utils/routerStatus` (13 tes unit) supaya
    aturannya bisa diuji tanpa merender halaman.
  */
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
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

  type Row = RouterLike & {
    id: string;
    name: string;
    host: string;
    port: number;
    username: string;
    identity?: string | null;
    ros_version?: string | null;
  };

  let rows = $state<Row[]>([]);
  let loading = $state(true);
  let search = $state('');
  let now = $state(Date.now());
  let testing = $state<string | null>(null);
  let tick: ReturnType<typeof setInterval> | null = null;

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
      .map((r) => ({ row: r, status: routerStatus(r, now) }))
      .filter((x) => x.status.state === 'stale' || x.status.state === 'disabled')
      .map((x) => ({
        icon: x.status.state === 'disabled' ? ('lock' as const) : ('clock' as const),
        title: `${x.row.name} — ${x.status.label.toLowerCase()}`,
        detail: x.status.reason,
        action: 'Buka',
        href: `/admin/network/routers/${x.row.id}`,
        severity: (x.status.state === 'stale' ? 'high' : 'medium') as 'high' | 'medium',
      })),
  );

  const columns: Column[] = [
    { key: 'name', label: 'Router' },
    { key: 'status', label: 'Status' },
    { key: 'latency', label: 'Latensi', align: 'right', num: true, hideSm: true },
    { key: 'seen', label: 'Data terakhir', hideSm: true },
    { key: 'actions', label: '', align: 'right', width: '150px' },
  ];

  async function load() {
    loading = true;
    try {
      rows = (await api.mikrotik.routers.list()) as Row[];
      now = Date.now();
    } catch (e: unknown) {
      toast.error(e instanceof Error ? e.message : String(e));
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
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      testing = null;
    }
  }

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }
    void load();

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

<AppShell title="Router">
  <PageHeader
    title="Router"
    eyebrow="Jaringan"
    desc="Perangkat MikroTik yang dipantau. Status dihitung dari status aktif, umur data, dan jendela pemeliharaan."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={load}>Muat ulang</Button>
      {#if $can('manage', 'router_inventory')}
        <Button icon="plus" href="/admin/network/routers?new=1">Tambah router</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile
        label="Dipantau"
        value={String(stats.monitored)}
        hint={`dari ${stats.total} router terdaftar`}
      />
      <StatTile
        label="Online"
        value={String(stats.online)}
        hint={stats.monitored ? `${stats.online} dari ${stats.monitored} yang dipantau` : 'tidak ada yang dipantau'}
        tone={stats.online === stats.monitored && stats.monitored > 0 ? 'positive' : 'neutral'}
      />
      <StatTile
        label="Offline"
        value={String(stats.offline)}
        hint={stats.offline ? 'tidak menjawab poll terakhir' : 'semua menjawab'}
        tone={stats.offline ? 'negative' : 'neutral'}
      />
      <StatTile
        label="Tidak dipantau"
        value={String(stats.disabled + stats.stale)}
        hint={`${stats.disabled} dinonaktifkan · ${stats.stale} data usang`}
        tone={stats.disabled + stats.stale ? 'warning' : 'neutral'}
      />
    </div>
  </Card>

  {#if perluPerhatian.length}
    <div class="mt-4">
      <AttentionPanel items={perluPerhatian} title="Status tidak bisa dipercaya" />
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
            placeholder="Cari nama, host, atau identity"
            aria-label="Cari router"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>
      </div>

      <DataTable
        {columns}
        rows={filtered}
        {loading}
        emptyTitle="Belum ada router"
        emptyHint="Tambahkan perangkat MikroTik untuk mulai memantau."
        footNote={`${filtered.length} dari ${rows.length} router · poller backend berjalan tiap ${Math.round(POLL_INTERVAL_MS / 60000)} menit`}
      >
        {#snippet cell(r, c)}
          {@const s = routerStatus(r, now)}

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
              <span class="text-ink-400" title="Nilai terakhir sebelum berhenti dipantau">
                {r.latency_ms} ms lama
              </span>
            {:else}
              <span class="text-ink-400">—</span>
            {/if}
          {:else if c.key === 'seen'}
            <span class="text-sm {s.metricsTrustworthy ? 'text-ink-500' : 'text-amber-700'}">
              {s.ageMs == null ? 'belum pernah' : `${humanAge(s.ageMs)} lalu`}
            </span>
          {:else if c.key === 'actions'}
            <RowActions
              primary={{ label: 'Buka', icon: 'chevronRight', href: `/admin/network/routers/${r.id}` }}
              rest={[
                {
                  label: testing === r.id ? 'Menguji…' : 'Uji koneksi',
                  icon: 'zap',
                  onclick: () => void uji(r),
                },
                ...($can('manage', 'router_inventory')
                  ? [
                      {
                        label: 'Ubah',
                        icon: 'cog' as const,
                        onclick: () => goto(`/admin/network/routers?edit=${r.id}`),
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
