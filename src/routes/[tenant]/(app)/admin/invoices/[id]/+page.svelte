<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api, type Invoice } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { user, tenant } from '$lib/stores/auth';
  import { getSlugFromDomain } from '$lib/utils/domain';

  let invoice = $state<Invoice | null>(null);
  let loading = $state(true);
  let checking = $state(false);
  let processing = $state(false);
  let error = $state('');
  let showConfirm = $state(false);
  let pendingVerifyStatus = $state<'paid' | 'failed'>('paid');

  const domainSlug = $derived(getSlugFromDomain($page.url.hostname));
  const effectiveTenantSlug = $derived(
    ($tenant?.slug || $user?.tenant_slug || $page.params.tenant || '').trim(),
  );
  const isCustomDomain = $derived(domainSlug && domainSlug === effectiveTenantSlug);
  const invoiceId = $derived($page.params.id ?? '');
  const tenantPrefix = $derived(
    effectiveTenantSlug && !isCustomDomain ? `/${effectiveTenantSlug}` : '',
  );
  const backPath = $derived(`${tenantPrefix}/admin/invoices`);

  onMount(() => {
    void loadInvoice();
  });

  async function loadInvoice() {
    if (!invoiceId) return;
    loading = true;
    error = '';
    try {
      const row = await api.payment.getInvoice(invoiceId);
      if (!String(row.external_id || '').startsWith('pkgsub:')) {
        throw new Error(
          get(t)('admin.package_invoices.detail.errors.not_customer_package') ||
            'Invoice is not a customer package invoice',
        );
      }
      invoice = row;
    } catch (e: any) {
      error = e?.message || String(e);
      toast.error(
        error || get(t)('admin.package_invoices.detail.errors.load_failed') || 'Failed to load invoice',
      );
      invoice = null;
    } finally {
      loading = false;
    }
  }

  async function checkStatus() {
    if (!invoice) return;
    checking = true;
    try {
      await api.payment.checkStatus(invoice.id);
      await loadInvoice();
      toast.success(get(t)('admin.package_invoices.detail.toasts.status_updated') || 'Status updated');
    } catch (e: any) {
      toast.error(
        e?.message ||
          get(t)('admin.package_invoices.detail.errors.check_failed') ||
          'Failed to check status',
      );
    } finally {
      checking = false;
    }
  }

  function formatCurrency(amount: number, currency?: string) {
    return formatMoney(amount, { currency });
  }

  function statusLabel(status: string) {
    const map: Record<string, string> = {
      pending: get(t)('admin.package_invoices.statuses.pending') || 'Pending',
      verification_pending:
        get(t)('admin.package_invoices.statuses.verification_pending') || 'Verification pending',
      paid: get(t)('admin.package_invoices.statuses.paid') || 'Paid',
      failed: get(t)('admin.package_invoices.statuses.failed') || 'Failed',
    };
    return map[status] || status;
  }

  async function markPayment(status: 'paid' | 'failed') {
    if (!invoice || processing) return;
    processing = true;
    try {
      await api.payment.verifyCustomerPackagePayment(invoice.id, status);
      await loadInvoice();
      toast.success(
        (get(t)('admin.package_invoices.detail.toasts.marked') || 'Invoice marked as') + ` ${status}`,
      );
    } catch (e: any) {
      toast.error(
        e?.message ||
          get(t)('admin.package_invoices.detail.errors.verify_failed') ||
          'Failed to verify invoice',
      );
    } finally {
      processing = false;
    }
  }

  function requestMarkPayment(status: 'paid' | 'failed') {
    pendingVerifyStatus = status;
    showConfirm = true;
  }

  async function confirmMarkPayment() {
    await markPayment(pendingVerifyStatus);
    showConfirm = false;
  }
</script>

