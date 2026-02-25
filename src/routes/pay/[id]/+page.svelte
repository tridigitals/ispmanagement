<script lang="ts">
  import { page } from '$app/stores';
  import { onDestroy, onMount } from 'svelte';
  import { api, type Invoice, type BankAccount } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from 'svelte-sonner';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  let invoiceId = $page.params.id as string;
  let invoice = $state<Invoice | null>(null);
  let bankAccounts = $state<BankAccount[]>([]);
  let loading = $state(true);
  let paymentMethod = $state<'online' | 'manual'>('online');
  let midtransEnabled = $state(false);
  let manualEnabled = $state(true);
  let snapToken = $state('');
  let snapReady = $state(false);
  let snapLoading = $state(false);
  let autoChecking = $state(false);
  let statusCheckTimer: ReturnType<typeof setInterval> | null = null;
  let statusCheckAttempts = 0;
  const STATUS_CHECK_INTERVAL_MS = 3000;
  const MAX_STATUS_CHECK_ATTEMPTS = 20;
  let manualInstructions = $state('');
  let publicSettings = $state<any>({});
  let returnPath = $derived($user?.role === 'admin' ? '/admin/subscription' : '/dashboard');

  onMount(async () => {
    try {
      // Load Public Settings (contains payment config)
      publicSettings = await api.settings.getPublicSettings();

      midtransEnabled = !!publicSettings.payment_midtrans_enabled;
      manualEnabled = publicSettings.payment_manual_enabled ?? true; // Default true

      // Set default method
      if (midtransEnabled) paymentMethod = 'online';
      else if (manualEnabled) paymentMethod = 'manual';

      // Load Invoice
      invoice = await api.payment.getInvoice(invoiceId);

      // If invoice currency is not IDR, disable Midtrans (backend also enforces this)
      if (invoice?.currency_code && String(invoice.currency_code).toUpperCase() !== 'IDR') {
        midtransEnabled = false;
        if (paymentMethod === 'online') paymentMethod = 'manual';
      }

      // Load Manual Bank Accounts & Instructions
      if (manualEnabled) {
        bankAccounts = await api.payment.listBanks();
      }

      // Load Midtrans Snap JS if enabled
      if (midtransEnabled) {
        const clientKey = publicSettings.payment_midtrans_client_key;
        const isProd = !!publicSettings.payment_midtrans_is_production;
        if (clientKey) loadSnapScript(clientKey, isProd);
      }

      if (midtransEnabled && invoice?.status === 'pending' && hasMidtransPending()) {
        startStatusPolling();
      }
    } catch (e: any) {
      toast.error(
        e.message || get(t)('payment.checkout.errors.load_failed') || 'Failed to load invoice',
      );
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    stopStatusPolling();
  });

  function pendingKey() {
    return `midtrans:pending:${invoiceId}`;
  }

  function markMidtransPending() {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(pendingKey(), '1');
  }

  function clearMidtransPending() {
    if (typeof localStorage === 'undefined') return;
    localStorage.removeItem(pendingKey());
  }

  function hasMidtransPending() {
    if (typeof localStorage === 'undefined') return false;
    return localStorage.getItem(pendingKey()) === '1';
  }

  function loadSnapScript(clientKey: string, isProd: boolean) {
    if (typeof window !== 'undefined' && (window as any).snap) {
      snapReady = true;
      return;
    }

    const existing = document.getElementById('midtrans-snap-js') as HTMLScriptElement | null;
    if (existing) {
      existing.addEventListener('load', () => {
        snapReady = true;
        snapLoading = false;
      });
      return;
    }

    snapLoading = true;
    const script = document.createElement('script');
    script.id = 'midtrans-snap-js';
    script.src = isProd
      ? 'https://app.midtrans.com/snap/snap.js'
      : 'https://app.sandbox.midtrans.com/snap/snap.js';
    script.setAttribute('data-client-key', clientKey);
    script.onload = () => {
      snapReady = true;
      snapLoading = false;
    };
    script.onerror = () => {
      snapLoading = false;
      toast.error(
        get(t)('payment.checkout.errors.load_failed') || 'Failed to load Midtrans payment script',
      );
    };
    document.head.appendChild(script);
  }

  async function handlePayOnline() {
    if (!invoice) return;
    try {
      const token = await api.payment.payMidtrans(invoice.id);
      snapToken = token;

      const snap = (window as any).snap;
      if (!snap) {
        toast.error(
          get(t)('payment.checkout.errors.load_failed') ||
            'Midtrans is not ready yet. Please try again.',
        );
        return;
      }

      markMidtransPending();
      snap.pay(token, {
        onSuccess: function (result: any) {
          toast.success(get(t)('payment.checkout.toasts.payment_success') || 'Payment successful!');
          startStatusPolling();
        },
        onPending: function (result: any) {
          toast.info(get(t)('payment.checkout.toasts.waiting') || 'Waiting for payment...');
          startStatusPolling();
        },
        onError: function (result: any) {
          toast.error(get(t)('payment.checkout.toasts.payment_failed') || 'Payment failed');
          clearMidtransPending();
          stopStatusPolling();
        },
        onClose: function () {
          // closed
        },
      });
    } catch (e: any) {
      toast.error(
        (get(t)('payment.checkout.errors.initiate_failed') || 'Failed to initiate payment: ') +
          e.message,
      );
    }
  }

  async function checkPaymentStatus(options?: { silent?: boolean; notifyOnChange?: boolean }) {
    if (!invoice) return;
    try {
      const status = await api.payment.checkStatus(invoice.id);
      const notifyOnChange = options?.notifyOnChange ?? true;
      const silent = options?.silent ?? false;

      if (status !== invoice.status) {
        invoice = { ...invoice, status };
        if (status === 'paid' || status === 'failed') {
          clearMidtransPending();
          stopStatusPolling();
          invoice = await api.payment.getInvoice(invoice.id);
        }
        if (notifyOnChange && !silent) {
          toast.success(
            (get(t)('payment.checkout.toasts.status_updated') || 'Status updated: ') + status,
          );
        } else if (notifyOnChange && status === 'paid') {
          toast.success(get(t)('payment.checkout.toasts.payment_success') || 'Payment successful!');
        } else if (notifyOnChange && status === 'failed') {
          toast.error(get(t)('payment.checkout.toasts.payment_failed') || 'Payment failed');
        }
      } else if (!silent) {
        toast.info(
          (get(t)('payment.checkout.toasts.current_status') || 'Current status: ') + status,
        );
      }

      return status;
    } catch (e: any) {
      if (!options?.silent) {
        toast.error(
          (get(t)('payment.checkout.errors.check_status_failed') || 'Failed to check status: ') +
            e.message,
        );
      }
    }
  }

  function startStatusPolling() {
    if (!invoice || invoice.status !== 'pending') return;
    if (statusCheckTimer) return;
    autoChecking = true;
    statusCheckAttempts = 0;

    const poll = async () => {
      statusCheckAttempts += 1;
      const status = await checkPaymentStatus({
        silent: true,
        notifyOnChange: true,
      });
      if (status && status !== 'pending') {
        stopStatusPolling();
        return;
      }
      if (statusCheckAttempts >= MAX_STATUS_CHECK_ATTEMPTS) {
        stopStatusPolling();
      }
    };

    void poll();
    statusCheckTimer = setInterval(poll, STATUS_CHECK_INTERVAL_MS);
  }

  function stopStatusPolling() {
    if (statusCheckTimer) {
      clearInterval(statusCheckTimer);
      statusCheckTimer = null;
    }
    autoChecking = false;
  }

  function formatCurrency(amount: number) {
    const locale = publicSettings?.default_locale || 'id-ID';
    const currency = invoice?.currency_code || publicSettings?.currency_code || 'IDR';

    return new Intl.NumberFormat(locale, {
      style: 'currency',
      currency,
    }).format(amount);
  }

  function formatDateValue(value?: string | null) {
    if (!value) return '-';
    const d = new Date(value);
    if (Number.isNaN(d.getTime())) return '-';
    return new Intl.DateTimeFormat(publicSettings?.default_locale || 'id-ID', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    }).format(d);
  }
  let fileInput = $state<HTMLInputElement | null>(null);
  let uploading = $state(false);

  async function handleFileUpload(e: Event) {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;

    if (file.size > 5 * 1024 * 1024) {
      toast.error(
        get(t)('payment.checkout.errors.file_too_large') || 'File size must be less than 5MB',
      );
      return;
    }

    uploading = true;
    try {
      // 1. Upload file to storage
      // Note: This requires the user to be logged in, which they should be for subscription payment.
      // If this is a public invoice link for non-logged users, we'd need a public upload endpoint.
      // Assuming logged in for now as per `submit_payment_proof` requirement.

      const uploadedFile = await api.storage.uploadFile(file, {
        paymentInvoiceId: invoice!.id,
      });

      // 2. Submit proof path/url
      // We'll store the URL or ID. Let's store the URL for easy display.
      // Assuming `uploadedFile.url` or we construct it.
      // `FileRecord` has `url` usually? Let's check `client.ts` interface.
      // If not, we store `uploadedFile.id` and fetch via ID?
      // Actually `uploadFile` returns `FileRecord`.

      // Let's assume we can get a serving URL.
      // For local storage, it might be served via specific route.
      // For now, let's store the file ID or name.
      // Ideally, we store the full accessible URL.
      // Let's verify `FileRecord` interface in `client.ts`.

      // Temporary: Just store the ID or Name if URL isn't explicit in `FileRecord`
      // But `submit_payment_proof` takes string.

      await api.payment.submitPaymentProof(invoice!.id, uploadedFile.id); // Storing ID for security/lookup

      toast.success(
        get(t)('payment.checkout.toasts.proof_uploaded') || 'Proof uploaded successfully!',
      );
      // Reload to show pending state
      location.reload();
    } catch (e: any) {
      toast.error(
        (get(t)('payment.checkout.errors.upload_failed') || 'Upload failed: ') + e.message,
      );
    } finally {
      uploading = false;
    }
  }
