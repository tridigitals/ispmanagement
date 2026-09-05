<script lang="ts">
  /*
    Tiket dukungan v2.

    Versi lama: `(app)/admin/support/+page.svelte` — 803 baris, 182 script +
    416 CSS scoped, dengan tiga angka yang tidak bisa dipercaya:

      baris  28: stats = { all, open, pending, closed }  -> 'resolved' tidak ada
      baris 262: <div class="stat-value">—</div>          -> "Belum ditugaskan"
                                                             em-dash HARDCODE
      baris  25: statusFilter tanpa 'resolved'            -> tidak bisa difilter

    Terukur di produksi (probe-support-stale.cjs): Total 20 tapi
    Open 18 + Pending 0 + Closed 1 = 19. Satu tiket resolved hilang dari
    ringkasan, dan 9 tiket tanpa penerima tugas ditampilkan sebagai "—".

    Aturan angka pindah ke `$lib/utils/supportStats` (21 tes unit).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { SupportTicketListItem, SupportTicketStats } from '$lib/api/types';
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
    bucketByAge,
    buildStatCards,
    isStale,
    shouldShowPending,
    unaccounted,
    waitingLabel,
    type TicketStatus,
  } from '$lib/utils/supportStats';

  type Filter = TicketStatus | 'unassigned' | null;

  let tickets = $state<SupportTicketListItem[]>([]);
  let stats = $state<SupportTicketStats>({
    all: 0,
    open: 0,
    pending: 0,
    closed: 0,
    resolved: 0,
    unassigned: 0,
  });
  let loading = $state(true);
  let loadingMore = $state(false);
  let total = $state(0);
  let pageNum = $state(1);
  let search = $state('');
  let filter = $state<Filter>(null);
  let category = $state<string>('all');
  let claiming = $state<string | null>(null);
  let now = $state(Date.now());
  let ready = $state(false);
  let seq = 0;

  const PER_PAGE = 25;

  const cards = $derived(buildStatCards(stats));
  const showPending = $derived(shouldShowPending(stats));
  const hilang = $derived(unaccounted(stats));
  const hasMore = $derived(tickets.length < total);
  const ageBuckets = $derived(bucketByAge(tickets, now));

  /* Tiket aktif yang menunggu lebih dari 30 hari. Halaman lama menampilkan
     kolom "Updated" berisi tanggal, jadi tiket 195 hari terlihat sama biasa
     dengan tiket kemarin. */
  const terlantar = $derived.by(() =>
    tickets
      .filter((t) => isStale(t, now))
      .slice(0, 5)
      .map((t) => ({
        icon: 'clock' as const,
        title: t.subject,
        detail: `Menunggu ${waitingLabel(t.created_at, now)}${t.assigned_to ? '' : ' · belum ditugaskan'}`,
        action: 'Buka',
        href: `/v2/admin/support/${t.id}`,
        severity: 'high' as const,
      })),
  );

  const columns: Column[] = [
    { key: 'subject', label: 'Tiket' },
    { key: 'status', label: 'Status' },
    { key: 'assigned', label: 'Ditugaskan' },
    { key: 'waiting', label: 'Menunggu', hideSm: true },
    { key: 'messages', label: 'Pesan', align: 'right', num: true, hideSm: true },
    { key: 'actions', label: '', align: 'right', width: '150px' },
  ];

  const categories = [
    { value: 'all', label: 'Semua kategori' },
    { value: 'general', label: 'Umum' },
    { value: 'billing', label: 'Tagihan' },
    { value: 'technical', label: 'Teknis' },
    { value: 'installation', label: 'Instalasi' },
  ];

  async function loadStats() {
    try {
      stats = await api.support.stats();
    } catch {
      /* ringkasan tidak memblokir tabel */
    }
  }

  async function load(reset: boolean) {
    const mine = ++seq;
    loading = true;
    if (reset) {
      pageNum = 1;
      tickets = [];
      total = 0;
    }
    try {
      const res = await api.support.list({
        // 'unassigned' bukan status: ia filter penugasan, bukan status tiket.
        status: filter && filter !== 'unassigned' ? filter : undefined,
        assigned: filter === 'unassigned' ? 'unassigned' : undefined,
        category: category === 'all' ? undefined : category,
        search: search.trim() || undefined,
        page: pageNum,
        perPage: PER_PAGE,
      });
      if (mine !== seq) return;
      total = res.total || 0;
      tickets = reset ? res.data : [...tickets, ...res.data];
      now = Date.now();
    } catch (e: unknown) {
      if (mine === seq) toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      if (mine === seq) loading = false;
    }
  }

  async function loadMore() {
    if (loadingMore || loading || !hasMore) return;
    loadingMore = true;
    pageNum += 1;
    try {
      const res = await api.support.list({
        status: filter && filter !== 'unassigned' ? filter : undefined,
        assigned: filter === 'unassigned' ? 'unassigned' : undefined,
        category: category === 'all' ? undefined : category,
        search: search.trim() || undefined,
        page: pageNum,
        perPage: PER_PAGE,
      });
      total = res.total || total;
      tickets = [...tickets, ...res.data];
    } catch (e: unknown) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      loadingMore = false;
    }
  }

  function pilih(f: Filter) {
    filter = filter === f ? null : f;
    void load(true);
  }

  async function klaim(t: SupportTicketListItem) {
    if (claiming) return;
    claiming = t.id;
    try {
      await api.support.claim(t.id);
      toast.success('Tiket diambil');
      await Promise.all([load(true), loadStats()]);
    } catch (e: unknown) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      claiming = null;
    }
  }

  onMount(() => {
    if (!$can('read', 'support') && !$can('read_all', 'support')) {
      goto('/unauthorized');
      return;
    }
    void loadStats();
    void load(true);
    ready = true;
  });

  $effect(() => {
    if (!ready) return;
    const q = search;
    void q;
    const timer = setTimeout(() => void load(true), 250);
    return () => clearTimeout(timer);
  });
