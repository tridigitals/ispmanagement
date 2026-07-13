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

  let summary = $derived.by(() => {
    const invs = invoices;
    return {
      total: invs.length,
      pending: invs.filter(i => i.status === 'pending').length,
      paid: invs.filter(i => i.status === 'paid').length,
      overdue: invs.filter(i => {
        if (i.status !== 'pending') return false;
        if (!i.due_date) return false;
        return new Date(i.due_date) < new Date();
      }).length,
      pendingTotal: invs.filter(i => i.status === 'pending').reduce((sum, i) => sum + (Number(i.amount) || 0), 0),
    };
  });

  // ---- printable invoice state -------------------------------------------
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

  async function openPrintModal(item: Invoice) {
    if (printPreparing) return;
    printPreparing = true;
    try {
      printInvoice = item;

      // Customer-side: build a Customer record from the active session user.
      // The portal API doesn't expose a `getMe` for the customer entity,
      // so we use the auth store to seed the bill-to block.
      const u = get(user) as any;
      printCustomer = u
        ? {
            id: u.id,
            tenant_id: u.tenant_id || '',
            name: u.name || u.email || 'Customer',
            email: u.email || null,
            phone: u.phone || null,
            notes: null,
            is_active: true,
            created_at: '',
            updated_at: '',
          }
        : null;

      // Bank accounts are tenant-scoped public-ish data; cache on first fetch.
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

<div class="page-container fade-in">
  <section class="hero-card invoice-hero">
    <div class="welcome-body">
      <h1>{$t('admin.invoices.title')}</h1>
      <p class="welcome-sub">{$t('admin.invoices.subtitle')}</p>
    </div>
    <button class="btn btn-secondary btn-sm" onclick={loadInvoices}>
      <Icon name="refresh-cw" size={14} />
      <span>{$t('common.refresh')}</span>
    </button>
  </section>

  <!-- Summary Strip -->
  {#if !loading && !error}
    <div class="bento-grid">
      <div class="bento-card">
        <div class="bento-icon" style="background:color-mix(in srgb, var(--color-primary) 18%, transparent);color:var(--color-primary)">
          <Icon name="file-text" size={18} />
        </div>
        <span class="bento-value">{summary.total}</span>
        <span class="bento-label">Total Invoice</span>
      </div>
      <div class="bento-card">
        <div class="bento-icon" style="background:color-mix(in srgb, var(--color-success) 18%, transparent);color:var(--color-success)">
          <Icon name="check-circle" size={18} />
        </div>
        <span class="bento-value" style="background:linear-gradient(135deg,#7dd3ae,#34d399);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text">{summary.paid}</span>
        <span class="bento-label">Lunas</span>
      </div>
      <div class="bento-card">
        <div class="bento-icon" style="background:color-mix(in srgb, var(--color-warning) 18%, transparent);color:var(--color-warning)">
          <Icon name="clock" size={18} />
        </div>
        <span class="bento-value" style="background:linear-gradient(135deg,#fbbf24,#f59e0b);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text">{summary.pending}</span>
        <span class="bento-label">Menunggu ({formatCurrency(summary.pendingTotal)})</span>
      </div>
      <div class="bento-card">
        <div class="bento-icon" style="background:color-mix(in srgb, var(--color-danger) 18%, transparent);color:var(--color-danger)">
          <Icon name="alert-triangle" size={18} />
        </div>
        <span class="bento-value" style="background:linear-gradient(135deg,#fca5a5,#ef4444);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text">{summary.overdue}</span>
        <span class="bento-label">Jatuh Tempo</span>
      </div>
    </div>
  {/if}

  <div class="card content-card">
    {#if error}
      <div class="alert alert-error">{error}</div>
    {/if}

    <Table
      {loading}
      data={invoices}
      {columns}
      searchable={true}
      searchPlaceholder={$t('admin.invoices.search_placeholder')}
    >
      {#snippet cell({ item, column })}
        {#if column.key === 'amount'}
          {formatCurrency(item.amount, item.currency_code)}
        {:else if column.key === 'status'}
          <span class="status-pill {item.status}">{item.status}</span>
        {:else if column.key === 'due_date'}
          {formatDate(item[column.key], { timeZone: $appSettings.app_timezone })}
        {:else if column.key === 'actions'}
          <div class="actions">
            {#if item.status === 'pending'}
              <button
                type="button"
                class="btn btn-primary btn-sm"
                onclick={() => goto(`/pay/${item.id}`)}
              >
                <Icon name="credit-card" size={14} />
                {$t('admin.invoices.pay_now')}
              </button>
            {:else}
              <button
                type="button"
                class="action-btn"
                title={$t('admin.invoices.view_details')}
                aria-label={$t('admin.invoices.view_details')}
                onclick={() => goto(`/pay/${item.id}`)}
              >
                <Icon name="eye" size={18} />
              </button>
            {/if}
            <button
              type="button"
              class="action-btn"
              title={$t('admin.invoices.print_pdf')}
              aria-label={$t('admin.invoices.print_pdf')}
              onclick={() => openPrintModal(item)}
              disabled={printPreparing}
            >
              <Icon name="printer" size={18} />
            </button>
          </div>
        {:else}
          {item[column.key]}
        {/if}
      {/snippet}
    </Table>
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
  .page-container {
    padding: clamp(1rem, 2.2vw, 2rem);
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .invoice-hero {
    padding: 1.5rem 1.75rem;
    display: flex; align-items: center; justify-content: space-between;
  }
  .invoice-hero h1 {
    font-size: clamp(1.4rem, 2.2vw, 1.85rem);
    font-weight: 750; color: var(--text-primary);
    margin: 0 0 .35rem;
  }
  .invoice-hero .welcome-sub {
    color: var(--text-secondary); margin: 0; font-size: .92rem;
  }
  .content-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  .status-pill {
    padding: 0.25rem 0.6rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
  }
  .status-pill.pending {
    background: #fef3c7;
    color: #d97706;
  }
  .status-pill.paid {
    background: #dcfce7;
    color: #16a34a;
  }
  .status-pill.failed {
    background: #fee2e2;
    color: #dc2626;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    align-items: center;
  }
  .action-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 6px;
  }
  .action-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1rem;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    border: none;
    text-decoration: none;
  }
  .btn-sm {
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
  }
  .btn-primary {
    background: var(--color-primary);
    color: white;
  }
  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }

    .btn.btn-secondary {
      width: 100%;
      justify-content: center;
    }

    .header-content h1 {
      font-size: 1.35rem;
    }

    .content-card {
      border-radius: var(--radius-lg);
    }

    .actions {
      justify-content: flex-start;
    }
  }
</style>
