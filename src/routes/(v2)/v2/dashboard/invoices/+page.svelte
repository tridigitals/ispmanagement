<script lang="ts">
  /*
    Tagihan portal v2 — gelombang 25b.
    Versi lama: (app)/dashboard/invoices/+page.svelte (651 baris).
    Perilaku identik: alert overdue + KPI + tabel (cari, sort) + bayar + cetak.
    Pola DS: PortalShell + PageHeader + StatTile + DataTable + Badge + Button.
  */
  import { onMount } from 'svelte';
  import { api, type Invoice } from '$lib/api/client';
  import type { BankAccount, Customer } from '$lib/api/types';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { user } from '$lib/stores/auth';
  import { formatMoney } from '$lib/utils/money';
  import { formatDate } from '$lib/utils/date';
  import { goto } from '$app/navigation';
  import { get } from 'svelte/store';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import InvoicePrintModal from '$lib/components/invoice/InvoicePrintModal.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import type { StatusTone } from '$lib/components/ds/tokens';

  let invoices = $state<Invoice[]>([]);
  let loading = $state(true);
  let error = $state('');
  let search = $state('');
  let sortKey = $state<'invoice_number' | 'description' | 'amount' | 'status' | 'due_date'>('due_date');
  let sortDir = $state<'asc' | 'desc'>('asc');

  const OPEN_STATUSES = new Set(['pending', 'verification_pending', 'failed']);

  const summary = $derived.by(() => {
    const invs = invoices;
    const open = invs.filter((i) => OPEN_STATUSES.has(String(i.status || '').toLowerCase()));
    const overdue = open.filter((i) => {
      const st = String(i.status || '').toLowerCase();
      if (st === 'verification_pending') return false;
      if (!i.due_date) return false;
      return new Date(i.due_date).getTime() < Date.now();
    });
    const payable = open.filter((i) => String(i.status || '').toLowerCase() !== 'verification_pending');
    return {
      total: invs.length,
      open: open.length,
      paid: invs.filter((i) => String(i.status || '').toLowerCase() === 'paid').length,
      overdue: overdue.length,
      pendingTotal: open.reduce((sum, i) => sum + (Number(i.amount) || 0), 0),
      payableTotal: payable.reduce((sum, i) => sum + (Number(i.amount) || 0), 0),
      firstPayable: payable[0] || overdue[0] || null,
      firstOverdue: overdue[0] || null,
    };
  });

  const visible = $derived.by(() => {
    const q = search.trim().toLowerCase();
    let rows = q
      ? invoices.filter((i) =>
          [i.invoice_number, i.description, i.status].some((v) => String(v || '').toLowerCase().includes(q)),
        )
      : [...invoices];
    const dir = sortDir === 'asc' ? 1 : -1;
    rows.sort((a, b) => {
      const av = String((a as any)[sortKey] ?? '').toLowerCase();
      const bv = String((b as any)[sortKey] ?? '').toLowerCase();
      if (sortKey === 'amount') return ((Number(a.amount) || 0) - (Number(b.amount) || 0)) * dir;
      return av < bv ? -dir : av > bv ? dir : 0;
    });
    return rows;
  });

  let showPrintModal = $state(false);
  let printInvoice = $state<Invoice | null>(null);
  let printCustomer = $state<Customer | null>(null);
  let printBankAccounts = $state<BankAccount[]>([]);
  let printPreparing = $state(false);
  let cachedBanks = $state<BankAccount[] | null>(null);

  const columns: Column[] = $derived([
    { key: 'invoice_number', label: 'Invoice' },
    { key: 'description', label: 'Deskripsi' },
    { key: 'amount', label: 'Jumlah', align: 'right', num: true },
    { key: 'status', label: 'Status' },
    { key: 'due_date', label: 'Jatuh tempo' },
    { key: 'actions', label: '', align: 'right' },
  ] as Column[]);

  onMount(() => {
    loadInvoices();
  });

  async function loadInvoices() {
    loading = true;
    error = '';
    try {
      invoices = await api.payment.listInvoices();
    } catch (e: any) {
      error = e.toString();
      toast.error('Gagal memuat tagihan.');
    } finally {
      loading = false;
    }
  }

  function formatCurrency(amount: number, currency?: string) {
    return formatMoney(amount, { currency });
  }
  function statusKey(status?: string | null) {
    return String(status || '').toLowerCase();
  }
  function statusLabel(status?: string | null) {
    const st = statusKey(status);
    if (st === 'verification_pending') return 'Menunggu verifikasi';
    if (st === 'pending') return 'Menunggu bayar';
    if (st === 'paid') return 'Lunas';
    if (st === 'failed') return 'Gagal';
    if (st === 'cancelled' || st === 'canceled') return 'Dibatalkan';
    return status || '—';
  }
  function statusTone(item: Invoice): StatusTone {
    const st = statusKey(item.status);
    if (isOverdue(item)) return 'negative';
    if (st === 'paid') return 'positive';
    if (st === 'failed') return 'negative';
    if (st === 'verification_pending') return 'warning';
    if (st === 'pending') return 'info';
    return 'neutral';
  }
  function isOverdue(item: Invoice) {
    const st = statusKey(item.status);
    if (st !== 'pending' && st !== 'failed') return false;
    if (!item.due_date) return false;
    return new Date(item.due_date).getTime() < Date.now();
  }
  function canPay(item: Invoice) {
    const st = statusKey(item.status);
    return st === 'pending' || st === 'failed';
  }
  function payFirst() {
    const target = summary.firstOverdue || summary.firstPayable;
    if (target) goto(`/pay/${target.id}`);
  }
  function toggleSort(key: string) {
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key as typeof sortKey;
      sortDir = 'asc';
    }
  }

  async function openPrintModal(item: Invoice) {
    if (printPreparing) return;
    printPreparing = true;
    try {
      printInvoice = item;
      const u = get(user) as any;
      printCustomer = u
        ? {
            id: u.id,
            tenant_id: u.tenant_id || '',
            name: u.name || u.email || 'Pelanggan',
            email: u.email || null,
            phone: u.phone || null,
            customer_number: null,
            notes: null,
            is_active: true,
            created_at: '',
            updated_at: '',
          }
        : null;
      if (cachedBanks === null) {
        try {
          cachedBanks = await api.payment.listBanks();
        } catch {
          cachedBanks = [];
        }
      }
      printBankAccounts = cachedBanks;
      showPrintModal = true;
    } catch (e: any) {
      toast.error(e?.message || 'Gagal menyiapkan invoice untuk dicetak.');
    } finally {
      printPreparing = false;
    }
  }
