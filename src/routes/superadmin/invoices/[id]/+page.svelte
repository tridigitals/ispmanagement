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
  import Lightbox from '$lib/components/ui/Lightbox.svelte';
  import { t } from 'svelte-i18n';
  import { getTenantsCached } from '$lib/stores/superadminTenantsCache';
  import { appSettings } from '$lib/stores/settings';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';

  let invoiceId = $state('');
  let invoice = $state<Invoice | null>(null);
  let loading = $state(true);
  let processing = $state(false);
  let tenantName = $state<string | null>(null);
  let tenantSlug = $state<string | null>(null);

  // For Lightbox
  let showLightbox = $state(false);
  let lightboxFiles = $state<any[]>([]);

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
          (e.message || String(e)),
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
          (e.message || String(e)),
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
    return `${API_BASE}/storage/files/${fileId}/content`;
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

<div class="page-container fade-in">
  <div class="page-header">
    <button class="back-btn" onclick={() => goto('/superadmin/invoices')}>
      <Icon name="arrow-left" size={20} />
      {$t('superadmin.invoices.detail.back') || 'Back to Invoices'}
    </button>
    <h1>{$t('superadmin.invoices.detail.title') || 'Invoice Details'}</h1>
  </div>

  {#if loading}
    <div class="loading">
      {$t('superadmin.invoices.detail.loading') || 'Loading details...'}
    </div>
  {:else if invoice}
    <div class="details-grid">
      <!-- Left: Info -->
      <div class="card info-card">
        <div class="card-header">
          <h2>
            {$t('superadmin.invoices.detail.invoice') || 'Invoice'}
            #{invoice.invoice_number}
          </h2>
          <span class="status-pill {invoice.status}">{invoice.status}</span>
        </div>

        <div class="info-rows">
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.tenant') || 'Tenant'}</span>
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
              >{$t('superadmin.invoices.detail.description') || 'Description'}</span
            >
            <span class="value">{invoice.description}</span>
          </div>
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.amount') || 'Amount'}</span>
            <span class="value highlight"
              >{formatCurrency(invoice.amount, invoice.currency_code)}</span
            >
          </div>
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.created_at') || 'Created At'}</span>
            <span class="value"
              >{invoice.created_at
                ? formatDateTime(invoice.created_at, { timeZone: $appSettings.app_timezone })
                : '-'}</span
            >
          </div>
          <div class="row">
            <span class="label">{$t('superadmin.invoices.detail.updated_at') || 'Updated At'}</span>
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
              {$t('superadmin.invoices.detail.manual_verification') || 'Manual Verification'}
            </h3>
            <div class="btn-group">
              <button
                class="btn btn-success"
                onclick={() => triggerVerify('paid')}
                disabled={processing}
              >
                <Icon name="check" size={18} />
                {$t('superadmin.invoices.detail.approve_title') || 'Approve Payment'}
              </button>
              <button
                class="btn btn-danger"
                onclick={() => triggerVerify('failed')}
                disabled={processing}
              >
                <Icon name="x" size={18} />
                {$t('superadmin.invoices.detail.reject') || 'Reject'}
              </button>
            </div>
          {:else}
            <p class="info-text">
              {$t('superadmin.invoices.detail.already_status') || 'This invoice is already'}
              {invoice.status}.
            </p>
          {/if}
        </div>
      </div>

      <!-- Right: Proof Attachment -->
      <div class="card proof-card">
        <h2>
          {$t('superadmin.invoices.detail.payment_proof') || 'Payment Proof'}
        </h2>
        {#if invoice.proof_attachment}
          <div class="proof-wrapper">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <img
              src={getProofUrl(invoice.proof_attachment)}
              alt="Payment Proof"
              class="proof-img"
              onclick={() => openLightbox(invoice!.proof_attachment!)}
            />
            <p class="hint">
              {$t('superadmin.invoices.detail.click_enlarge') || 'Click to enlarge'}
            </p>
          </div>
        {:else}
          <div class="no-proof">
            <Icon name="image" size={48} />
            <p>
              {$t('superadmin.invoices.detail.no_proof') || 'No proof uploaded yet.'}
            </p>
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="error">
      {$t('superadmin.invoices.detail.not_found') || 'Invoice not found'}
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

{#if showLightbox}
  <Lightbox files={lightboxFiles} onclose={() => (showLightbox = false)} />
{/if}

<style>
  .page-container {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
  }
  .page-header {
    margin-bottom: 2rem;
  }
  .back-btn {
    background: none;
    border: none;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    cursor: pointer;
    font-weight: 500;
    margin-bottom: 0.5rem;
  }
  .back-btn:hover {
    color: var(--text-primary);
  }

  .details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
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
    padding: 1.5rem;
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
    border-radius: 20px;
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
    border-radius: 8px;
    border: 1px solid var(--border-color);
    cursor: zoom-in;
    transition: transform 0.2s;
  }
  .proof-img:hover {
    transform: scale(1.02);
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
</style>