</script>

<AppShell title="Tiket dukungan">
  <PageHeader
    title="Tiket dukungan"
    eyebrow="Layanan"
    desc="Keluhan dan permintaan pelanggan. Ringkasan menghitung setiap status, termasuk yang sudah diselesaikan."
  >
    {#snippet actions()}
      <Button
        variant="ghost"
        icon="refresh"
        onclick={() => {
          void loadStats();
          void load(true);
        }}>Muat ulang</Button
      >
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-5">
      {#each cards as c (c.key)}
        {#if c.key !== 'pending' || showPending}
          <button
            type="button"
            onclick={() => pilih(c.filter)}
            aria-pressed={filter === c.filter}
            class="focus-ring rounded-lg p-2 text-left transition-colors {filter === c.filter
              ? 'bg-ink-100 ring-1 ring-inset ring-ink-300'
              : 'hover:bg-ink-50'}"
          >
            <StatTile label={c.label} value={String(c.value)} hint={c.hint} tone={c.tone} />
          </button>
        {/if}
      {/each}
    </div>

    {#if hilang > 0}
      <!-- Jaring pengaman: kalau backend menulis status yang belum masuk
           ringkasan, selisihnya tampil di sini alih-alih hilang diam-diam. -->
      <p class="mt-3 border-t border-ink-100 pt-3 text-sm text-amber-800">
        {hilang} tiket punya status di luar ember di atas. Ringkasan perlu diperbarui.
      </p>
    {/if}
  </Card>

  {#if terlantar.length}
    <div class="mt-4">
      <AttentionPanel items={terlantar} title="Tiket menunggu lebih dari 30 hari" />
    </div>
  {/if}

  <div class="mt-4">
    <Card>
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <div class="relative min-w-[220px] flex-1">
          <Icon
            name="search"
            size={15}
            class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400"
          />
          <input
            bind:value={search}
            placeholder="Cari subjek atau nama pelapor"
            aria-label="Cari tiket"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>

        <select
          bind:value={category}
          onchange={() => load(true)}
          aria-label="Filter kategori"
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
        >
          {#each categories as c (c.value)}
            <option value={c.value}>{c.label}</option>
          {/each}
        </select>

        {#if filter}
          <Button variant="ghost" icon="close" onclick={() => pilih(filter)}>Hapus filter</Button>
        {/if}
      </div>

      {#if ageBuckets.some((b) => b.count > 0)}
        <!-- Distribusi umur tiket aktif: informasi yang tidak ada di halaman lama. -->
        <div class="mb-3 flex flex-wrap gap-4 border-b border-ink-100 pb-3">
          {#each ageBuckets as b (b.label)}
            <div class="text-sm">
              <span class="num font-semibold text-ink-900">{b.count}</span>
              <span class="text-ink-500"> {b.label}</span>
            </div>
          {/each}
        </div>
      {/if}

      <DataTable
        {columns}
        rows={tickets}
        {loading}
        emptyTitle="Tidak ada tiket"
        emptyHint={filter || search ? 'Coba hapus filter atau ubah kata kunci.' : 'Belum ada keluhan masuk.'}
        footNote={`${tickets.length} dari ${total} tiket`}
      >
        {#snippet cell(t, c)}
          {#if c.key === 'subject'}
            <div class="min-w-0">
              <a
                href={`/v2/admin/support/${t.id}`}
                class="focus-ring block truncate font-medium text-ink-900 hover:underline"
              >
                {t.subject}
              </a>
              <div class="truncate text-sm text-ink-500">
                {t.created_by_name || 'Tanpa nama'}
                {#if t.category}· {t.category}{/if}
              </div>
            </div>
          {:else if c.key === 'status'}
            <div class="flex flex-wrap items-center gap-1.5">
              <Badge status={t.status} />
              {#if t.priority && t.priority !== 'normal'}
                <Badge status={t.priority} />
              {/if}
            </div>
          {:else if c.key === 'assigned'}
            {#if t.assigned_to}
              <span class="text-ink-700">{t.assigned_to_name || 'Sudah ditugaskan'}</span>
            {:else}
              <!-- Ditandai eksplisit: ini pekerjaan tanpa pemilik. -->
              <span class="text-amber-800">Belum ditugaskan</span>
            {/if}
          {:else if c.key === 'waiting'}
            <span class="text-sm {isStale(t, now) ? 'font-medium text-red-700' : 'text-ink-500'}">
              {waitingLabel(t.created_at, now)}
            </span>
          {:else if c.key === 'messages'}
            <span class="text-ink-700">{t.message_count ?? 0}</span>
          {:else if c.key === 'actions'}
            <RowActions
              primary={{ label: 'Buka', icon: 'chevronRight', href: `/v2/admin/support/${t.id}` }}
              rest={!t.assigned_to && t.status !== 'closed' && t.status !== 'resolved'
                ? [
                    {
                      label: claiming === t.id ? 'Mengambil…' : 'Ambil tiket',
                      icon: 'users' as const,
                      onclick: () => void klaim(t),
                    },
                  ]
                : []}
            />
          {/if}
        {/snippet}
      </DataTable>

      {#if hasMore}
        <div class="mt-3 flex justify-center">
          <Button variant="secondary" loading={loadingMore} onclick={loadMore}>
            Muat {Math.min(PER_PAGE, total - tickets.length)} tiket lagi
          </Button>
        </div>
      {/if}
    </Card>
  </div>
</AppShell>