</script>

<PortalShell title="Tagihan">
  {#if !loading && summary.overdue > 0 && summary.firstOverdue}
    <div class="alert-overdue" role="alert">
      <div class="flex items-center gap-3">
        <Icon name="alert" size={20} />
        <div>
          <p class="text-sm font-semibold">{summary.overdue} tagihan jatuh tempo</p>
          <p class="text-xs text-ink-300">
            {summary.firstOverdue.description || summary.firstOverdue.invoice_number}
            · {formatCurrency(summary.firstOverdue.amount, summary.firstOverdue.currency_code)}
          </p>
        </div>
      </div>
      <Button variant="danger" size="sm" onclick={() => goto(`/pay/${summary.firstOverdue!.id}`)}>
        Bayar sekarang
      </Button>
    </div>
  {/if}

  <PageHeader
    title="Tagihan"
    desc={loading
      ? 'Memuat tagihan…'
      : `${summary.total} invoice` +
        (summary.open > 0 ? ` · ${summary.open} terbuka` : '') +
        (summary.overdue > 0 ? ` · ${summary.overdue} jatuh tempo` : '')}
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={loadInvoices}>Segarkan</Button>
      {#if summary.firstPayable || summary.firstOverdue}
        <Button icon="card" onclick={payFirst}>
          {summary.payableTotal > 0 ? `Bayar · ${formatCurrency(summary.payableTotal)}` : 'Lihat tagihan'}
        </Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if !loading && !error}
    <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
      <StatTile label="Total" value={String(summary.total)} hint="semua invoice" />
      <StatTile
        label="Terbuka"
        value={summary.open > 0 ? formatCurrency(summary.pendingTotal) : 'Rp 0'}
        hint={`${summary.open} menunggu`}
        tone={summary.open > 0 ? 'warning' : 'neutral'}
      />
      <StatTile
        label="Jatuh tempo"
        value={String(summary.overdue)}
        hint="perlu bayar"
        tone={summary.overdue > 0 ? 'negative' : 'positive'}
      />
      <StatTile label="Lunas" value={String(summary.paid)} hint="sudah dibayar" tone="positive" />
    </div>
  {/if}

  <Card title="Daftar tagihan" padded={false}>
    <div class="flex flex-wrap items-end gap-3 border-b border-ink-200 p-3">
      <div class="min-w-56 flex-1">
        <Field
          id="inv-search"
          label="Cari"
          type="text"
          value={search}
          placeholder="Cari invoice…"
          onchange={(v) => (search = v)}
        />
      </div>
      <div class="w-44">
        <Field
          id="inv-sort"
          label="Urut"
          type="select"
          value={sortKey}
          options={[
            { value: 'due_date', label: 'Jatuh tempo' },
            { value: 'amount', label: 'Jumlah' },
            { value: 'invoice_number', label: 'Nomor' },
            { value: 'description', label: 'Deskripsi' },
          ]}
          onchange={(v) => (sortKey = v as typeof sortKey)}
        />
      </div>
      <Button
        variant="ghost"
        size="sm"
        icon="filter"
        onclick={() => (sortDir = sortDir === 'asc' ? 'desc' : 'asc')}
      >
        {sortDir === 'asc' ? 'Terlama dulu' : 'Terbaru dulu'}
      </Button>
    </div>
    {#if error}
      <div class="flex flex-col items-start gap-3 p-6">
        <Icon name="alert" size={28} />
        <div>
          <p class="text-sm font-medium">Gagal memuat</p>
          <p class="text-sm text-ink-500">{error}</p>
        </div>
        <Button variant="secondary" onclick={loadInvoices}>Coba lagi</Button>
      </div>
    {:else}
      <DataTable
        rows={visible}
        pageSize={25}
        {columns}
        {loading}
        emptyTitle="Tidak ada tagihan"
        emptyHint="Tagihan Anda akan tampil di sini."
      >
        {#snippet cell(item: Invoice, col: Column)}
          {#if col.key === 'invoice_number'}
            <span class="font-mono text-xs">{item.invoice_number}</span>
          {:else if col.key === 'description'}
            <span class="block max-w-56 truncate">{item.description || '—'}</span>
          {:else if col.key === 'amount'}
            <span class="tabular-nums font-medium">{formatCurrency(item.amount, item.currency_code)}</span>
          {:else if col.key === 'status'}
            <Badge tone={statusTone(item)} label={isOverdue(item) ? 'Jatuh tempo' : statusLabel(item.status)} />
          {:else if col.key === 'due_date'}
            <span class={isOverdue(item) ? 'text-red-600 font-medium' : ''}>
              {formatDate(item.due_date, { timeZone: $appSettings.app_timezone })}
            </span>
          {:else if col.key === 'actions'}
            {#if canPay(item)}
              <Button size="sm" icon="card" onclick={() => goto(`/pay/${item.id}`)}>Bayar</Button>
            {:else}
              <Button variant="ghost" size="sm" icon="receipt" onclick={() => goto(`/pay/${item.id}`)}>
                Detail
              </Button>
            {/if}
            <Button
              variant="ghost"
              size="sm"
              icon="download"
              loading={printPreparing}
              onclick={() => openPrintModal(item)}
            >
              Cetak
            </Button>
          {/if}
        {/snippet}
      </DataTable>
    {/if}
  </Card>

  {#if printInvoice}
    <InvoicePrintModal
      bind:show={showPrintModal}
      invoice={printInvoice}
      customer={printCustomer}
      bankAccounts={printBankAccounts}
    />
  {/if}
</PortalShell>

<style>
  .alert-overdue {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    border-radius: 0.75rem;
    border: 1px solid #fecaca;
    background: #fef2f2;
    color: #991b1b;
    padding: 0.85rem 1.1rem;
    margin-bottom: 1.25rem;
  }
</style>
