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
  import { extractApiErrorMessage } from '$lib/api/core';
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
  import { t } from 'svelte-i18n';
  import { get as getStore } from 'svelte/store';

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
  let total = $state(0);
  let pageNum = $state(1);
  let search = $state('');
  let filter = $state<Filter>(null);
  let category = $state<string>('all');
  let claiming = $state<string | null>(null);
  let now = $state(Date.now());
  let ready = $state(false);
  let seq = 0;

  let PER_PAGE = $state(25);

  const cards = $derived(buildStatCards(stats, (k, v) => $t(k, v ? { values: v } : undefined)));
  const showPending = $derived(shouldShowPending(stats));
  const hilang = $derived(unaccounted(stats));
  const ageBuckets = $derived(bucketByAge(tickets, now, (k, v) => getStore(t)(k, v ? { values: v } : undefined)));

  /* Tiket aktif yang menunggu lebih dari 30 hari. Halaman lama menampilkan
     kolom "Updated" berisi tanggal, jadi tiket 195 hari terlihat sama biasa
     dengan tiket kemarin. */
  const terlantar = $derived.by(() =>
    tickets
      .filter((tk) => isStale(tk, now))
      .slice(0, 5)
      .map((tk) => ({
        icon: 'clock' as const,
        title: tk.subject,
        detail: $t('support.admin_v2.waiting_prefix') + ' ' + waitingLabel(tk.created_at, now, (k, v) => getStore(t)(k, v ? { values: v } : undefined)) + (tk.assigned_to ? '' : ' · ' + $t('support.admin_v2.unassigned')),
        action: $t('common.open'),
        href: `/v2/admin/support/${tk.id}`,
        severity: 'high' as const,
      })),
  );

  const columns = $derived<Column[]>([
    { key: 'subject', label: $t('support.admin_v2.col_ticket') },
    { key: 'status', label: $t('common.status') },
    { key: 'assigned', label: $t('support.admin_v2.col_assigned') },
    { key: 'waiting', label: $t('support.admin_v2.col_waiting'), hideSm: true },
    { key: 'messages', label: $t('support.admin_v2.col_messages'), align: 'right', num: true, hideSm: true },
    { key: 'actions', label: '', align: 'right', width: '150px' },
  ]);

  const categories = $derived([
    { value: 'all', label: $t('support.admin_v2.cat_all') },
    { value: 'general', label: $t('support.categories.general') },
    { value: 'billing', label: $t('support.categories.billing') },
    { value: 'technical', label: $t('support.categories.technical') },
    { value: 'installation', label: $t('support.categories.installation') },
  ]);

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
    if (reset) pageNum = 1;
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
      tickets = res.data;
      now = Date.now();
    } catch (e: unknown) {
      if (mine === seq) toast.error(extractApiErrorMessage(e));
    } finally {
      if (mine === seq) loading = false;
    }
  }

  function pilih(f: Filter) {
    filter = filter === f ? null : f;
    void load(true);
  }

  async function klaim(tk: SupportTicketListItem) {
    if (claiming) return;
    claiming = tk.id;
    try {
      await api.support.claim(tk.id);
      toast.success(getStore(t)('support.admin_v2.claimed'))
      await Promise.all([load(true), loadStats()]);
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e));
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

<AppShell title={ $t('support.admin_v2.title') }>
  <PageHeader
    title={ $t('support.admin_v2.title') }
    eyebrow={ $t('admin.eyebrows.services') }
    desc={ $t('support.admin_v2.desc') }
  >
    {#snippet actions()}
      <Button
        variant="ghost"
        icon="refresh"
        onclick={() => {
          void loadStats();
          void load(true);
        }}> { $t('common.refresh') }</Button
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
        {$t('support.admin_v2.bucket_note', { values: { n: hilang } })}
      </p>
    {/if}
  </Card>

  {#if terlantar.length}
    <div class="mt-4">
      <AttentionPanel items={terlantar} title={ $t('support.admin_v2.abandon_title') } />
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
            placeholder={ $t('support.admin_v2.search_ph') }
            aria-label={ $t('support.admin_v2.search_aria') }
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>

        <select
          bind:value={category}
          onchange={() => load(true)}
          aria-label={ $t('support.admin_v2.cat_aria') }
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
        >
          {#each categories as c (c.value)}
            <option value={c.value}>{c.label}</option>
          {/each}
        </select>

        {#if filter}
          <Button variant="ghost" icon="close" onclick={() => pilih(filter)}>{ $t('support.admin_v2.clear_filter') }</Button>
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
        pageSize={PER_PAGE}
        page={pageNum}
        {total}
        onpage={(p) => {
          pageNum = p;
          void load(false);
        }}
        onpagesize={(n) => {
          PER_PAGE = n;
          pageNum = 1;
          void load(false);
        }}
        emptyTitle={ $t('support.admin_v2.empty_title') }
        emptyHint={filter || search ? $t('support.admin_v2.empty_hint') : $t('support.admin_v2.empty_hint_none')}
      >
        {#snippet cell(tk, c)}
          {#if c.key === 'subject'}
            <div class="min-w-0">
              <a
                href={`/v2/admin/support/${tk.id}`}
                class="focus-ring block truncate font-medium text-ink-900 hover:underline"
              >
                {tk.subject}
              </a>
              <div class="truncate text-sm text-ink-500">
                {tk.created_by_name || $t('support.admin_v2.anon')}
                {#if tk.category}· {tk.category}{/if}
              </div>
            </div>
          {:else if c.key === 'status'}
            <div class="flex flex-wrap items-center gap-1.5">
              <Badge status={tk.status} />
              {#if tk.priority && tk.priority !== 'normal'}
                <Badge status={tk.priority} />
              {/if}
            </div>
          {:else if c.key === 'assigned'}
            {#if tk.assigned_to}
              <span class="text-ink-700">{tk.assigned_to_name || $t('support.admin_v2.assigned_ok')}</span>
            {:else}
              <!-- Ditandai eksplisit: ini pekerjaan tanpa pemilik. -->
              <span class="text-amber-800">{ $t('support.admin_v2.unassigned') }</span>
            {/if}
          {:else if c.key === 'waiting'}
            <span class="text-sm {isStale(tk, now) ? 'font-medium text-red-700' : 'text-ink-500'}">
              {waitingLabel(tk.created_at, now, (k, v) => getStore(t)(k, v ? { values: v } : undefined))}
            </span>
          {:else if c.key === 'messages'}
            <span class="text-ink-700">{tk.message_count ?? 0}</span>
          {:else if c.key === 'actions'}
            <RowActions
              primary={{ label: $t('common.open'), icon: 'chevronRight', href: `/v2/admin/support/${tk.id}` }}
              rest={!tk.assigned_to && tk.status !== 'closed' && tk.status !== 'resolved'
                ? [
                    {
                      label: claiming === tk.id ? $t('support.admin_v2.claiming') : $t('support.admin_v2.claim_btn'),
                      icon: 'users' as const,
                      onclick: () => void klaim(tk),
                    },
                  ]
                : []}
            />
          {/if}
        {/snippet}
      </DataTable>
    </Card>
  </div>
</AppShell>
