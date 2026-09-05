<script lang="ts">
  /*
    Email Outbox v2.

    Versi lama: `(app)/admin/email-outbox/+page.svelte` (1.096 baris).

    Temuan yang dikunci gelombang ini:

    1. RETRY MENGIRIM ULANG EMAIL YANG SUDAH SAMPAI.
       Tombol Retry aktif untuk status `queued`/`failed` di FE, tapi server
       hanya menolak `sending` — baris `sent` bisa di-requeue lewat API dan
       pelanggan menerima email yang sama dua kali. Kini SQL retry menegakkan
       status IN ('queued','failed') di DUA layer (http + commands), dan FE
       memakai `retryable` dari server.

    2. WILDCARD LIKE TIDAK DI-ESCAPE (bug yang sama dengan audit-logs &
       services). Mencari "%" dulu mencocokkan seluruh tabel. Pakai
       like_pattern() dari audit_service.

    3. SERVER SUDAH MENGIRUNG metadata retry (retryable, next_retry_at,
       delivery_status_summary) tapi layar lama tidak pernah menampilkannya.
       v2 memakai ketiganya: badge "bisa dicoba lagi", jadwal antrian ulang,
       dan ringkasan pengiriman berbahasa Indonesia.

    4. Retry baris sent/sending dulu membalas "not found" yang menyesatkan.
       Pesan kini eksplisit dan diterjemahkan friendlyOutboxError().

    Logika murni pindah ke $lib/utils/outboxInsights (17 tes unit).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { EmailOutboxItem, EmailOutboxStats, PaginatedResponse } from '$lib/api/client';
  import { can, user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { formatDateTime } from '$lib/utils/date';
  import Modal from '$lib/components/ui/Modal.svelte';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
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
  import type { RowAction } from '$lib/components/ds/RowActions.svelte';
  import {
    clampBulkIds,
    deliverySummary,
    friendlyOutboxError,
    isRetryable,
    outboxStatusLabel,
    outboxStatusTone,
  } from '$lib/utils/outboxInsights';

  type Scope = 'tenant' | 'global' | 'all';
  type StatusFilter = 'all' | 'queued' | 'sending' | 'sent' | 'failed';

  let loading = $state(true);
  let busyId = $state<string | null>(null);
  let bulkBusy = $state(false);

  let items = $state<EmailOutboxItem[]>([]);
  let stats = $state<EmailOutboxStats>({ all: 0, queued: 0, sending: 0, sent: 0, failed: 0 });

  let search = $state('');
  let statusFilter = $state<StatusFilter>('all');
  let scope = $state<Scope>('tenant');
  let selectedIds = $state<string[]>([]);

  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 25;
  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  let detailOpen = $state(false);
  let detailLoading = $state(false);
  let detailItem = $state<EmailOutboxItem | null>(null);
  let detailTab = $state<'text' | 'html'>('text');

  let confirmOpen = $state(false);
  let confirmMode = $state<'single' | 'bulk'>('single');
  let confirmTargetId = $state<string | null>(null);

  let ready = $state(false);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  const isSuper = $derived(Boolean($user?.is_super_admin));
  const canRetry = $derived($can('retry', 'email_outbox'));
  const canDelete = $derived($can('delete', 'email_outbox'));
  const selectedCount = $derived(selectedIds.length);

  const attention = $state<AttentionItem[]>([]);

  type StatTone = 'neutral' | 'positive' | 'negative';
  const statTiles: {
    st: StatusFilter;
    label: string;
    hint: string;
    baseTone: StatTone;
  }[] = [
    { st: 'all', label: 'Total', hint: 'semua status pada cakupan ini', baseTone: 'neutral' },
    { st: 'queued', label: 'Antri', hint: 'menunggu worker', baseTone: 'neutral' },
    { st: 'sending', label: 'Mengirim', hint: 'sedang diproses worker', baseTone: 'neutral' },
    { st: 'sent', label: 'Terkirim', hint: 'sukses ke penerima', baseTone: 'positive' },
    { st: 'failed', label: 'Gagal', hint: 'butuh diperiksa', baseTone: 'negative' },
  ];

  const columns = $derived<Column[]>([
    { key: 'sel', label: '', width: '40px' },
    { key: 'to', label: 'Penerima' },
    { key: 'subject', label: 'Subjek' },
    { key: 'status', label: 'Status', width: '150px' },
    { key: 'attempts', label: 'Percobaan', width: '96px' },
    { key: 'scheduled', label: 'Dijadwalkan', width: '150px', hideSm: true },
    { key: 'updated', label: 'Diperbarui', width: '150px', hideSm: true },
    { key: 'actions', label: '', align: 'right', width: '120px' },
  ]);

  onMount(() => {
    void (async () => {
      const q = new URLSearchParams(location.search).get('status');
      if (q === 'queued' || q === 'sending' || q === 'sent' || q === 'failed') {
        statusFilter = q;
      }
      if (!$can('read', 'email_outbox')) {
        goto('/unauthorized');
        return;
      }
      await Promise.all([refreshStats(), load()]);
      ready = true;
    })();
  });

  $effect(() => {
    if (!ready) return;
    const _q = search;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      pageNum = 1;
      void load();
    }, 300);
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
    };
  });

  async function refreshStats() {
    try {
      stats = await api.emailOutbox.stats(scope);
    } catch {
      // non-blocking; tabel tetap bisa dimuat
    }
  }

  async function load() {
    loading = true;
    try {
      const res: PaginatedResponse<EmailOutboxItem> = await api.emailOutbox.list({
        scope,
        status: statusFilter === 'all' ? undefined : statusFilter,
        search: search.trim() || undefined,
        page: pageNum,
        perPage,
      });
      total = res.total || 0;
      items = res.data || [];
      selectedIds = [];
      buildAttention();
    } catch (e) {
      toast.error(friendlyOutboxError(extractApiErrorMessage(e)));
    } finally {
      loading = false;
    }
  }

  function buildAttention() {
    attention.length = 0;
    if (stats.failed > 0) {
      attention.push({
        severity: 'high',
        icon: 'alert',
        title: `${stats.failed} email gagal terkirim`,
        detail: 'Periksa error terakhir, lalu coba ulang atau hapus dari antrian.',
        action: 'Lihat yang gagal',
        href: '/v2/admin/email-outbox?status=failed',
      });
    }
    if (stats.sending > 0) {
      attention.push({
        severity: 'low',
        icon: 'clock',
        title: `${stats.sending} email sedang dikirim`,
        detail: 'Tunggu worker selesai sebelum mencoba ulang — baris sending tidak bisa diubah.',
        action: '',
      });
    }
  }

  function setStatusFilter(v: StatusFilter) {
    if (statusFilter === v) return;
    statusFilter = v;
    pageNum = 1;
    void load();
  }

  function setScope(v: Scope) {
    if (scope === v) return;
    scope = v;
    pageNum = 1;
    void Promise.all([refreshStats(), load()]);
  }

  function isSelected(id: string) {
    return selectedIds.includes(id);
  }

  function toggleSelected(id: string) {
    selectedIds = isSelected(id) ? selectedIds.filter((x) => x !== id) : [...selectedIds, id];
  }

  function selectVisibleRetryable() {
    const ids = items.filter((i) => rowRetryable(i)).map((i) => i.id);
    selectedIds = Array.from(new Set([...selectedIds, ...ids]));
  }

  function rowRetryable(i: EmailOutboxItem): boolean {
    // Server adalah sumber kebenaran; fallback ke perhitungan FE yang identik.
    return typeof i.retryable === 'boolean'
      ? i.retryable
      : isRetryable(i.status, i.attempts, i.max_attempts);
  }

  async function retryOne(id: string) {
    if (busyId) return;
    busyId = id;
    try {
      await api.emailOutbox.retry(id);
      toast.success('Email dimasukin lagi ke antrian.');
      await Promise.all([refreshStats(), load()]);
    } catch (e) {
      toast.error(friendlyOutboxError(extractApiErrorMessage(e)));
    } finally {
      busyId = null;
    }
  }

  async function bulkRetry() {
    const ids = clampBulkIds(selectedIds);
    if (!ids.length || bulkBusy) return;
    bulkBusy = true;
    try {
      const res = await api.emailOutbox.retryBulk(ids);
      toast.success(`${res.count} email dimasukin lagi ke antrian.`);
      selectedIds = [];
      await Promise.all([refreshStats(), load()]);
    } catch (e) {
      toast.error(friendlyOutboxError(extractApiErrorMessage(e)));
    } finally {
      bulkBusy = false;
    }
  }

  function confirmDelete(id: string | null, mode: 'single' | 'bulk') {
    confirmTargetId = id;
    confirmMode = mode;
    confirmOpen = true;
  }

  async function handleConfirmDelete() {
    confirmOpen = false;
    try {
      if (confirmMode === 'bulk') {
        const ids = clampBulkIds(selectedIds);
        if (!ids.length) return;
        const res = await api.emailOutbox.deleteBulk(ids);
        toast.success(`${res.count} baris dihapus.`);
        selectedIds = [];
      } else if (confirmTargetId) {
        await api.emailOutbox.delete(confirmTargetId);
        toast.success('Baris dihapus.');
      }
      await Promise.all([refreshStats(), load()]);
    } catch (e) {
      toast.error(friendlyOutboxError(extractApiErrorMessage(e)));
    } finally {
      confirmTargetId = null;
    }
  }

  async function exportCsv() {
    try {
      const res = await api.emailOutbox.exportCsv({
        scope,
        status: statusFilter === 'all' ? undefined : statusFilter,
        search: search.trim() || undefined,
      });
      const csv = res?.csv || '';
      const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      const stamp = new Date().toISOString().slice(0, 10);
      a.href = url;
      a.download = `email-outbox_${scope}_${stamp}.csv`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      toast.success('CSV diunduh.');
    } catch (e) {
      toast.error(friendlyOutboxError(extractApiErrorMessage(e)));
    }
  }

  async function openDetails(id: string) {
    detailOpen = true;
    detailLoading = true;
    detailItem = null;
    detailTab = 'text';
    try {
      detailItem = await api.emailOutbox.get(id);
    } catch (e) {
      toast.error(friendlyOutboxError(extractApiErrorMessage(e)));
      detailOpen = false;
    } finally {
      detailLoading = false;
    }
  }

  function rowRest(i: EmailOutboxItem): RowAction[] {
    const acts: RowAction[] = [];
    if (canRetry) {
      acts.push({
        label: 'Coba ulang',
        icon: 'refresh',
        disabled: !rowRetryable(i) || busyId === i.id,
        onclick: () => void retryOne(i.id),
      });
    }
    if (canDelete) {
      acts.push({
        label: 'Hapus',
        icon: 'close',
        danger: true,
        disabled: busyId === i.id || i.status === 'sending',
        onclick: () => confirmDelete(i.id, 'single'),
      });
    }
    return acts;
  }

  function relativeNextRetry(iso: string | null): string {
    if (!iso) return '';
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    const mins = Math.round((d.getTime() - Date.now()) / 60000);
    if (mins <= 0) return 'sekarang';
    if (mins < 60) return `±${mins} menit lagi`;
    return `±${Math.round(mins / 60)} jam lagi`;
  }
</script>

<AppShell title="Email Outbox">
  <PageHeader
    title="Email Outbox"
    eyebrow="Komunikasi"
    desc="Antrian email sistem: notifikasi tagihan, tiket, dan pengumuman yang dikirim atas nama tenant."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="download" onclick={() => void exportCsv()}>Ekspor CSV</Button>
      <Button variant="ghost" icon="refresh" onclick={() => void Promise.all([refreshStats(), load()])}>
        Muat ulang
      </Button>
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-5">
      {#each statTiles as tile}
        {@const count = stats[tile.st]}
        {@const tone = tile.st === 'failed' && count > 0 ? 'negative' : tile.baseTone}
        <button
          type="button"
          class="rounded-xl text-left focus-ring {statusFilter === tile.st ? 'ring-2 ring-ink-900' : ''}"
          onclick={() => setStatusFilter(tile.st)}
          aria-pressed={statusFilter === tile.st}
        >
          <StatTile label={tile.label} value={String(count)} hint={tile.hint} {tone} />
        </button>
      {/each}
    </div>
  </Card>

  {#if attention.length}
    <div class="mt-4">
      <AttentionPanel items={attention} title="Perlu perhatian" />
    </div>
  {/if}

  <div class="mt-4">
    <Card>
      <div class="mb-3 flex flex-wrap items-center gap-2">
        {#if isSuper}
          <div class="flex rounded-lg bg-ink-100 p-0.5" role="group" aria-label="Cakupan">
            {#each [['tenant', 'Tenant'], ['global', 'Global'], ['all', 'Semua']] as [v, l] (v)}
              <button
                type="button"
                class="rounded-md px-3 py-1.5 text-sm font-medium transition {scope === v
                  ? 'bg-white text-ink-900 shadow-sm'
                  : 'text-ink-500 hover:text-ink-700'}"
                onclick={() => setScope(v as Scope)}
              >
                {l}
              </button>
            {/each}
          </div>
        {/if}
        <div class="relative min-w-[220px] flex-1">
          <Icon
            name="search"
            size={15}
            class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400"
          />
          <input
            bind:value={search}
            placeholder="Cari email penerima atau subjek"
            aria-label="Cari email"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>
        {#if selectedCount > 0}
          <span class="rounded-full bg-ink-100 px-3 py-1 text-sm text-ink-700">
            {selectedCount} dipilih
          </span>
          {#if canRetry}
            <Button variant="ghost" icon="refresh" disabled={bulkBusy} onclick={() => void bulkRetry()}>
              Coba ulang terpilih
            </Button>
          {/if}
          {#if canDelete}
            <Button variant="danger" icon="close" disabled={bulkBusy} onclick={() => confirmDelete(null, 'bulk')}>
              Hapus terpilih
            </Button>
          {/if}
          <Button variant="ghost" onclick={() => (selectedIds = [])}>Batal pilih</Button>
        {:else}
          {#if canRetry}
            <Button variant="ghost" icon="check" onclick={selectVisibleRetryable}>
              Pilih yang bisa diulang
            </Button>
          {/if}
        {/if}
      </div>

      <DataTable
        {columns}
        rows={items}
        {loading}
        emptyTitle="Tidak ada email"
        emptyHint={search
          ? 'Coba kata kunci lain atau hapus pencarian.'
          : statusFilter === 'all'
            ? 'Belum ada email yang masuk antrian.'
            : `Tidak ada email berstatus ${outboxStatusLabel(statusFilter).toLowerCase()}.`}
        footNote={`${items.length} dari ${total} email · halaman ${pageNum}/${totalPages}`}
      >
        {#snippet cell(row, col)}
          {#if col.key === 'sel'}
            <input
              type="checkbox"
              class="h-4 w-4 accent-ink-900"
              checked={isSelected(row.id)}
              onchange={() => toggleSelected(row.id)}
              aria-label={`Pilih email ke ${row.to_email}`}
            />
          {:else if col.key === 'to'}
            <div class="min-w-0 max-w-[260px]">
              <div class="truncate font-medium text-ink-900">{row.to_email}</div>
              <div class="truncate text-sm text-ink-500">{formatDateTime(row.created_at)}</div>
            </div>
          {:else if col.key === 'subject'}
            <div class="min-w-0 max-w-[380px]">
              <div class="truncate font-medium text-ink-900">{row.subject}</div>
              {#if row.status === 'failed' && row.last_error}
                <div class="truncate text-sm text-red-600" title={row.last_error}>{row.last_error}</div>
              {:else if row.delivery_status_summary}
                <div class="truncate text-sm text-ink-500">{row.delivery_status_summary}</div>
              {/if}
            </div>
          {:else if col.key === 'status'}
            <div class="flex flex-col items-start gap-1">
              <Badge tone={outboxStatusTone(row.status)} label={outboxStatusLabel(row.status)} />
              {#if row.status === 'queued' && row.next_retry_at}
                <span class="text-xs text-ink-500">
                  antrian ulang {relativeNextRetry(row.next_retry_at)}
                </span>
              {/if}
            </div>
          {:else if col.key === 'attempts'}
            <span class="tabular-nums text-ink-700">{row.attempts}/{row.max_attempts}</span>
          {:else if col.key === 'scheduled'}
            <span class="text-sm text-ink-500">{formatDateTime(row.scheduled_at)}</span>
          {:else if col.key === 'updated'}
            <span class="text-sm text-ink-500">{formatDateTime(row.updated_at)}</span>
          {:else if col.key === 'actions'}
            <RowActions
              primary={{ label: 'Lihat isi', icon: 'mail', onclick: () => void openDetails(row.id) }}
              rest={rowRest(row)}
            />
          {/if}
        {/snippet}
      </DataTable>

      {#if totalPages > 1}
        <div class="mt-3 flex items-center justify-between">
          <Button
            variant="ghost"
            disabled={pageNum <= 1 || loading}
            onclick={() => {
              pageNum -= 1;
              void load();
            }}
          >
            Sebelumnya
          </Button>
          <span class="text-sm text-ink-500">Halaman {pageNum} dari {totalPages}</span>
          <Button
            variant="ghost"
            disabled={pageNum >= totalPages || loading}
            onclick={() => {
              pageNum += 1;
              void load();
            }}
          >
            Berikutnya
          </Button>
        </div>
      {/if}
    </Card>
  </div>
</AppShell>

<Modal
  bind:show={detailOpen}
  title="Isi email"
  width="920px"
  onclose={() => {
    detailItem = null;
    detailLoading = false;
  }}
>
  {#snippet children()}
    {#if detailLoading}
      <div class="flex items-center gap-3 py-8 text-ink-500">
        <div class="h-4 w-4 animate-spin rounded-full border-2 border-ink-300 border-t-ink-700"></div>
        Memuat…
      </div>
    {:else if detailItem}
      {@const d = detailItem}
      <div class="grid gap-x-6 gap-y-3 py-1 sm:grid-cols-2">
        <div>
          <div class="text-[13px] font-medium text-ink-500">Penerima</div>
          <div class="text-ink-900">{d.to_email}</div>
        </div>
        <div>
          <div class="text-[13px] font-medium text-ink-500">Status</div>
          <div class="flex items-center gap-2">
            <Badge tone={outboxStatusTone(d.status)} label={outboxStatusLabel(d.status)} />
            <span class="text-sm text-ink-500">{d.attempts}/{d.max_attempts} percobaan</span>
          </div>
        </div>
        <div>
          <div class="text-[13px] font-medium text-ink-500">Dijadwalkan</div>
          <div class="text-ink-900">{formatDateTime(d.scheduled_at)}</div>
        </div>
        <div>
          <div class="text-[13px] font-medium text-ink-500">Terkirim</div>
          <div class="text-ink-900">{d.sent_at ? formatDateTime(d.sent_at) : '—'}</div>
        </div>
        <div class="sm:col-span-2">
          <div class="text-[13px] font-medium text-ink-500">Subjek</div>
          <div class="text-ink-900">{d.subject}</div>
        </div>
        <div class="sm:col-span-2">
          <div class="text-[13px] font-medium text-ink-500">Ringkasan</div>
          <div class="text-ink-700">
            {d.delivery_status_summary ||
              deliverySummary(d.status, d.attempts, d.max_attempts, rowRetryable(d))}
          </div>
        </div>
        {#if d.last_error}
          <div class="sm:col-span-2">
            <div class="text-[13px] font-medium text-ink-500">Error terakhir</div>
            <div class="rounded-lg bg-red-50 p-2 font-mono text-sm break-all text-red-700">
              {d.last_error}
            </div>
          </div>
        {/if}
      </div>

      <div class="mt-4 flex gap-1 border-b border-ink-200">
        <button
          type="button"
          class="px-3 py-2 text-sm font-medium {detailTab === 'text'
            ? 'border-b-2 border-ink-900 text-ink-900'
            : 'text-ink-500 hover:text-ink-700'}"
          onclick={() => (detailTab = 'text')}
        >
          Teks
        </button>
        {#if d.body_html}
          <button
            type="button"
            class="px-3 py-2 text-sm font-medium {detailTab === 'html'
              ? 'border-b-2 border-ink-900 text-ink-900'
              : 'text-ink-500 hover:text-ink-700'}"
            onclick={() => (detailTab = 'html')}
          >
            Pratinjau HTML
          </button>
        {/if}
      </div>

      {#if detailTab === 'html' && d.body_html}
        <iframe
          class="mt-3 h-[420px] w-full rounded-xl bg-white ring-1 ring-inset ring-ink-200"
          sandbox=""
          srcdoc={d.body_html}
          title="Pratinjau HTML"
        ></iframe>
        <details class="mt-2">
          <summary class="cursor-pointer text-sm text-ink-500 hover:text-ink-700">Lihat sumber HTML</summary>
          <pre class="mt-2 max-h-72 overflow-auto rounded-lg bg-ink-50 p-3 font-mono text-xs text-ink-700">{d.body_html}</pre>
        </details>
      {:else}
        <pre class="mt-3 max-h-[420px] overflow-auto rounded-xl bg-ink-50 p-3 font-mono text-sm whitespace-pre-wrap text-ink-800">{d.body}</pre>
      {/if}
    {:else}
      <div class="py-6 text-ink-500">Email tidak ditemukan.</div>
    {/if}
  {/snippet}
</Modal>

<Modal bind:show={confirmOpen} title={confirmMode === 'bulk' ? 'Hapus email terpilih' : 'Hapus email'} width="460px">
  {#snippet children()}
    <p class="py-2 text-ink-700">
      {#if confirmMode === 'bulk'}
        {selectedIds.length} baris akan dihapus permanen dari antrian. Riwayat email yang sudah
        terkirim juga hilang.
      {:else}
        Email ini akan dihapus permanen dari antrian.
      {/if}
    </p>
  {/snippet}
  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <Button variant="ghost" onclick={() => (confirmOpen = false)}>Batal</Button>
      <Button variant="danger" onclick={() => void handleConfirmDelete()}>Hapus</Button>
    </div>
  {/snippet}
</Modal>
