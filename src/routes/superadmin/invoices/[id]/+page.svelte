<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { api, type Invoice } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from 'svelte-sonner';
  import { goto } from '$app/navigation';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { formatMoney } from '$lib/utils/money';
  import { formatDateTime } from '$lib/utils/date';
  import { loadLightboxModule } from '$lib/components/ui/lightboxModule';
  import { t } from 'svelte-i18n';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { getTenantsCached } from '$lib/stores/superadminTenantsCache';
  import { appSettings } from '$lib/stores/settings';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import { token } from '$lib/stores/auth';

  let invoiceId = $state('');
  let invoice = $state<Invoice | null>(null);
  let loading = $state(true);
  let processing = $state(false);
  let tenantName = $state<string | null>(null);
  let tenantSlug = $state<string | null>(null);

  // For Lightbox
  let showLightbox = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let LightboxComponent = $state<any>(null);

  // For Confirmation
  let showConfirm = $state(false);
  let confirmConfig = $state({
    title: '',
    message: '',
    type: 'info' as 'danger' | 'warning' | 'info',
    confirmText: '',
    onConfirm: async () => {},
  });

  $effect(() => {
    invoiceId = $page.params.id ?? '';
  });

  $effect(() => {
    if (!showLightbox) return;
    void loadLightboxModule().then(({ LightboxComponent: Lightbox }) => {
      LightboxComponent = Lightbox;
    });
  });

  onMount(() => {
    void loadInvoice();
  });

  async function loadInvoice() {
    if (!invoiceId) {
      invoice = null;
      loading = false;
      toast.error($t('superadmin.invoices.detail.missing_id') || 'Missing invoice id');
      return;
    }
    loading = true;
    try {
      tenantName = null;
      tenantSlug = null;

      const [inv, tenants] = await Promise.all([
        api.payment.getInvoice(invoiceId),
        getTenantsCached().catch(() => []),
      ]);
      invoice = inv;

      if (inv?.tenant_id) {
        const t = (tenants || []).find((x: any) => x.id === inv.tenant_id);
        if (t) {
          tenantName = t.name ?? null;
          tenantSlug = t.slug ?? null;
        }
      }
    } catch (e: any) {
      toast.error(
        ($t('superadmin.invoices.detail.load_failed') || 'Failed to load invoice') +
          ': ' +
          extractApiErrorMessage(e, String(e || 'unknown error')),
      );
    } finally {
      loading = false;
    }
  }

  function triggerVerify(status: 'paid' | 'failed') {
    confirmConfig = {
      title:
        status === 'paid'
          ? $t('superadmin.invoices.detail.approve_title') || 'Approve Payment'
          : $t('superadmin.invoices.detail.reject_title') || 'Reject Payment',
      message:
        status === 'paid'
          ? $t('superadmin.invoices.detail.approve_message') ||
            'Are you sure you want to approve this payment? This will activate the subscription immediately.'
          : $t('superadmin.invoices.detail.reject_message') ||
            'Are you sure you want to reject this payment? The user will be notified.',
      type: status === 'paid' ? 'info' : 'danger',
      confirmText:
        status === 'paid'
          ? $t('superadmin.invoices.detail.approve') || 'Approve'
          : $t('superadmin.invoices.detail.reject') || 'Reject',
      onConfirm: async () => await handleVerify(status),
    };
    showConfirm = true;
  }

  async function handleVerify(status: 'paid' | 'failed') {
    if (!invoiceId) return;
    processing = true;
    try {
      await api.payment.verifyPayment(invoiceId, status);
      toast.success(
        ($t('superadmin.invoices.detail.marked_as') || 'Invoice marked as') + ` ${status}`,
      );
      void loadInvoice();
      showConfirm = false;
    } catch (e: any) {
      toast.error(
        ($t('superadmin.invoices.detail.verify_failed') || 'Verification failed') +
          ': ' +
          extractApiErrorMessage(e, String(e || 'unknown error')),
      );
    } finally {
      processing = false;
    }
  }

  function formatCurrency(amount: number, currency?: string) {
    return formatMoney(amount, { currency });
  }

  function getProofUrl(fileId: string) {
    const API_BASE = getApiBaseUrl();
    const authParam = $token ? `?token=${encodeURIComponent($token)}` : '';
    return `${API_BASE}/storage/files/${fileId}/content${authParam}`;
  }

  function openLightbox(fileId: string) {
    lightboxFiles = [
      {
        id: fileId,
        original_name: $t('superadmin.invoices.detail.payment_proof') || 'Payment Proof',
        content_type: 'image/jpeg',
        size: 0,
        created_at: new Date().toISOString(),
      },
    ];
    showLightbox = true;
  }