<div class="page-container fade-in">
  <div class="page-header">
    <button class="back-btn" onclick={() => goto(backPath)}>
      <Icon name="arrow-left" size={18} />
      <span>{$t('admin.package_invoices.detail.back') || 'Back to Invoices'}</span>
    </button>
    <div class="header-right">
      <button class="btn btn-secondary" onclick={loadInvoice} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        <span>{$t('common.refresh') || 'Refresh'}</span>
      </button>
      {#if invoice && invoice.status === 'pending'}
        <button class="btn btn-primary" onclick={() => goto(`/pay/${invoice?.id}`)}>
          <Icon name="credit-card" size={16} />
          <span>{$t('admin.package_invoices.detail.actions.pay_now') || 'Pay Now'}</span>
        </button>
      {/if}
    </div>
  </div>

  {#if loading}
    <div class="state-card">{$t('admin.package_invoices.detail.loading') || 'Loading invoice...'}</div>
  {:else if error}
    <div class="state-card error">{error}</div>
  {:else if invoice}
    <div class="invoice-card">
      <div class="invoice-head">
        <div>
          <h1>{$t('admin.package_invoices.detail.invoice_prefix') || 'Invoice #'}{invoice.invoice_number}</h1>
          <p>{invoice.description || '-'}</p>
        </div>
        <span class="status-pill {invoice.status}">{statusLabel(invoice.status)}</span>
      </div>

      <div class="grid">
        <div class="field">
          <span>{$t('admin.package_invoices.detail.labels.amount') || 'Amount'}</span>
          <strong>{formatCurrency(invoice.amount, invoice.currency_code)}</strong>
        </div>
        <div class="field">
          <span>{$t('admin.package_invoices.detail.labels.due_date') || 'Due Date'}</span>
          <strong>{formatDateTime(invoice.due_date, { timeZone: $appSettings.app_timezone })}</strong>
        </div>
        <div class="field">
          <span>{$t('admin.package_invoices.detail.labels.created') || 'Created'}</span>
          <strong
            >{invoice.created_at
              ? formatDateTime(invoice.created_at, { timeZone: $appSettings.app_timezone })
              : '-'}</strong
          >
        </div>
        <div class="field">
          <span>{$t('admin.package_invoices.detail.labels.updated') || 'Updated'}</span>
          <strong
            >{invoice.updated_at
              ? formatDateTime(invoice.updated_at, { timeZone: $appSettings.app_timezone })
              : '-'}</strong
          >
        </div>
      </div>

      <div class="actions">
        <button class="btn btn-secondary" onclick={checkStatus} disabled={checking}>
          <Icon name="rotate-cw" size={16} />
          <span
            >{checking
              ? $t('admin.package_invoices.detail.actions.checking') || 'Checking...'
              : $t('admin.package_invoices.detail.actions.check_status') || 'Check Status'}</span
          >
        </button>
        {#if invoice.status === 'pending' || invoice.status === 'verification_pending'}
          <button class="btn btn-success" onclick={() => requestMarkPayment('paid')} disabled={processing}>
            <Icon name="check" size={16} />
            <span
              >{processing
                ? $t('admin.package_invoices.detail.actions.processing') || 'Processing...'
                : $t('admin.package_invoices.detail.actions.mark_paid') || 'Mark Paid'}</span
            >
          </button>
          <button class="btn btn-danger" onclick={() => requestMarkPayment('failed')} disabled={processing}>
            <Icon name="x" size={16} />
            <span
              >{processing
                ? $t('admin.package_invoices.detail.actions.processing') || 'Processing...'
                : $t('admin.package_invoices.detail.actions.mark_failed') || 'Mark Failed'}</span
            >
          </button>
        {/if}
        {#if invoice.status !== 'pending'}
          <button class="btn btn-secondary" onclick={() => goto(`/pay/${invoice?.id}`)}>
            <Icon name="eye" size={16} />
            <span
              >{$t('admin.package_invoices.detail.actions.open_payment_page') || 'Open Payment Page'}</span
            >
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1000px;
    margin: 0 auto;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .header-right {
    display: flex;
    gap: 0.5rem;
  }
  .back-btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0.45rem 0.75rem;
    border-radius: 8px;
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    cursor: pointer;
  }
  .state-card,
  .invoice-card {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    border-radius: 14px;
    padding: 1rem;
  }
  .state-card.error {
    color: var(--color-danger, #ef4444);
  }
  .invoice-head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }
  .invoice-head h1 {
    margin: 0 0 0.4rem;
    font-size: 1.2rem;
  }
  .invoice-head p {
    margin: 0;
    color: var(--text-secondary);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }
  .field {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.7rem;
  }
  .field span {
    display: block;
    color: var(--text-secondary);
    font-size: 0.8rem;
    margin-bottom: 0.2rem;
  }
  .field strong {
    font-size: 0.95rem;
  }
  .actions {
    margin-top: 1rem;
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .status-pill {
    padding: 0.24rem 0.58rem;
    border-radius: 999px;
    text-transform: uppercase;
    font-weight: 700;
    font-size: 0.72rem;
    border: 1px solid transparent;
  }
  .status-pill.pending,
  .status-pill.verification_pending {
    color: var(--color-warning, #f59e0b);
    background: rgba(245, 158, 11, 0.14);
    border-color: rgba(245, 158, 11, 0.22);
  }
  .status-pill.paid {
    color: var(--color-success, #10b981);
    background: rgba(16, 185, 129, 0.14);
    border-color: rgba(16, 185, 129, 0.22);
  }
  .status-pill.failed {
    color: var(--color-danger, #ef4444);
    background: rgba(239, 68, 68, 0.14);
    border-color: rgba(239, 68, 68, 0.22);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.5rem 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-primary {
    background: var(--color-primary);
    color: #fff;
  }
  .btn-secondary {
    background: var(--bg-surface);
    color: var(--text-primary);
    border-color: var(--border-color);
  }
  .btn-success {
    background: rgba(16, 185, 129, 0.16);
    color: #34d399;
    border-color: rgba(16, 185, 129, 0.28);
  }
  .btn-danger {
    background: rgba(239, 68, 68, 0.16);
    color: #f87171;
    border-color: rgba(239, 68, 68, 0.28);
  }
  @media (max-width: 720px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-right {
      justify-content: flex-start;
    }
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>

<ConfirmDialog
  bind:show={showConfirm}
  title={pendingVerifyStatus === 'paid'
    ? $t('admin.package_invoices.detail.confirm.mark_paid_title') || 'Mark Invoice as Paid'
    : $t('admin.package_invoices.detail.confirm.mark_failed_title') || 'Mark Invoice as Failed'}
  message={pendingVerifyStatus === 'paid'
    ? $t('admin.package_invoices.detail.confirm.mark_paid_message') ||
      'This will set the invoice status to paid.'
    : $t('admin.package_invoices.detail.confirm.mark_failed_message') ||
      'This will set the invoice status to failed.'}
  type={pendingVerifyStatus === 'paid' ? 'info' : 'danger'}
  confirmText={pendingVerifyStatus === 'paid'
    ? $t('admin.package_invoices.detail.confirm.mark_paid_confirm') || 'Mark Paid'
    : $t('admin.package_invoices.detail.confirm.mark_failed_confirm') || 'Mark Failed'}
  onconfirm={confirmMarkPayment}
  loading={processing}
/>
