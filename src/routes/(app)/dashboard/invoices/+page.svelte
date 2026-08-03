<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Invoice } from '$lib/api/client';
  import type { BankAccount, Customer } from '$lib/api/types';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import InvoicePrintModal from '$lib/components/invoice/InvoicePrintModal.svelte';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { user } from '$lib/stores/auth';
  import { formatMoney } from '$lib/utils/money';
  import { formatDate } from '$lib/utils/date';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  let invoices = $state<Invoice[]>([]);
  let loading = $state(true);
  let error = $state('');

  const OPEN_STATUSES = new Set(['pending', 'verification_pending', 'failed']);

  let summary = $derived.by(() => {
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

  let showPrintModal = $state(false);
  let printInvoice = $state<Invoice | null>(null);
  let printCustomer = $state<Customer | null>(null);
  let printBankAccounts = $state<BankAccount[]>([]);
  let printPreparing = $state(false);
  let cachedBanks = $state<BankAccount[] | null>(null);

  const columns = $derived.by(() => [
    {
      key: 'invoice_number',
      label: $t('admin.subscription.invoices.invoice_number') || 'Invoice #',
      sortable: true,
    },
    {
      key: 'description',
      label: $t('admin.subscription.invoices.description') || 'Description',
      sortable: true,
    },
    {
      key: 'amount',
      label: $t('admin.subscription.invoices.amount') || 'Amount',
      sortable: true,
    },
    {
      key: 'status',
      label: $t('admin.subscription.invoices.status') || 'Status',
      sortable: true,
    },
    {
      key: 'due_date',
      label: $t('admin.subscription.invoices.due_date') || 'Due Date',
      sortable: true,
    },
    {
      key: 'actions',
      label: $t('admin.subscription.invoices.actions') || 'Actions',
      align: 'right',
    },
  ]);

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
      toast.error(get(t)('admin.invoices.load_error') || 'Failed to load invoices');
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

  function statusClass(status?: string | null) {
    const st = statusKey(status);
    if (st === 'paid') return 'pill-paid';
    if (st === 'verification_pending') return 'pill-verify';
    if (st === 'failed') return 'pill-failed';
    if (st === 'pending') {
      // overdue pending gets warn look via due check in cell
      return 'pill-pending';
    }
    return 'pill-neutral';
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
            name: u.name || u.email || 'Customer',
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
      toast.error(
        e?.message ||
          get(t)('admin.invoices.print_failed') ||
          'Failed to prepare invoice for print',
      );
    } finally {
      printPreparing = false;
    }
  }
</script>

<div class="page fade-in">
  {#if !loading && summary.overdue > 0 && summary.firstOverdue}
    <div class="alert-overdue" role="alert">
      <div class="alert-left">
        <div class="alert-ico"><Icon name="alert-triangle" size={18} /></div>
        <div class="alert-text">
          <strong>{summary.overdue} tagihan jatuh tempo</strong>
          <p>
            {summary.firstOverdue.description || summary.firstOverdue.invoice_number}
            · {formatCurrency(summary.firstOverdue.amount, summary.firstOverdue.currency_code)}
          </p>
        </div>
      </div>
      <button class="btn btn-danger" type="button" onclick={() => goto(`/pay/${summary.firstOverdue!.id}`)}>
        Bayar sekarang
      </button>
    </div>
  {/if}

  <div class="page-head">
    <div class="page-head-text">
      <h1>{$t('admin.invoices.title') || 'Tagihan'}</h1>
      <p class="page-sub">
        {#if loading}
          Memuat tagihan...
        {:else}
          {summary.total} invoice
          {#if summary.open > 0}
            · {summary.open} terbuka
          {/if}
          {#if summary.overdue > 0}
            · {summary.overdue} overdue
          {/if}
        {/if}
      </p>
    </div>
    <div class="head-actions">
      <button class="btn btn-ghost" type="button" onclick={loadInvoices} disabled={loading}>
        <Icon name="refresh-cw" size={14} />
        <span>{$t('common.refresh') || 'Refresh'}</span>
      </button>
      {#if summary.firstPayable || summary.firstOverdue}
        <button class="btn btn-primary" type="button" onclick={payFirst}>
          {#if summary.payableTotal > 0}
            Bayar · {formatCurrency(summary.payableTotal)}
          {:else}
            Lihat tagihan
          {/if}
        </button>
      {/if}
    </div>
  </div>

  {#if !loading && !error}
    <div class="kpis">
      <div class="kpi">
        <div class="kpi-label">Total</div>
        <div class="kpi-val">{summary.total}</div>
        <div class="kpi-sub">semua invoice</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Terbuka</div>
        <div class="kpi-val {summary.open > 0 ? 'warn' : ''}">
          {summary.open > 0 ? formatCurrency(summary.pendingTotal) : 'Rp 0'}
        </div>
        <div class="kpi-sub">{summary.open} menunggu</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Jatuh tempo</div>
        <div class="kpi-val {summary.overdue > 0 ? 'bad' : 'ok'}">{summary.overdue}</div>
        <div class="kpi-sub">perlu bayar</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Lunas</div>
        <div class="kpi-val ok">{summary.paid}</div>
        <div class="kpi-sub">sudah dibayar</div>
      </div>
    </div>
  {/if}

  <div class="panel">
    {#if error}
      <div class="panel-pad">
        <div class="empty">
          <Icon name="alert-triangle" size={36} />
          <h3>Gagal memuat</h3>
          <p>{error}</p>
          <button class="btn btn-secondary" type="button" onclick={loadInvoices}>Coba lagi</button>
        </div>
      </div>
    {:else}
      <Table
        {loading}
        data={invoices}
        {columns}
        searchable={true}
        searchPlaceholder={$t('admin.invoices.search_placeholder') || 'Cari invoice...'}
      >
        {#snippet cell({ item, column })}
          {#if column.key === 'amount'}
            <span class="amount">{formatCurrency(item.amount, item.currency_code)}</span>
          {:else if column.key === 'status'}
            <span class="pill {statusClass(item.status)} {isOverdue(item) ? 'pill-overdue' : ''}">
              {isOverdue(item) ? 'Jatuh tempo' : statusLabel(item.status)}
            </span>
          {:else if column.key === 'due_date'}
            <span class={isOverdue(item) ? 'due-bad' : ''}>
              {formatDate(item[column.key], { timeZone: $appSettings.app_timezone })}
            </span>
          {:else if column.key === 'actions'}
            <div class="actions">
              {#if canPay(item)}
                <button
                  type="button"
                  class="btn btn-primary btn-sm"
                  onclick={() => goto(`/pay/${item.id}`)}
                >
                  <Icon name="credit-card" size={14} />
                  {$t('admin.invoices.pay_now') || 'Bayar'}
                </button>
              {:else}
                <button
                  type="button"
                  class="btn btn-ghost btn-sm"
                  onclick={() => goto(`/pay/${item.id}`)}
                >
                  <Icon name="eye" size={14} />
                  Detail
                </button>
              {/if}
              <button
                type="button"
                class="icon-btn"
                title={$t('admin.invoices.print_pdf') || 'Print'}
                aria-label={$t('admin.invoices.print_pdf') || 'Print'}
                onclick={() => openPrintModal(item)}
                disabled={printPreparing}
              >
                <Icon name="printer" size={16} />
              </button>
            </div>
          {:else if column.key === 'invoice_number'}
            <span class="mono">{item.invoice_number}</span>
          {:else}
            {item[column.key]}
          {/if}
        {/snippet}
      </Table>
    {/if}
  </div>
</div>

{#if printInvoice}
  <InvoicePrintModal
    bind:show={showPrintModal}
    invoice={printInvoice}
    customer={printCustomer}
    bankAccounts={printBankAccounts}
  />
{/if}

<style>
  .page {
    padding: clamp(1rem, 2.2vw, 1.75rem);
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .alert-overdue {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    padding: 0.9rem 1.1rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--color-danger) 35%, transparent);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  }
  .alert-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
  }
  .alert-ico {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--color-danger) 20%, transparent);
    color: var(--color-danger);
    flex-shrink: 0;
  }
  .alert-text strong {
    display: block;
    font-size: 0.92rem;
    color: var(--text-primary);
  }
  .alert-text p {
    margin: 0.15rem 0 0;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .page-head h1 {
    font-size: clamp(1.25rem, 2.2vw, 1.45rem);
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
    color: var(--text-primary);
  }
  .page-sub {
    color: var(--text-secondary);
    font-size: 0.88rem;
    margin: 0.25rem 0 0;
  }
  .head-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .kpis {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.7rem;
  }
  .kpi {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    padding: 0.9rem 1rem;
  }
  .kpi-label {
    font-size: 0.7rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    margin-bottom: 0.35rem;
  }
  .kpi-val {
    font-size: 1.15rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }
  .kpi-val.ok {
    color: var(--color-success);
  }
  .kpi-val.warn {
    color: var(--color-warning);
  }
  .kpi-val.bad {
    color: var(--color-danger);
  }
  .kpi-sub {
    font-size: 0.74rem;
    color: var(--text-secondary);
    margin-top: 0.2rem;
  }

  .panel {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg, 12px);
    overflow: hidden;
  }
  .panel-pad {
    padding: 1.5rem;
  }

  .empty {
    text-align: center;
    max-width: 360px;
    margin: 0 auto;
    color: var(--text-secondary);
    padding: 2rem 1rem;
  }
  .empty h3 {
    margin: 0.75rem 0 0.35rem;
    color: var(--text-primary);
    font-size: 1.05rem;
  }
  .empty p {
    margin: 0 0 1rem;
    font-size: 0.88rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .pill-pending {
    background: color-mix(in srgb, var(--color-warning) 16%, transparent);
    color: var(--color-warning);
  }
  .pill-verify {
    background: color-mix(in srgb, var(--color-primary) 16%, transparent);
    color: var(--color-primary);
  }
  .pill-paid {
    background: color-mix(in srgb, var(--color-success) 16%, transparent);
    color: var(--color-success);
  }
  .pill-failed,
  .pill-overdue {
    background: color-mix(in srgb, var(--color-danger) 16%, transparent);
    color: var(--color-danger);
  }
  .pill-neutral {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
  }

  .amount {
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.85rem;
  }
  .due-bad {
    color: var(--color-danger);
    font-weight: 650;
  }

  .actions {
    display: flex;
    gap: 0.4rem;
    justify-content: flex-end;
    align-items: center;
    flex-wrap: wrap;
  }
  .icon-btn {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 8px;
  }
  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
  }
  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem 0.95rem;
    border-radius: 8px;
    font-weight: 650;
    font-size: 0.88rem;
    cursor: pointer;
    border: none;
    text-decoration: none;
    min-height: 40px;
  }
  .btn-sm {
    padding: 0.35rem 0.7rem;
    font-size: 0.82rem;
    min-height: 34px;
  }
  .btn-primary {
    background: var(--color-primary);
    color: #fff;
  }
  .btn-danger {
    background: var(--color-danger);
    color: #fff;
  }
  .btn-secondary {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .btn-ghost:hover:not(:disabled) {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  @media (max-width: 900px) {
    .kpis {
      grid-template-columns: 1fr 1fr;
    }
  }
  @media (max-width: 560px) {
    .page-head {
      align-items: stretch;
    }
    .head-actions {
      width: 100%;
    }
    .head-actions .btn {
      flex: 1;
      min-height: 44px;
    }
    .alert-overdue {
      align-items: stretch;
    }
    .alert-overdue .btn {
      width: 100%;
      min-height: 44px;
    }
    .kpis {
      gap: 0.55rem;
    }
    .kpi {
      padding: 0.75rem 0.85rem;
    }
    .kpi-val {
      font-size: 1.05rem;
    }
    .actions {
      justify-content: flex-start;
    }
  }
</style>