</script>

<div class="sa-invoice-detail fade-in">
  <div class="page-head">
    <div class="page-head-copy">
      <div class="crumbs">
        <button type="button" onclick={() => goto('/superadmin')}>
          {$t('superadmin.invoices.crumbs.root') || 'Superadmin'}
        </button>
        <span aria-hidden="true">›</span>
        <button type="button" onclick={() => goto('/superadmin/invoices')}>
          {$t('superadmin.invoices.crumbs.invoices') || 'Invoices'}
        </button>
        <span aria-hidden="true">›</span>
        <b>{invoice?.invoice_number || invoiceId}</b>
      </div>
      <h1>{$t('superadmin.invoices.detail.title')}</h1>
      <p class="page-sub">{$t('superadmin.invoices.list.subtitle')}</p>
    </div>
    <button class="back-btn" type="button" onclick={() => goto('/superadmin/invoices')}>
      <Icon name="arrow-left" size={16} />
      {$t('superadmin.invoices.detail.back')}
    </button>
  </div>

  {#if loading}
    <div class="loading" role="status" aria-live="polite">
      {$t('superadmin.invoices.detail.loading')}
    </div>
  {:else if invoice}
    <div class="details-grid">
      <!-- Left: Info -->
      <div class="card info-card">
        <div class="card-header">
          <h2>
            {$t('superadmin.invoices.detail.invoice')}
            #{invoice.invoice_number}
          </h2>
          <span class="status-pill {invoice.status}">{invoice.status}</span>
        </div>

        <div class="info-rows">
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.tenant')}</span>
            <span class="value">
              {#if tenantName}
                {tenantName}
                {#if tenantSlug}
                  <span class="value-sub">{tenantSlug}</span>
                {/if}
              {:else}
                —
              {/if}
            </span>
          </div>
          <div class="row">
            <span class="label"
              >{$t('superadmin.invoices.detail.description')}</span
            >
            <span class="value">{invoice.description}</span>
          </div>
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.amount')}</span>
            <span class="value highlight"
              >{formatCurrency(invoice.amount, invoice.currency_code)}</span
            >
          </div>
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.created_at')}</span>
            <span class="value"
              >{invoice.created_at
                ? formatDateTime(invoice.created_at, { timeZone: $appSettings.app_timezone })
                : '-'}</span
            >
          </div>
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.updated_at')}</span>
            <span class="value"
              >{invoice.updated_at
                ? formatDateTime(invoice.updated_at, { timeZone: $appSettings.app_timezone })
                : '-'}</span
            >
          </div>
        </div>

        <div class="actions">
          {#if invoice.status === 'verification_pending' || invoice.status === 'pending'}
            <h3 class="section-title">
              {$t('superadmin.invoices.detail.manual_verification')}
            </h3>
            <div class="btn-group">
              <button
                class="btn btn-success"
                onclick={() => triggerVerify('paid')}
                disabled={processing}
              >
                <Icon name="check" size={18} />
                {$t('superadmin.invoices.detail.approve_title')}
              </button>
              <button
                class="btn btn-danger"
                onclick={() => triggerVerify('failed')}
                disabled={processing}
              >
                <Icon name="x" size={18} />
                {$t('superadmin.invoices.detail.reject')}
              </button>
            </div>
          {:else}
            <p class="info-text">
              {$t('superadmin.invoices.detail.already_status')}
              {invoice.status}.
            </p>
          {/if}
        </div>
      </div>

      <!-- Right: Proof Attachment -->
      <div class="card proof-card">
        <h2>
          {$t('superadmin.invoices.detail.payment_proof')}
        </h2>
        {#if invoice.proof_attachment}
          <div class="proof-wrapper">
            <button
              class="proof-button"
              type="button"
              aria-label={$t('superadmin.invoices.detail.click_enlarge')}
              onclick={() => openLightbox(invoice!.proof_attachment!)}
            >
              <img
                src={getProofUrl(invoice.proof_attachment)}
                alt={$t('superadmin.invoices.payment_proof')}
                class="proof-img"
              />
            </button>
            <p class="hint">
              {$t('superadmin.invoices.detail.click_enlarge')}
            </p>
          </div>
        {:else}
          <div class="no-proof">
            <Icon name="image" size={48} />
            <p>
              {$t('superadmin.invoices.detail.no_proof')}
            </p>
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="error">
      {$t('superadmin.invoices.detail.not_found')}
    </div>
  {/if}
</div>

<ConfirmDialog
  bind:show={showConfirm}
  title={confirmConfig.title}
  message={confirmConfig.message}
  type={confirmConfig.type}
  confirmText={confirmConfig.confirmText}
  onconfirm={confirmConfig.onConfirm}
  loading={processing}
/>

{#if showLightbox && LightboxComponent}
  <LightboxComponent files={lightboxFiles} onclose={() => (showLightbox = false)} />
{/if}

<style>
  .sa-invoice-detail {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
  }

  .page-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .page-head-copy {
    min-width: 0;
  }

  .crumbs {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
  }

  .crumbs button {
    border: 0;
    padding: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .crumbs button:hover,
  .back-btn:hover {
    color: var(--text-primary);
  }

  .crumbs b {
    color: var(--text-primary);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .page-head h1 {
    margin: 0;
    font-size: clamp(1.25rem, 2.4vw, 1.55rem);
    font-weight: 750;
    letter-spacing: -0.02em;
  }

  .page-sub {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .back-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    cursor: pointer;
    font-weight: 650;
    padding: 0.55rem 0.75rem;
    border-radius: var(--radius-md);
  }

  .details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }
  @media (max-width: 768px) {
    .details-grid {
      grid-template-columns: 1fr;
    }
  }

  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: clamp(1rem, 2.5vw, 1.5rem);
    box-shadow: var(--shadow-sm);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .status-pill {
    padding: 0.25rem 0.75rem;
    border-radius: var(--radius-lg);
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.85rem;
    border: 1px solid transparent;
  }
  .status-pill.pending {
    background: rgba(245, 158, 11, 0.14);
    color: var(--color-warning, #f59e0b);
    border-color: rgba(245, 158, 11, 0.22);
  }
  .status-pill.verification_pending {
    background: rgba(245, 158, 11, 0.14);
    color: var(--color-warning, #f59e0b);
    border-color: rgba(245, 158, 11, 0.22);
  }
  .status-pill.paid {
    background: rgba(16, 185, 129, 0.14);
    color: var(--color-success, #10b981);
    border-color: rgba(16, 185, 129, 0.22);
  }
  .status-pill.failed {
    background: rgba(239, 68, 68, 0.14);
    color: var(--color-danger, #ef4444);
    border-color: rgba(239, 68, 68, 0.22);
  }

  .info-rows {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border-color-light);
  }
  .label {
    color: var(--text-secondary);
    font-weight: 500;
  }
  .value {
    font-weight: 600;
    color: var(--text-primary);
  }
  .value-sub {
    display: inline-block;
    margin-left: 0.5rem;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 800;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.04);
  }
  .value.highlight {
    font-size: 1.1em;
    color: var(--primary-color);
  }

  .actions {
    margin-top: 2rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color);
  }
  .btn-group {
    display: flex;
    gap: 1rem;
  }
  .btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    border: none;
  }
  .btn-success {
    background: #16a34a;
    color: white;
  }
  .btn-success:hover {
    background: #15803d;
  }
  .btn-danger {
    background: #dc2626;
    color: white;
  }
  .btn-danger:hover {
    background: #b91c1c;
  }

  .proof-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;
  }
  .proof-wrapper {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .proof-img {
    max-width: 100%;
    max-height: 500px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
  }

  .proof-button {
    display: block;
    max-width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: zoom-in;
  }

  .proof-button:hover .proof-img,
  .proof-button:focus-visible .proof-img {
    transform: scale(1.02);
  }

  .proof-button:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 4px;
    border-radius: var(--radius-md);
  }
  .hint {
    margin-top: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .no-proof {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    color: var(--text-tertiary);
  }

  @media (max-width: 768px) {
    .page-head {
      align-items: flex-start;
      flex-direction: column;
    }

    .back-btn {
      width: 100%;
      justify-content: center;
    }

    .row {
      align-items: flex-start;
      flex-direction: column;
      gap: 0.25rem;
    }

    .btn-group {
      flex-direction: column;
    }
  }
</style>
