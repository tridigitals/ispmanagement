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
  import Lightbox from '$lib/components/ui/Lightbox.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { can, user, tenant, token } from '$lib/stores/auth';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';

  let invoice = $state<Invoice | null>(null);
  let loading = $state(true);
  let checking = $state(false);
  let processing = $state(false);
  let error = $state('');
  let showConfirm = $state(false);
  let pendingVerifyStatus = $state<'paid' | 'failed'>('paid');
  let showRejectModal = $state(false);
  let rejectReason = $state('');
  const rejectReasonOptions = $derived.by(() => [
    get(t)('admin.package_invoices.detail.reject.options.blurry') || 'Proof image is unclear',
    get(t)('admin.package_invoices.detail.reject.options.amount_mismatch') ||
      'Transfer amount does not match invoice total',
    get(t)('admin.package_invoices.detail.reject.options.wrong_destination') ||
      'Transfer destination account is incorrect',
    get(t)('admin.package_invoices.detail.reject.options.invalid_proof') ||
      'Invalid or unrelated payment proof',
    get(t)('admin.package_invoices.detail.reject.options.duplicate') ||
      'Duplicate proof already used',
  ]);
  let showLightbox = $state(false);
  let lightboxFiles = $state<any[]>([]);

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const invoiceId = $derived($page.params.id ?? '');
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const backPath = $derived(`${tenantPrefix}/admin/invoices`);
  const billingLogsPath = $derived(`${tenantPrefix}/admin/invoices/collection`);

  onMount(() => {
    if (!$can('read', 'billing') && !$can('manage', 'billing')) {
      goto('/unauthorized');
      return;
    }
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

  function isManualPaymentInvoice(row: Invoice | null): boolean {
    if (!row) return false;
    const method = String(row.payment_method || '').toLowerCase();
    return (
      row.status === 'verification_pending' ||
      !!row.proof_attachment ||
      method.includes('bank') ||
      method.includes('manual')
    );
  }

  function isOnlinePaymentInvoice(row: Invoice | null): boolean {
    if (!row) return false;
    return !isManualPaymentInvoice(row);
  }

  function paymentMethodLabel(row: Invoice | null): string {
    if (!row) return '-';
    if (isManualPaymentInvoice(row)) {
      return get(t)('admin.package_invoices.detail.payment_methods.bank_transfer') || 'Bank Transfer';
    }

    const method = String(row.payment_method || '').toLowerCase();
    if (method.includes('midtrans')) {
      return get(t)('admin.package_invoices.detail.payment_methods.online_payment') || 'Online Payment';
    }
    if (!method) {
      return get(t)('admin.package_invoices.detail.payment_methods.online_payment') || 'Online Payment';
    }
    return row.payment_method || '-';
  }

  function getProofUrl(fileId: string) {
    const API_BASE = getApiBaseUrl();
    const authParam = $token ? `?token=${encodeURIComponent($token)}` : '';
    return `${API_BASE}/storage/files/${fileId}/content${authParam}`;
  }

  function openProofLightbox() {
    const fileId = invoice?.proof_attachment;
    if (!fileId) {
      toast.error(
        get(t)('admin.package_invoices.detail.errors.proof_not_available') ||
          'Payment proof is not available yet',
      );
      return;
    }

    lightboxFiles = [
      {
        id: fileId,
        original_name:
          get(t)('admin.package_invoices.detail.payment_proof.title') || 'Payment Proof',
        content_type: 'image/jpeg',
        size: 0,
        created_at: new Date().toISOString(),
      },
    ];
    showLightbox = true;
  }

  async function markPayment(status: 'paid' | 'failed', rejectionReason?: string) {
    if (!invoice || processing) return;
    processing = true;
    try {
      await api.payment.verifyCustomerPackagePayment(invoice.id, status, rejectionReason);
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
    if (status === 'failed') {
      rejectReason = '';
      showRejectModal = true;
      return;
    }
    pendingVerifyStatus = status;
    showConfirm = true;
  }

  async function confirmMarkPayment() {
    await markPayment(pendingVerifyStatus);
    showConfirm = false;
  }

  async function submitRejectPayment() {
    const reason = rejectReason.trim();
    if (!reason) {
      toast.error(
        get(t)('admin.package_invoices.detail.reject.reason_required') || 'Rejection reason is required',
      );
      return;
    }
    await markPayment('failed', reason);
    if (!processing) {
      showRejectModal = false;
      rejectReason = '';
    }
  }
</script>

<div class="page-container fade-in">
  <nav class="breadcrumb" aria-label="Breadcrumb">
    <button class="crumb-link" type="button" onclick={() => goto(backPath)}>
      {$t('sidebar.invoices') || 'Invoices'}
    </button>
    <span class="crumb-sep">/</span>
    <span class="crumb-current">{$t('common.details') || 'Details'}</span>
    <span class="crumb-sep">/</span>
    <button class="crumb-link" type="button" onclick={() => goto(billingLogsPath)}>
      {$t('admin.package_invoices.list.actions.billing_logs') || 'Billing Logs'}
    </button>
  </nav>

  <div class="page-header">
    <button class="back-btn" onclick={() => goto(backPath)}>
      <Icon name="arrow-left" size={18} />
      <span>{$t('admin.package_invoices.detail.back') || 'Back to Invoices'}</span>
    </button>
    <div class="header-right">
      <button class="btn btn-secondary" onclick={() => goto(billingLogsPath)}>
        <Icon name="activity" size={16} />
        <span>{$t('admin.package_invoices.list.actions.billing_logs') || 'Billing Logs'}</span>
      </button>
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
        <div class="field">
          <span>{$t('admin.package_invoices.detail.labels.payment_method') || 'Payment Method'}</span>
          <strong>{paymentMethodLabel(invoice)}</strong>
        </div>
        {#if invoice.status === 'failed' && invoice.rejection_reason}
          <div class="field field-wide">
            <span>{$t('admin.package_invoices.detail.labels.rejection_reason') || 'Rejection Reason'}</span>
            <strong>{invoice.rejection_reason}</strong>
          </div>
        {/if}
      </div>

      {#if invoice.proof_attachment}
        <div class="proof-section">
          <div class="proof-head">
            <h2>{$t('admin.package_invoices.detail.payment_proof.title') || 'Payment Proof'}</h2>
            <span class="proof-hint"
              >{$t('admin.package_invoices.detail.payment_proof.hint') || 'Click image to enlarge'}</span
            >
          </div>
          <button class="proof-image-button" type="button" onclick={openProofLightbox}>
            <img
              src={getProofUrl(invoice.proof_attachment)}
              alt={$t('admin.package_invoices.detail.payment_proof.title') || 'Payment Proof'}
              class="proof-image"
            />
          </button>
        </div>
      {/if}

      <div class="actions">
        {#if isOnlinePaymentInvoice(invoice)}
          <button class="btn btn-secondary" onclick={checkStatus} disabled={checking}>
            <Icon name="rotate-cw" size={16} />
            <span
              >{checking
                ? $t('admin.package_invoices.detail.actions.checking') || 'Checking...'
                : $t('admin.package_invoices.detail.actions.check_status') || 'Check Status'}</span
            >
          </button>
        {/if}
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
      </div>
    </div>
  {/if}
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1360px;
    margin: 0 auto;
  }
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    flex-wrap: wrap;
  }
  .crumb-link {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    padding: 0;
    cursor: pointer;
  }
  .crumb-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
  }
  .crumb-current {
    color: var(--text-primary);
    font-weight: 600;
  }
  .crumb-sep {
    color: var(--text-tertiary);
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.1rem;
    padding-bottom: 0.9rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border-color) 85%, transparent 15%);
  }
  .header-right {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
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
    padding: 1.15rem;
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
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }
  .field {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.7rem;
  }
  .field-wide {
    grid-column: 1 / -1;
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
  .proof-section {
    margin-top: 1rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.8rem;
    background: color-mix(in srgb, var(--bg-surface) 92%, transparent 8%);
  }
  .proof-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    margin-bottom: 0.55rem;
  }
  .proof-head h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
  }
  .proof-hint {
    color: var(--text-secondary);
    font-size: 0.8rem;
  }
  .proof-image-button {
    border: 0;
    background: transparent;
    padding: 0;
    margin: 0;
    display: block;
    width: 100%;
    cursor: zoom-in;
  }
  .proof-image {
    width: 100%;
    max-height: 360px;
    object-fit: cover;
    border-radius: 10px;
    border: 1px solid var(--border-color);
  }
  .reject-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
    background: rgba(2, 6, 23, 0.62);
    display: grid;
    place-items: center;
    padding: 1rem;
  }
  .reject-modal {
    width: min(560px, 100%);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.42);
    padding: 1rem;
  }
  .reject-modal h3 {
    margin: 0 0 0.35rem;
    font-size: 1.05rem;
  }
  .reject-modal p {
    margin: 0 0 0.8rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }
  .reject-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 0.6rem;
  }
  .reject-chip {
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.32rem 0.62rem;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .reject-reason-input {
    width: 100%;
    resize: vertical;
    min-height: 92px;
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.62rem 0.7rem;
    margin-bottom: 0.7rem;
  }
  .reject-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.45rem;
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
  @media (max-width: 1200px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
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

{#if showLightbox}
  <Lightbox files={lightboxFiles} onclose={() => (showLightbox = false)} />
{/if}

{#if showRejectModal}
  <div class="reject-modal-backdrop">
    <div class="reject-modal">
      <h3>{$t('admin.package_invoices.detail.reject.title') || 'Mark Invoice as Failed'}</h3>
      <p>
        {$t('admin.package_invoices.detail.reject.description') ||
          'Provide a clear reason so customer can fix and re-upload payment proof.'}
      </p>
      <div class="reject-presets">
        {#each rejectReasonOptions as opt}
          <button class="reject-chip" type="button" onclick={() => (rejectReason = opt)}>
            {opt}
          </button>
        {/each}
      </div>
      <textarea
        class="reject-reason-input"
        bind:value={rejectReason}
        placeholder={$t('admin.package_invoices.detail.reject.placeholder') || 'Write rejection reason...'}
      ></textarea>
      <div class="reject-actions">
        <button class="btn btn-secondary" type="button" onclick={() => (showRejectModal = false)}>
          {$t('common.cancel') || 'Cancel'}
        </button>
        <button class="btn btn-danger" type="button" onclick={submitRejectPayment} disabled={processing}>
          {#if processing}
            {$t('admin.package_invoices.detail.actions.processing') || 'Processing...'}
          {:else}
            {$t('admin.package_invoices.detail.actions.mark_failed') || 'Mark Failed'}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}
