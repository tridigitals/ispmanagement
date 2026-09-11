<script lang="ts">
  /*
    Tagihan v2 — halaman uang. Versi lama: 1.147 baris, 7.982 karakter CSS.

    Keputusan yang membedakan dari versi lama:

    1. ANGKA AGREGAT DIHITUNG ATAS SELURUH DATA, BUKAN SATU HALAMAN.
       Versi lama menampilkan tabel berpaginasi 25 baris tanpa ringkasan sama
       sekali, jadi tidak ada satu tempat pun di aplikasi yang memberi tahu
       "berapa total piutang". Di sini ringkasan diambil lewat `fetchAllPages`
       (backend clamp per_page ke 100) dan ditandai jujur bila belum lengkap.

    2. FILTER STATUS JADI CHIP BERANGKA. Versi lama memakai dropdown status:
       pengguna harus membuka dropdown untuk tahu pilihannya, dan tidak pernah
       tahu ada berapa banyak per status.

    3. PILIH BANYAK -> KIRIM. Aksi massal tetap ada (bulkSendInvoices), tapi
       tombolnya baru muncul setelah ada baris terpilih, bukan selalu nongkrong
       di toolbar.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import { formatRupiah, formatDate, formatPercent } from '$lib/components/ds/format';
  import type { Column } from '$lib/components/ds/table-types';
  import { fetchAllPages } from '$lib/utils/fetchAllPages';
  import type { Invoice } from '$lib/api/types';

  import { t } from 'svelte-i18n';
  type StatusKey = 'all' | 'pending' | 'verification_pending' | 'paid' | 'failed';

  let rows = $state<Invoice[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = $state(25);
  let statusFilter = $state<StatusKey>('all');
  let loading = $state(true);
  let err = $state('');

  /* Ringkasan seluruh tenant, terpisah dari tabel berpaginasi. */
  let summary = $state({
    count: 0,
    complete: true,
    pending: { n: 0, amount: 0 },
    paid: { n: 0, amount: 0 },
    verification: { n: 0, amount: 0 },
    failed: { n: 0, amount: 0 },
    overdue: { n: 0, amount: 0 },
    /* Aging: piutang yang lewat >90 hari. Terukur di tenant ini 474 dari 476
       piutang sudah >90 hari (tertua 754 hari), jadi tile "jatuh tempo" dan
       "piutang" akan selalu kembar — aging yang benar-benar membedakan. */
    aged90: { n: 0, amount: 0 },
  });
  let summaryLoading = $state(true);

  let selected = $state<Set<string>>(new Set());
  let sending = $state(false);
  let generating = $state(false);
  let notice = $state('');

  /* Resource-nya 'billing', bukan 'payments' — tabel permissions tidak punya baris
     'payments' sama sekali, jadi versi lama silently false untuk semua non-Owner dan
     menyembunyikan tombol aksi dari Admin yang berhak. Halaman legacy memakai
     can('manage','billing'). */
  const canManage = $derived($can('manage', 'billing'));
  const columns = $derived<Column[]>([
    ...(canManage ? [{ key: 'select', label: '', width: '44px' }] : []),
    { key: 'invoice_number', label: $t('admin.invoices.columns.invoice_number'), num: true },
    { key: 'description', label: $t('admin.invoices.columns.description'), hideSm: true },
    { key: 'amount', label: $t('admin.invoices.columns.amount'), align: 'right', num: true },
    { key: 'status', label: $t('common.status') },
    { key: 'due_date', label: $t('admin.invoices.columns.due_date'), hideSm: true },
    { key: 'actions', label: $t('admin.invoices.v2list.col_actions'), align: 'right', width: '110px' },
  ]);

  const billed = $derived(
    summary.pending.amount +
      summary.paid.amount +
      summary.verification.amount +
      summary.failed.amount,
  );
  const collectionRate = $derived(billed === 0 ? 0 : (summary.paid.amount / billed) * 100);

  const chips = $derived([
    { key: 'all' as StatusKey, label: $t('common.all'), count: summary.count },
    { key: 'pending' as StatusKey, label: $t('admin.invoices.v2list.status.pending'), count: summary.pending.n },
    {
      key: 'verification_pending' as StatusKey,
      label: $t('admin.invoices.v2list.status.verification_pending'),
      count: summary.verification.n,
    },
    { key: 'paid' as StatusKey, label: $t('admin.invoices.v2list.status.paid'), count: summary.paid.n },
    { key: 'failed' as StatusKey, label: $t('admin.invoices.v2list.status.failed'), count: summary.failed.n },
  ]);

  function isOverdue(inv: Invoice): boolean {
    if (inv.status !== 'pending' && inv.status !== 'verification_pending') return false;
    return !!inv.due_date && new Date(inv.due_date).getTime() < Date.now();
  }

  /** Umur tunggakan dalam hari; negatif berarti belum jatuh tempo. */
  function daysLate(inv: Invoice): number {
    if (!inv.due_date) return 0;
    return Math.floor((Date.now() - new Date(inv.due_date).getTime()) / 86_400_000);
  }

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  function toggleAll() {
    selected = selected.size === rows.length ? new Set() : new Set(rows.map((r) => r.id));
  }

  async function load() {
    loading = true;
    err = '';
    try {
      const res = await api.payment.listCustomerPackageInvoices({
        status: statusFilter === 'all' ? undefined : statusFilter,
        sort_by: 'due_date',
        sort_dir: 'asc',
        page,
        per_page: perPage,
      });
      rows = res.data ?? [];
      total = res.total ?? 0;
      selected = new Set();
    } catch (e) {
      err = 'Gagal memuat daftar tagihan';
      console.warn('list invoices gagal', e);
    } finally {
      loading = false;
    }
  }

  async function loadSummary() {
    summaryLoading = true;
    try {
      /* Semua halaman, bukan satu: lihat catatan 1 di kepala file. */
      const { rows: all, total: n, complete } = await fetchAllPages<Invoice>((p, per_page) =>
        api.payment.listCustomerPackageInvoices({ page: p, per_page }),
      );

      const bucket = (predicate: (i: Invoice) => boolean) => {
        const list = all.filter(predicate);
        return { n: list.length, amount: list.reduce((s, i) => s + (i.amount ?? 0), 0) };
      };

      summary = {
        count: n,
        complete,
        pending: bucket((i) => i.status === 'pending'),
        paid: bucket((i) => i.status === 'paid'),
        verification: bucket((i) => i.status === 'verification_pending'),
        failed: bucket((i) => i.status === 'failed'),
        overdue: bucket(isOverdue),
        aged90: bucket((i) => isOverdue(i) && daysLate(i) > 90),
      };
    } catch (e) {
      console.warn('ringkasan tagihan gagal', e);
    } finally {
      summaryLoading = false;
    }
  }

  async function sendSelected() {
    if (selected.size === 0) return;
    sending = true;
    notice = '';
    try {
      const res = await api.payment.bulkSendInvoices({
        invoice_ids: [...selected],
        channels: ['email', 'notification'],
        attach_pdf: true,
      });
      const parts = [`terkirim ${res.sent_count}`];
      if (res.skipped_count > 0) parts.push(`dilewati ${res.skipped_count}`);
      if (res.failed_count > 0) parts.push(`gagal ${res.failed_count}`);
      notice = `Dari ${res.total} tagihan: ${parts.join(', ')}.`;
      selected = new Set();
    } catch (e) {
      err = `Gagal mengirim tagihan: ${(e as Error)?.message ?? e}`;
    } finally {
      sending = false;
    }
  }

  async function generateDue() {
    generating = true;
    notice = '';
    try {
      const res = await api.payment.generateDueCustomerPackageInvoices();
      notice =
        `Dibuat ${res.created_count} tagihan baru` +
        (res.skipped_count > 0 ? `, dilewati ${res.skipped_count}` : '') +
        (res.failed_count > 0 ? `, gagal ${res.failed_count}` : '') +
        '.';
      await Promise.all([load(), loadSummary()]);
    } catch (e) {
      err = `Gagal membuat tagihan: ${(e as Error)?.message ?? e}`;
    } finally {
      generating = false;
    }
  }

  function applyChip(key: StatusKey) {
    statusFilter = key;
    page = 1;
    load();
  }

  onMount(() => {
    void load();
    void loadSummary();
  });
