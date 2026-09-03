<script lang="ts">
  /*
    PPPoE v2 — halaman jaringan. Versi lama: 1.434 baris + 6.468 karakter CSS.

    Temuan DB yang membentuk desain halaman ini (1.037 akun terukur):

    1. `is_provisioned` BUKAN indikator kesehatan. Nilainya berkorelasi 100%
       dengan `account_source`: 545 akun managed_radius semuanya true, 492 akun
       router semuanya false. Jadi filter "belum di-apply 492" pada versi lama
       menakut-nakuti tanpa sebab — itu cuma cara akun dibuat, bukan masalah.

    2. YANG BENAR-BENAR MASALAH: 7 akun bersumber router yang TIDAK ADA di
       router (`router_present=false`, tidak disabled). Ini drift nyata antara
       aplikasi dan perangkat, dan versi lama tidak pernah menyorotinya.

    3. 542 akun managed_radius berstatus disabled — jumlahnya sama dengan 542
       langganan suspended. Itu isolir massal hasil impor MixRadius, bukan
       kerusakan.

    Karena itu tile dan chip di sini memakai tiga status operasional
    (melayani / terisolir / hilang di router), bukan `is_provisioned`.
  */
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import TableSkeleton from '$lib/components/ds/TableSkeleton.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import AttentionPanel from '$lib/components/ds/AttentionPanel.svelte';
  import { fetchAllPages, fetchAllRows } from '$lib/utils/fetchAllPages';
  import type { PppoeAccountPublic, CustomerListItem } from '$lib/api/types';

  /* api.mikrotik.routers.list() masih bertipe Promise<any[]> di lib/api/mikrotik.ts;
     dua field yang dipakai halaman ini dinyatakan eksplisit di sini. */
  type RouterRef = { id: string; name: string };

  type Health = 'serving' | 'isolated' | 'missing' | 'draft';
  type ChipKey = 'all' | Health;

  let all = $state<PppoeAccountPublic[]>([]);
  let complete = $state(true);
  let customerName = $state<Map<string, string>>(new Map());
  let routerName = $state<Map<string, string>>(new Map());
  let loading = $state(true);
  let err = $state('');

  let q = $state('');
  let chip = $state<ChipKey>('all');
  let routerFilter = $state('');
  let page = $state(1);
  const perPage = 25;

  const canManage = $derived($can('manage', 'pppoe'));

  /**
   * Status operasional satu akun. Urutan pemeriksaan penting: "hilang di
   * router" diperiksa sebelum disabled, karena akun yang tidak ada di
   * perangkat adalah masalah konfigurasi terlepas dari flag disabled-nya.
   */
  function healthOf(a: PppoeAccountPublic): Health {
    if (a.account_source === 'router' && !a.router_present) return 'missing';
    if (a.disabled) return 'isolated';
    if (a.account_source === 'managed_radius' && !a.is_provisioned) return 'draft';
    return 'serving';
  }

  const healthMeta: Record<Health, { label: string; tone: 'positive' | 'negative' | 'warning' | 'neutral' }> = {
    serving: { label: 'Melayani', tone: 'positive' },
    isolated: { label: 'Terisolir', tone: 'neutral' },
    missing: { label: 'Hilang di router', tone: 'negative' },
    draft: { label: 'Belum aktif', tone: 'warning' },
  };

  const counts = $derived.by(() => {
    const c = { all: all.length, serving: 0, isolated: 0, missing: 0, draft: 0 };
    for (const a of all) c[healthOf(a)]++;
    return c;
  });

  const routers = $derived(
    [...routerName.entries()]
      .map(([id, name]) => ({ id, name, n: all.filter((a) => a.router_id === id).length }))
      .filter((r) => r.n > 0)
      .sort((a, b) => b.n - a.n),
  );

  const filtered = $derived.by(() => {
    const needle = q.trim().toLowerCase();
    return all.filter((a) => {
      if (chip !== 'all' && healthOf(a) !== chip) return false;
      if (routerFilter && a.router_id !== routerFilter) return false;
      if (!needle) return true;
      const name = customerName.get(a.customer_id) ?? '';
      return (
        a.username.toLowerCase().includes(needle) ||
        name.toLowerCase().includes(needle) ||
        (a.comment ?? '').toLowerCase().includes(needle) ||
        (a.remote_address ?? '').toLowerCase().includes(needle)
      );
    });
  });

  const lastPage = $derived(Math.max(1, Math.ceil(filtered.length / perPage)));
  const rows = $derived(filtered.slice((page - 1) * perPage, page * perPage));
  const from = $derived(filtered.length === 0 ? 0 : (page - 1) * perPage + 1);
  const to = $derived(Math.min(page * perPage, filtered.length));

  const chips = $derived([
    { key: 'all' as ChipKey, label: 'Semua', count: counts.all },
    { key: 'serving' as ChipKey, label: 'Melayani', count: counts.serving },
    { key: 'isolated' as ChipKey, label: 'Terisolir', count: counts.isolated },
    { key: 'missing' as ChipKey, label: 'Hilang di router', count: counts.missing },
    { key: 'draft' as ChipKey, label: 'Belum aktif', count: counts.draft },
  ]);

  function applyChip(key: ChipKey) {
    chip = key;
    page = 1;
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onSearch(value: string) {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      q = value;
      page = 1;
    }, 250);
  }

  onMount(async () => {
    try {
      /* Semua akun ditarik sekali supaya hitungan chip dan tile benar untuk
         seluruh tenant, bukan hanya 25 baris pertama seperti versi lama.
         Batas backend 1.000/permintaan (src-tauri/src/services/pagination.rs),
         jadi 1.010 akun selesai dalam 2 permintaan — sebelum batasnya dinaikkan
         ini butuh 11. */
      const [accounts, customers, routerList] = await Promise.all([
        fetchAllPages<PppoeAccountPublic>((p, per_page) =>
          api.pppoe.accounts.list({ page: p, per_page }),
        ),
        fetchAllRows<CustomerListItem>((p, per_page) =>
          api.customers.list({ page: p, perPage: per_page }),
        ).catch(() => [] as CustomerListItem[]),
        api.mikrotik.routers.list().catch(() => []) as Promise<RouterRef[]>,
      ]);

      all = accounts.rows;
      /* Jujur soal kelengkapan: kalau penarikan berhenti di batas halaman,
         tile dan chip harus mengaku "minimal N", bukan mengklaim total. */
      complete = accounts.complete;
      customerName = new Map(customers.map((c) => [c.id, c.name]));
      routerName = new Map(routerList.map((r) => [r.id, r.name]));
    } catch (e) {
      err = 'Gagal memuat akun PPPoE';
      console.warn('list pppoe gagal', e);
    } finally {
      loading = false;
    }
  });