</script>

<div class="checkout-page fade-in">
  <div class="invoice-shell">
    {#if loading}
      <div class="state">{$t('payment.checkout.loading') || 'Loading invoice...'}</div>
    {:else if invoice}
      <div class="invoice-head">
        <button
          class="back-link"
          onclick={() => goto(returnPath)}
        >
          <Icon name="arrow-left" size={16} />
          <span>{$t('common.back') || 'Back'}</span>
        </button>
        <div class="head-title">
          <h1>{$t('payment.checkout.title') || 'Checkout'}</h1>
          <span class="invoice-number">#{invoice.invoice_number}</span>
        </div>
        <div class="head-right">
          <span class="doc-mark">INVOICE</span>
          <span class="status-pill {invoice.status}">{invoice.status}</span>
        </div>
      </div>

      <div class="invoice-body">
        <div class="party-grid">
          <div class="party-card">
            <span class="party-k">From</span>
            <strong>{publicSettings?.app_name || 'ISP Management'}</strong>
            <span>{publicSettings?.support_email || '-'}</span>
            <span>{publicSettings?.company_phone || '-'}</span>
          </div>
          <div class="party-card">
            <span class="party-k">Bill to</span>
            <strong>{$user?.name || 'Customer'}</strong>
            <span>{$user?.email || '-'}</span>
            <span>{publicSettings?.tenant_name || '-'}</span>
          </div>
        </div>

        <div class="meta-grid">
          <div class="meta-item">
            <span class="k">Invoice #</span>
            <span class="v">{invoice.invoice_number}</span>
          </div>
          <div class="meta-item">
            <span class="k">Created</span>
            <span class="v">{formatDateValue(invoice.created_at)}</span>
          </div>
          <div class="meta-item">
            <span class="k">Due date</span>
            <span class="v">{formatDateValue(invoice.due_date)}</span>
          </div>
          <div class="meta-item">
            <span class="k">{$t('payment.checkout.status') || 'Status'}</span>
            <span class="status-pill {invoice.status}">{invoice.status}</span>
          </div>
        </div>

        <div class="line-items">
          <table class="invoice-table">
            <thead>
              <tr>
                <th>{$t('payment.checkout.item') || 'Item'}</th>
                <th>Unit Price</th>
                <th>Qty</th>
                <th>Amount</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>{invoice.description || '-'}</td>
                <td>{formatCurrency(invoice.amount)}</td>
                <td>1</td>
                <td>{formatCurrency(invoice.amount)}</td>
              </tr>
            </tbody>
          </table>
          <div class="totals-box">
            <div><span>Subtotal</span><strong>{formatCurrency(invoice.amount)}</strong></div>
            <div><span>Tax</span><strong>{formatCurrency(0)}</strong></div>
            <div class="grand-total">
              <span>{$t('payment.checkout.total') || 'Total'}</span>
              <strong>{formatCurrency(invoice.amount)}</strong>
            </div>
          </div>
        </div>

        {#if invoice.status === 'pending'}
          <div class="method-tabs">
            {#if midtransEnabled}
              <button
                class="method-tab {paymentMethod === 'online' ? 'active' : ''}"
                onclick={() => (paymentMethod = 'online')}
              >
                <Icon name="credit-card" size={16} />
                {$t('payment.checkout.tabs.online') || 'Online Payment'}
              </button>
            {/if}
            {#if manualEnabled}
              <button
                class="method-tab {paymentMethod === 'manual' ? 'active' : ''}"
                onclick={() => (paymentMethod = 'manual')}
              >
                <Icon name="landmark" size={16} />
                {$t('payment.checkout.tabs.manual') || 'Bank Transfer'}
              </button>
            {/if}
          </div>

          <div class="payment-block">
            {#if paymentMethod === 'online' && midtransEnabled}
              <p class="helper">
                {$t('payment.checkout.online.description') ||
                  'Pay securely with Credit Card, GoPay, ShopeePay, or Virtual Account via Midtrans.'}
              </p>
              <button class="btn btn-primary w-full" onclick={handlePayOnline}>
                {$t('payment.checkout.online.pay_now') || 'Pay Now'}
              </button>
              <button
                class="btn btn-secondary w-full"
                onclick={() =>
                  checkPaymentStatus({
                    silent: false,
                    notifyOnChange: true,
                  })}
                disabled={autoChecking}
              >
                {#if autoChecking}
                  {$t('payment.checkout.online.checking') || 'Checking...'}
                {:else}
                  {$t('payment.checkout.online.check_status') || 'Check Payment Status'}
                {/if}
              </button>
            {:else if paymentMethod === 'manual' && manualEnabled}
              <p class="helper">
                {$t('payment.checkout.manual.instructions') ||
                  'Please transfer the exact amount to one of the following accounts:'}
              </p>

              <div class="bank-list">
                {#each bankAccounts as bank}
                  <div class="bank-item">
                    <div class="bank-left">
                      <strong>{bank.bank_name}</strong>
                      <span class="holder">{bank.account_holder}</span>
                    </div>
                    <span class="number">{bank.account_number}</span>
                  </div>
                {/each}
              </div>

              <div class="upload-card">
                <p>
                  {$t('payment.checkout.manual.upload_hint') ||
                    'Already transferred? Upload your receipt.'}
                </p>
                <input
                  type="file"
                  accept="image/*,application/pdf"
                  onchange={handleFileUpload}
                  style="display: none;"
                  bind:this={fileInput}
                />
                <button
                  class="btn btn-secondary w-full"
                  onclick={() => fileInput?.click()}
                  disabled={uploading}
                >
                  {#if uploading}
                    {$t('payment.checkout.manual.uploading') || 'Uploading...'}
                  {:else}
                    <Icon name="upload" size={16} />
                    {$t('payment.checkout.manual.upload') || 'Upload Proof of Payment'}
                  {/if}
                </button>
              </div>
            {/if}
          </div>
        {:else if invoice.status === 'verification_pending'}
          <div class="state-card">
            <div class="icon-circle pending">
              <Icon name="clock" size={26} />
            </div>
            <h3>{$t('payment.checkout.pending.title') || 'Payment Verification Pending'}</h3>
            <p>
              {$t('payment.checkout.pending.message') ||
                'We have received your payment proof. Our team is verifying it. We will notify you once approved.'}
            </p>
            <button
              class="btn btn-secondary"
              onclick={() => goto(returnPath)}
            >
              {$t('payment.checkout.pending.back') || 'Return to Dashboard'}
            </button>
          </div>
        {:else if invoice.status === 'paid'}
          <div class="state-card">
            <div class="icon-circle success">
              <Icon name="check" size={26} />
            </div>
            <h3>{$t('payment.checkout.success.title') || 'Payment Successful!'}</h3>
            <p>
              {$t('payment.checkout.success.message') ||
                'Thank you for your payment. Your subscription has been activated.'}
            </p>
            <button
              class="btn btn-primary"
              onclick={() => goto(returnPath)}
            >
              {$t('payment.checkout.success.cta') || 'Go to Subscription'}
            </button>
          </div>
        {:else if invoice.status === 'failed'}
          <div class="state-card">
            <div class="icon-circle failed">
              <Icon name="circle-alert" size={26} />
            </div>
            <h3>{$t('payment.checkout.failed.title') || 'Payment Verification Failed'}</h3>
            <p>
              {$t('payment.checkout.failed.message') ||
                'Your payment proof was rejected. Please check the reason below and upload a new proof.'}
            </p>
            {#if invoice.rejection_reason}
              <div class="failed-reason">
                <span>{$t('payment.checkout.failed.reason_label') || 'Reason'}</span>
                <strong>{invoice.rejection_reason}</strong>
              </div>
            {/if}
            <div class="upload-card">
              <p>
                {$t('payment.checkout.failed.reupload_hint') ||
                  'After re-transfering, upload your new payment proof.'}
              </p>
              <input
                type="file"
                accept="image/*,application/pdf"
                onchange={handleFileUpload}
                style="display: none;"
                bind:this={fileInput}
              />
              <button
                class="btn btn-secondary w-full"
                onclick={() => fileInput?.click()}
                disabled={uploading}
              >
                {#if uploading}
                  {$t('payment.checkout.manual.uploading') || 'Uploading...'}
                {:else}
                  <Icon name="upload" size={16} />
                  {$t('payment.checkout.failed.upload_again') || 'Upload Proof Again'}
                {/if}
              </button>
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="state">{$t('payment.checkout.not_found') || 'Invoice not found'}</div>
    {/if}
  </div>
</div>

<style>
  .checkout-page {
    min-height: 100vh;
    background:
      radial-gradient(
        1200px 600px at 10% -10%,
        color-mix(in srgb, var(--accent-primary) 14%, transparent),
        transparent
      ),
      radial-gradient(
        900px 500px at 110% 110%,
        color-mix(in srgb, var(--accent-primary) 10%, transparent),
        transparent
      ),
      var(--bg-app);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(0.75rem, 2vw, 1.5rem);
  }

  .invoice-shell {
    width: 100%;
    max-width: 820px;
    border: 1px solid color-mix(in srgb, var(--border-color) 88%, #ffffff12);
    border-radius: 22px;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--bg-surface) 95%, #131827 5%) 0%,
        color-mix(in srgb, var(--bg-surface) 92%, #0f1320 8%) 100%
      );
    box-shadow:
      0 26px 70px rgba(0, 0, 0, 0.45),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
    overflow: hidden;
    position: relative;
  }

  .invoice-shell::before {
    content: '';
    position: absolute;
    inset: 0;
    background:
      radial-gradient(
        600px 220px at 85% -15%,
        color-mix(in srgb, var(--accent-primary) 14%, transparent),
        transparent
      ),
      linear-gradient(transparent, transparent);
    pointer-events: none;
  }

  .invoice-head {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 1rem;
    padding: 1.15rem 1.2rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border-color) 82%, transparent);
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--bg-tertiary) 88%, transparent),
      color-mix(in srgb, var(--bg-tertiary) 70%, transparent)
    );
    position: relative;
    z-index: 1;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 10px;
    padding: 0.4rem 0.5rem;
  }

  .back-link:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .head-title h1 {
    margin: 0;
    line-height: 1.2;
    font-size: clamp(1.28rem, 2.2vw, 1.64rem);
    letter-spacing: -0.01em;
  }

  .invoice-number {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .head-right {
    display: grid;
    justify-items: end;
    gap: 0.35rem;
  }

  .doc-mark {
    font-size: 0.7rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--accent-primary) 70%, var(--text-secondary));
    font-weight: 700;
  }

  .invoice-body {
    padding: 1.05rem 1.2rem 1.3rem;
    display: grid;
    gap: 1.1rem;
    position: relative;
    z-index: 1;
  }

  .party-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.9rem;
  }

  .party-card {
    display: grid;
    gap: 0.2rem;
    border: 1px dashed color-mix(in srgb, var(--border-color) 88%, #ffffff12);
    border-radius: 12px;
    padding: 0.7rem 0.8rem;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--bg-secondary) 58%, transparent),
      color-mix(in srgb, var(--bg-secondary) 44%, transparent)
    );
  }

  .party-k {
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .party-card strong {
    font-size: 1rem;
  }

  .party-card span {
    color: var(--text-secondary);
    font-size: 0.84rem;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.9rem;
    border-bottom: 1px dashed color-mix(in srgb, var(--border-color) 85%, #ffffff12);
    padding-bottom: 0.9rem;
  }

  .meta-item {
    display: grid;
    gap: 0.35rem;
    border: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
    border-radius: 10px;
    padding: 0.58rem 0.64rem;
    background: color-mix(in srgb, var(--bg-secondary) 38%, transparent);
  }

  .meta-item .k {
    color: var(--text-secondary);
    font-size: 0.76rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .meta-item .v {
    color: var(--text-primary);
    font-size: 1rem;
    line-height: 1.35;
  }

  .line-items {
    display: grid;
    gap: 0.82rem;
  }

  .invoice-table {
    width: 100%;
    border-collapse: collapse;
    overflow: hidden;
    border-radius: 12px;
    border: 1px solid var(--border-color);
  }

  .invoice-table th {
    text-align: left;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: color-mix(in srgb, #ffffff 88%, var(--text-secondary));
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--accent-primary) 78%, #22d3ee 22%),
      color-mix(in srgb, var(--accent-primary) 55%, #0891b2 45%)
    );
    padding: 0.72rem 0.75rem;
  }

  .invoice-table td {
    padding: 0.72rem 0.75rem;
    border-top: 1px solid var(--border-color);
    font-size: 0.92rem;
  }

  .invoice-table th:nth-child(3),
  .invoice-table td:nth-child(3) {
    width: 60px;
    text-align: center;
  }

  .invoice-table th:nth-child(2),
  .invoice-table td:nth-child(2),
  .invoice-table th:nth-child(4),
  .invoice-table td:nth-child(4) {
    width: 160px;
    text-align: right;
  }

  .totals-box {
    margin-left: auto;
    min-width: min(100%, 340px);
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, #ffffff12);
    border-radius: 12px;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--bg-secondary) 58%, transparent),
      color-mix(in srgb, var(--bg-secondary) 45%, transparent)
    );
    padding: 0.72rem 0.8rem;
    display: grid;
    gap: 0.5rem;
  }

  .totals-box > div {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    font-size: 0.92rem;
  }

  .totals-box > div > span {
    color: var(--text-secondary);
  }

  .grand-total {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.5rem;
    font-size: 1rem;
  }

  .grand-total strong {
    font-size: 1.2rem;
  }

  .status-pill {
    padding: 0.28rem 0.7rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .status-pill.pending,
  .status-pill.verification_pending {
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

  .method-tabs {
    display: inline-flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .method-tab {
    border: 1px solid color-mix(in srgb, var(--border-color) 86%, #ffffff12);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.45rem 0.8rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.86rem;
    font-weight: 600;
  }

  .method-tab.active {
    color: var(--accent-primary);
    border-color: color-mix(in srgb, var(--accent-primary) 55%, var(--border-color));
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
  }

  .payment-block {
    display: grid;
    gap: 0.85rem;
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, #ffffff0f);
    border-radius: 14px;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--bg-secondary) 68%, transparent),
      color-mix(in srgb, var(--bg-secondary) 58%, transparent)
    );
    padding: 0.95rem;
  }

  .helper {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.45;
  }

  .bank-list {
    display: grid;
    gap: 0.65rem;
  }

  .bank-item {
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, #ffffff10);
    border-radius: 10px;
    background: var(--bg-surface);
    padding: 0.7rem 0.75rem;
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    align-items: center;
  }

  .bank-left {
    display: grid;
    gap: 0.15rem;
  }

  .holder {
    color: var(--text-secondary);
    font-size: 0.83rem;
  }

  .number {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    font-weight: 700;
    font-size: 0.92rem;
    color: var(--text-primary);
  }

  .upload-card {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.8rem;
    display: grid;
    gap: 0.65rem;
  }

  .upload-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .btn {
    border: 1px solid transparent;
    border-radius: 10px;
    padding: 0.66rem 0.95rem;
    font-weight: 700;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .btn-primary {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--color-primary) 85%, #22d3ee 15%),
      var(--color-primary)
    );
    color: #fff;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-color: var(--border-color);
  }

  .w-full {
    width: 100%;
  }

  .state {
    padding: 2rem 1.25rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .state-card {
    border: 1px solid color-mix(in srgb, var(--border-color) 82%, #ffffff10);
    border-radius: 14px;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--bg-secondary) 62%, transparent),
      color-mix(in srgb, var(--bg-secondary) 50%, transparent)
    );
    padding: 1.4rem;
    display: grid;
    justify-items: center;
    text-align: center;
    gap: 0.55rem;
  }

  .state-card h3 {
    margin: 0.1rem 0 0;
  }

  .state-card p {
    margin: 0;
    color: var(--text-secondary);
    max-width: 46ch;
    line-height: 1.45;
  }

  .icon-circle {
    width: 58px;
    height: 58px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-circle.success {
    background: #dcfce7;
    color: #16a34a;
  }

  .icon-circle.pending {
    background: #fef3c7;
    color: #d97706;
  }

  .icon-circle.failed {
    background: #fee2e2;
    color: #dc2626;
  }

  .failed-reason {
    width: min(100%, 46ch);
    border: 1px solid color-mix(in srgb, var(--color-danger) 28%, var(--border-color));
    border-radius: 12px;
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
    padding: 0.7rem 0.75rem;
    margin-top: 0.2rem;
  }

  .failed-reason span {
    display: block;
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-bottom: 0.2rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .failed-reason strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  @media (max-width: 760px) {
    .invoice-head {
      grid-template-columns: 1fr;
      justify-items: flex-start;
      gap: 0.45rem;
    }

    .head-right {
      justify-items: flex-start;
    }

    .party-grid {
      grid-template-columns: 1fr;
    }

    .meta-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