</script>

<AppShell title={ $t('admin.invoices.v2list.title') } badges={{ invoicesOverdue: summary.overdue.n }}>
  <PageHeader
    title={ $t('admin.invoices.v2list.title') }
    eyebrow={summaryLoading
      ? $t('admin.invoices.v2list.ey_calc')
      : summary.complete
        ? $t('admin.invoices.v2list.ey_count', { values: { n: summary.count } })
        : $t('admin.invoices.v2list.ey_min', { values: { n: summary.count } })}
    desc={ $t('admin.invoices.v2list.desc') }
  >
    {#snippet actions()}
      {#if canManage}
        <Button icon="refresh" loading={generating} onclick={generateDue}>
          Buat tagihan jatuh tempo
        </Button>
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

  {#if notice}
    <div
      role="status"
      class="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-3.5 py-2.5 text-base text-emerald-800"
    >
      {notice}
    </div>
  {/if}

  <!-- Ringkasan uang: 4 angka yang menentukan keputusan hari ini. -->
  <Card class="mb-4">
    <div class="grid grid-cols-2 gap-5 lg:grid-cols-4">
      <StatTile
        label={ $t('admin.invoices.v2list.st_receivable') }
        value={formatRupiah(summary.pending.amount + summary.verification.amount)}
        hint={summaryLoading
          ? $t('admin.invoices.v2list.calculating')
          : $t('admin.invoices.v2list.h_unpaid', { values: { a: summary.pending.n + summary.verification.n, b: summary.overdue.n } })}
        tone="negative"
      />
      <StatTile
        label={ $t('admin.invoices.v2list.st_over90') }
        value={formatRupiah(summary.aged90.amount)}
        hint={summaryLoading
          ? $t('admin.invoices.v2list.calculating')
          : $t('admin.invoices.v2list.h_of_due', { values: { a: summary.aged90.n, b: summary.overdue.n } })}
        tone="negative"
      />
      <StatTile
        label={ $t('admin.invoices.v2list.st_paid') }
        value={formatRupiah(summary.paid.amount)}
        hint={summaryLoading ? 'menghitung…' : `${summary.paid.n} tagihan lunas`}
        tone="positive"
      />
      <StatTile
        label={ $t('admin.invoices.v2list.st_collection') }
        value={formatPercent(collectionRate)}
        hint={summaryLoading ? $t('admin.invoices.v2list.calculating') : $t('admin.invoices.v2list.h_issued', { values: { a: formatRupiah(billed) } })}
        tone={collectionRate < 50 ? 'negative' : 'positive'}
      />
    </div>
  </Card>

  <!-- Filter + aksi massal -->
  <div class="mb-3 flex flex-wrap items-center gap-2">
    {#each chips as chip (chip.key)}
      <button
        onclick={() => applyChip(chip.key)}
        aria-pressed={statusFilter === chip.key}
        class="focus-ring flex h-8 items-center gap-1.5 rounded-lg px-3 text-base
          {statusFilter === chip.key
          ? 'bg-ink-900 font-medium text-white'
          : 'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50'}"
      >
        {chip.label}
        <span class="num text-sm {statusFilter === chip.key ? 'text-ink-300' : 'text-ink-400'}">
          {chip.count}
        </span>
      </button>
    {/each}

    {#if canManage}
      <div class="ml-auto flex items-center gap-2">
        {#if selected.size > 0}
          <span class="num text-sm text-ink-500">{selected.size} dipilih</span>
          <Button size="sm" variant="ghost" onclick={() => (selected = new Set())}>Bersihkan</Button>
          <Button variant="primary" icon="mail" loading={sending} onclick={sendSelected}>
            Kirim tagihan
          </Button>
        {:else}
          <Button size="sm" variant="ghost" icon="check" onclick={toggleAll}>
            Pilih semua di halaman ini
          </Button>
        {/if}
      </div>
    {/if}
  </div>

  <Card padded={false}>
    <DataTable
      {columns}
      rows={rows}
      {loading}
      pageSize={perPage}
      page={page}
      {total}
      onpage={(p) => {
        page = p;
        load();
      }}
      onpagesize={(n) => {
        perPage = n;
        page = 1;
        load();
      }}
      emptyTitle="Tidak ada tagihan"
      emptyHint="Coba pilih filter status yang lain."
      footNote={`${total} tagihan`}
    >
      {#snippet cell(inv: Invoice, col: Column)}
        {#if col.key === 'select'}
          <label class="inline-flex size-6 cursor-pointer items-center justify-center">
            <input
              type="checkbox"
              checked={selected.has(inv.id)}
              onchange={() => toggle(inv.id)}
              aria-label={ $t('admin.invoices.v2list.pick_aria', { values: { n: inv.invoice_number } }) }
              class="size-4 rounded border-ink-300"
            />
          </label>
        {:else if col.key === 'invoice_number'}
          <span class="font-medium text-ink-900">{inv.invoice_number}</span>
        {:else if col.key === 'description'}
          <div class="max-w-xs truncate">{inv.description || '—'}</div>
        {:else if col.key === 'amount'}
          <span class="text-ink-900">{formatRupiah(inv.amount)}</span>
        {:else if col.key === 'status'}
          <div class="flex items-center gap-1.5">
            <Badge status={inv.status} />
            {#if isOverdue(inv)}
              <!-- Umur tunggakan lebih berguna daripada label "lewat"
                   telanjang: 474 dari 476 piutang di sini sudah >90 hari. -->
              <Badge tone="negative" label="{ $t('admin.invoices.v2list.days_late', { values: { n: daysLate(inv) } }) }" />
            {/if}
          </div>
        {:else if col.key === 'due_date'}
          <span>{formatDate(inv.due_date)}</span>
        {:else if col.key === 'actions'}
          <RowActions
            primary={{
              label: $t('common.details'),
              icon: 'chevronRight',
              onclick: () => goto(`/v2/admin/invoices/${inv.id}`),
            }}
            rest={canManage
              ? [
                  { label: $t('admin.invoices.v2list.pay_page'), icon: 'chevronRight', onclick: () => window.open(`/pay/${inv.id}`, '_blank') },
                  { label: $t('admin.invoices.v2list.verify_page'), icon: 'check' },
                  { label: $t('admin.invoices.v2list.resend'), icon: 'mail' },
                ]
              : []}
          />
        {/if}
      {/snippet}
    </DataTable>
  </Card>
</AppShell>