</script>

<AppShell title="PPPoE">
  <PageHeader
    title="Akun PPPoE"
    eyebrow={loading
      ? 'Memuat seluruh akun…'
      : complete
        ? `${counts.all} akun`
        : `minimal ${counts.all} akun (data belum lengkap)`}
    desc="Status diturunkan dari keberadaan akun di router dan flag isolir, bukan dari cara akun dibuat."
  >
    {#snippet actions()}
      {#if canManage}
        <Button icon="refresh">Rekonsiliasi router</Button>
        <Button variant="primary" icon="plus">Tambah akun</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if err}
    <div
      role="alert"
      class="mb-4 rounded-lg border border-red-200 bg-red-50 px-3.5 py-2.5 text-base text-red-800"
    >
      {err}
    </div>
  {/if}

  <Card class="mb-4">
    <div class="grid grid-cols-2 gap-5 lg:grid-cols-4">
      <StatTile
        label="Melayani"
        value={String(counts.serving)}
        hint={loading ? 'menghitung…' : `dari ${counts.all} akun terdaftar`}
        tone="positive"
      />
      <StatTile
        label="Terisolir"
        value={String(counts.isolated)}
        hint={loading ? 'menghitung…' : 'akun dinonaktifkan di perangkat'}
      />
      <StatTile
        label="Hilang di router"
        value={String(counts.missing)}
        hint={loading ? 'menghitung…' : 'terdaftar di aplikasi, tidak ada di perangkat'}
        tone={counts.missing > 0 ? 'negative' : 'positive'}
      />
      <StatTile
        label="Router terpakai"
        value={String(routers.length)}
        hint={loading
          ? 'menghitung…'
          : routers[0]
            ? `terbanyak ${routers[0].name} (${routers[0].n})`
            : 'belum ada router'}
      />
    </div>
  </Card>

  {#if !loading && counts.missing > 0}
    <!-- Drift nyata: akun ada di aplikasi tapi tidak ada di perangkat. -->
    <div class="mb-4">
      <AttentionPanel
        title="Perlu rekonsiliasi"
        items={[
          {
            icon: 'router',
            title: `${counts.missing} akun tidak ditemukan di router`,
            detail:
              'Akun ini terdaftar di aplikasi tetapi tidak ada di perangkat, jadi pelanggannya tidak bisa terhubung.',
            action: 'Lihat daftar',
            severity: 'high',
          },
        ]}
      />
    </div>
  {/if}

  <div class="mb-3 flex flex-wrap items-center gap-2">
    {#each chips as c (c.key)}
      <button
        onclick={() => applyChip(c.key)}
        aria-pressed={chip === c.key}
        class="focus-ring flex h-8 items-center gap-1.5 rounded-lg px-3 text-base
          {chip === c.key
          ? 'bg-ink-900 font-medium text-white'
          : 'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50'}"
      >
        {c.label}
        <span class="num text-sm {chip === c.key ? 'text-ink-300' : 'text-ink-400'}">{c.count}</span>
      </button>
    {/each}

    <div class="ml-auto flex items-center gap-2">
      {#if routers.length > 1}
        <select
          bind:value={routerFilter}
          onchange={() => (page = 1)}
          aria-label="Filter router"
          class="h-8 rounded-lg bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200 focus:ring-brand-600 focus:outline-none"
        >
          <option value="">Semua router</option>
          {#each routers as r (r.id)}
            <option value={r.id}>{r.name} ({r.n})</option>
          {/each}
        </select>
      {/if}

      <div class="relative w-full sm:w-56">
        <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400">
          <Icon name="search" size={15} />
        </span>
        <input
          oninput={(e) => onSearch((e.currentTarget as HTMLInputElement).value)}
          type="search"
          placeholder="Cari username, pelanggan"
          aria-label="Cari akun PPPoE"
          class="h-8 w-full rounded-lg bg-white pr-3 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:ring-brand-600 focus:outline-none"
        />
      </div>
    </div>
  </div>

  <Card padded={false}>
    {#if loading}
      <div class="px-4 py-3"><TableSkeleton rows={10} cols={5} /></div>
    {:else if rows.length === 0}
      <div class="flex flex-col items-center gap-2 px-4 py-16 text-center">
        <Icon name="key" size={26} class="text-ink-300" />
        <div class="text-base font-medium text-ink-700">Tidak ada akun cocok</div>
        <div class="text-sm text-ink-500">
          {q ? `Tidak ada hasil untuk "${q}".` : 'Coba ubah filter di atas.'}
        </div>
      </div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full border-collapse text-base">
          <thead>
            <tr class="border-b border-ink-200 bg-ink-50">
              <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                >Username</th
              >
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase lg:table-cell"
                >Pelanggan</th
              >
              <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase">Status</th>
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase md:table-cell"
                >Router</th
              >
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase xl:table-cell"
                >IP / pool</th
              >
              <th class="px-4 py-2 text-right text-xs font-semibold text-ink-500 uppercase">Aksi</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as a (a.id)}
              {@const h = healthOf(a)}
              <tr class="border-b border-ink-100 last:border-0 hover:bg-ink-50">
                <td class="num px-4 py-2.5 font-medium text-ink-900">{a.username}</td>
                <td class="hidden max-w-xs truncate px-4 py-2.5 text-ink-700 lg:table-cell">
                  {customerName.get(a.customer_id) ?? '—'}
                </td>
                <td class="px-4 py-2.5">
                  <Badge tone={healthMeta[h].tone} label={healthMeta[h].label} />
                </td>
                <td class="hidden px-4 py-2.5 text-ink-700 md:table-cell">
                  {routerName.get(a.router_id) ?? '—'}
                </td>
                <td class="num hidden px-4 py-2.5 text-sm text-ink-500 xl:table-cell">
                  {a.remote_address || a.address_pool || '—'}
                </td>
                <td class="px-4 py-2.5">
                  <RowActions
                    primary={{ label: 'Ubah', icon: 'cog' }}
                    rest={canManage
                      ? [
                          { label: 'Terapkan ke router', icon: 'refresh' },
                          {
                            label: a.disabled ? 'Aktifkan' : 'Isolir',
                            icon: a.disabled ? 'check' : 'alert',
                          },
                          { label: 'Hapus akun', icon: 'close', danger: true },
                        ]
                      : []}
                  />
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div
        class="flex flex-wrap items-center justify-between gap-2 border-t border-ink-200 bg-ink-50 px-4 py-2"
      >
        <div class="num text-sm text-ink-500">{from}–{to} dari {filtered.length} akun</div>
        <div class="flex items-center gap-1.5">
          <Button
            size="sm"
            icon="chevronLeft"
            label="Halaman sebelumnya"
            disabled={page <= 1}
            onclick={() => (page -= 1)}
          />
          <span class="num text-sm text-ink-500">Hal {page} / {lastPage}</span>
          <Button
            size="sm"
            icon="chevronRight"
            label="Halaman berikutnya"
            disabled={page >= lastPage}
            onclick={() => (page += 1)}
          />
        </div>
      </div>
    {/if}
  </Card>
</AppShell>
