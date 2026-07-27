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
  let paymentMethod = $state<'midtrans' | 'duitku' | 'manual'>('midtrans');
  let midtransEnabled = $state(false);
  let duitkuEnabled = $state(false);
  let duitkuPaymentMethods = $state<string[]>([]);
  let selectedDuitkuPaymentMethod = $state('');
  let manualEnabled = $state(true);
  let snapToken = $state('');
  let snapReady = $state(false);
  let snapLoading = $state(false);
  let autoChecking = $state(false);
  let statusCheckTimer: ReturnType<typeof setInterval> | null = null;
  let statusCheckAttempts = 0;
  const STATUS_CHECK_INTERVAL_MS = 3000;
  const MAX_STATUS_CHECK_ATTEMPTS = 20;
  const DUITKU_PAYMENT_METHODS: Record<string, { name: string; description: string }> = {
    M2: {
      name: 'Mandiri Virtual Account',
      description: 'Bayar dari ATM, mobile banking, atau internet banking Mandiri.',
    },
    VA: {
      name: 'Maybank Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Maybank.',
    },
    I1: { name: 'BNI Virtual Account', description: 'Bayar melalui kanal Virtual Account BNI.' },
    B1: {
      name: 'CIMB Niaga Virtual Account',
      description: 'Bayar melalui kanal Virtual Account CIMB Niaga.',
    },
    BT: {
      name: 'Permata Bank Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Permata Bank.',
    },
    A1: {
      name: 'ATM Bersama',
      description: 'Bayar dari bank yang terhubung jaringan ATM Bersama.',
    },
    AG: {
      name: 'Bank Artha Graha Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Bank Artha Graha.',
    },
    NC: {
      name: 'Bank Neo Commerce Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Bank Neo Commerce.',
    },
    BR: {
      name: 'BRI Virtual Account (BRIVA)',
      description: 'Bayar melalui ATM, BRImo, atau internet banking BRI.',
    },
    S1: {
      name: 'Bank Sahabat Sampoerna Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Bank Sahabat Sampoerna.',
    },
    DM: {
      name: 'Danamon Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Danamon.',
    },
    BV: {
      name: 'Bank Victoria Virtual Account',
      description: 'Bayar melalui kanal Virtual Account Bank Victoria.',
    },
    BC: {
      name: 'BCA Virtual Account',
      description: 'Bayar melalui ATM, myBCA, BCA mobile, atau KlikBCA.',
    },
    FT: {
      name: 'Retail Outlet',
      description: 'Bayar melalui outlet ritel yang tersedia di checkout Duitku.',
    },
    IR: { name: 'Indomaret', description: 'Bayar tunai di gerai Indomaret.' },
    OV: { name: 'OVO', description: 'Bayar menggunakan saldo atau aplikasi OVO.' },
    SA: {
      name: 'ShopeePay Apps',
      description: 'Bayar menggunakan aplikasi Shopee atau ShopeePay.',
    },
    SP: {
      name: 'ShopeePay QRIS',
      description: 'Bayar dengan scan QRIS dari aplikasi yang mendukung.',
    },
    LQ: { name: 'LinkAja QRIS', description: 'Bayar dengan scan QRIS melalui LinkAja.' },
    NQ: { name: 'Nobu QRIS', description: 'Bayar dengan scan QRIS dari aplikasi yang mendukung.' },
    DA: { name: 'DANA', description: 'Bayar menggunakan saldo atau aplikasi DANA.' },
    LA: { name: 'LinkAja', description: 'Bayar menggunakan saldo atau aplikasi LinkAja.' },
    VC: {
      name: 'Kartu Kredit',
      description: 'Bayar menggunakan kartu kredit melalui checkout Duitku.',
    },
  };
  let manualInstructions = $state('');
  let publicSettings = $state<any>({});
  let returnPath = $derived($user?.role === 'admin' ? '/admin/subscription' : '/dashboard');

  onMount(async () => {
    try {
      // Load Invoice FIRST to get merchant_id for tenant context
      invoice = await api.payment.getInvoice(invoiceId);

      // Load Public Settings (filtered by invoice's tenant)
      publicSettings = await api.settings.getPublicSettings(
        invoice?.merchant_id ? { tenantId: invoice.merchant_id } : undefined,
      );

      midtransEnabled = !!publicSettings.payment_midtrans_enabled;
      duitkuEnabled = !!publicSettings.payment_duitku_enabled;
      duitkuPaymentMethods = parseDuitkuPaymentMethods(
        publicSettings.payment_duitku_payment_methods,
      );
      selectedDuitkuPaymentMethod = duitkuPaymentMethods[0] || '';
      manualEnabled = publicSettings.payment_manual_enabled ?? true; // Default true

      // Set default method
      if (midtransEnabled) paymentMethod = 'midtrans';
      else if (duitkuEnabled) paymentMethod = 'duitku';
      else if (manualEnabled) paymentMethod = 'manual';

      // If invoice currency is not IDR, disable online gateways (backend also enforces this)
      if (invoice?.currency_code && String(invoice.currency_code).toUpperCase() !== 'IDR') {
        midtransEnabled = false;
        duitkuEnabled = false;
        if (paymentMethod === 'midtrans' || paymentMethod === 'duitku') paymentMethod = 'manual';
      }

      // Load Manual Bank Accounts & Instructions (filtered by invoice's tenant)
      if (manualEnabled) {
        bankAccounts = await api.payment.listBanks(invoice?.merchant_id || undefined);
      }

      // Load Midtrans Snap JS if enabled
      if (midtransEnabled) {
        const clientKey = publicSettings.payment_midtrans_client_key;
        const isProd = !!publicSettings.payment_midtrans_is_production;
        if (clientKey) loadSnapScript(clientKey, isProd);
      }

      if (
        (midtransEnabled || duitkuEnabled) &&
        invoice?.status === 'pending' &&
        hasMidtransPending()
      ) {
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

  function parseDuitkuPaymentMethods(raw?: string | null) {
    if (!raw) return [];
    try {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.map((value) => String(value).trim().toUpperCase()).filter(Boolean);
    } catch {
      const code = String(raw).trim().toUpperCase();
      return code ? [code] : [];
    }
  }

  function getDuitkuPaymentMethodInfo(code: string) {
    const normalizedCode = String(code).trim().toUpperCase();
    return (
      DUITKU_PAYMENT_METHODS[normalizedCode] || {
        name: `Metode Pembayaran Duitku`,
        description: `Kode channel ${normalizedCode}. Detail pembayaran akan tampil di checkout Duitku.`,
      }
    );
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
    if (paymentMethod === 'duitku') {
      if (!selectedDuitkuPaymentMethod) {
        toast.error('Please select a Duitku payment method first.');
        return;
      }
      try {
        const paymentUrl = await api.payment.payDuitku(invoice.id, selectedDuitkuPaymentMethod);
        markMidtransPending();
        window.location.href = paymentUrl;
      } catch (e: any) {
        toast.error(
          (get(t)('payment.checkout.errors.initiate_failed') || 'Failed to initiate payment: ') +
            e.message,
        );
      }
      return;
    }

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
      <div class="state">{$t('payment.checkout.loading')}</div>
    {:else if invoice}
      <div class="invoice-head">
        <div class="header-top">
          <button class="back-link" onclick={() => goto(returnPath)}>
            <Icon name="arrow-left" size={16} />
            <span>{$t('common.back')}</span>
          </button>
          <span class="status-pill {invoice.status}">{invoice.status}</span>
        </div>
        <div class="header-main">
          <h1>{$t('payment.checkout.title')}</h1>
          <span class="invoice-number">#{invoice.invoice_number}</span>
        </div>
      </div>

      <div class="invoice-body">
        <section class="section">
          <h2 class="section-title">{$t('components.invoice_print.bill_to')}</h2>
          <div class="party-grid">
            <div class="party-card">
              <span class="party-k">{$t('common.from')}</span>
              <strong>{publicSettings?.app_name || 'ISP Management'}</strong>
              <span>{publicSettings?.support_email || '-'}</span>
              <span>{publicSettings?.company_phone || '-'}</span>
            </div>
            <div class="party-card">
              <span class="party-k">{$t('components.invoice_print.bill_to')}</span>
              <strong>{$user?.name || 'Customer'}</strong>
              <span>{$user?.email || '-'}</span>
              <span>{publicSettings?.tenant_name || '-'}</span>
            </div>
          </div>
        </section>

        <section class="section">
          <h2 class="section-title">Detail Invoice</h2>
          <div class="meta-grid">
            <div class="meta-item">
              <span class="k">Invoice #</span>
              <span class="v">{invoice.invoice_number}</span>
            </div>
            <div class="meta-item">
              <span class="k">{$t('payment.checkout.created')}</span>
              <span class="v">{formatDateValue(invoice.created_at)}</span>
            </div>
            <div class="meta-item">
              <span class="k">{$t('components.invoice_print.due_date')}</span>
              <span class="v">{formatDateValue(invoice.due_date)}</span>
            </div>
            <div class="meta-item">
              <span class="k">{$t('payment.checkout.status')}</span>
              <span class="v">{invoice.status}</span>
            </div>
          </div>
        </section>

        <section class="section">
          <h2 class="section-title">{$t('payment.checkout.item')}</h2>
          <div class="table-wrap">
            <table class="invoice-table">
              <thead>
                <tr>
                  <th>{$t('payment.checkout.item')}</th>
                  <th>{$t('components.invoice_print.unit_price')}</th>
                  <th>Qty</th>
                  <th>{$t('common.amount')}</th>
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
          </div>
          <div class="totals-box">
            <div><span>{$t('components.invoice_print.subtotal')}</span><strong>{formatCurrency(invoice.amount)}</strong></div>
            <div><span>Tax</span><strong>{formatCurrency(0)}</strong></div>
            <div class="grand-total">
              <span>{$t('payment.checkout.total')}</span>
              <strong>{formatCurrency(invoice.amount)}</strong>
            </div>
          </div>
        </section>

        {#if invoice.status === 'pending' || invoice.status === 'failed'}
          <section class="section">
            <h2 class="section-title">{$t('payment.checkout.payment_method')}</h2>
            <div class="method-tabs">
              {#if midtransEnabled}
                <button
                  class="method-tab {paymentMethod === 'midtrans' ? 'active' : ''}"
                  onclick={() => (paymentMethod = 'midtrans')}
                >
                  <Icon name="credit-card" size={16} />
                  Midtrans
                </button>
              {/if}
              {#if duitkuEnabled}
                <button
                  class="method-tab {paymentMethod === 'duitku' ? 'active' : ''}"
                  onclick={() => (paymentMethod = 'duitku')}
                >
                  <Icon name="wallet-cards" size={16} />
                  Duitku
                </button>
              {/if}
              {#if manualEnabled}
                <button
                  class="method-tab {paymentMethod === 'manual' ? 'active' : ''}"
                  onclick={() => (paymentMethod = 'manual')}
                >
                  <Icon name="landmark" size={16} />
                  {$t('payment.checkout.tabs.manual')}
                </button>
              {/if}
            </div>

            <div class="payment-block">
              {#if paymentMethod === 'midtrans' && midtransEnabled}
                <p class="helper">{$t('payment.checkout.online.description')}</p>
                <div class="actions">
                  <button class="btn btn-primary w-full" onclick={handlePayOnline}>
                    {$t('payment.checkout.online.pay_now')}
                  </button>
                  <button
                    class="btn btn-secondary w-full"
                    onclick={() => checkPaymentStatus({ silent: false, notifyOnChange: true })}
                    disabled={autoChecking}
                  >
                    {#if autoChecking}
                      {$t('payment.checkout.online.checking')}
                    {:else}
                      {$t('payment.checkout.online.check_status')}
                    {/if}
                  </button>
                </div>
              {:else if paymentMethod === 'duitku' && duitkuEnabled}
                <p class="helper">
                  Pilih kanal pembayaran. Setelah klik bayar, Anda akan diarahkan ke checkout aman
                  Duitku untuk menyelesaikan pembayaran.
                </p>
                {#if duitkuPaymentMethods.length > 1}
                  <div class="duitku-methods">
                    {#each duitkuPaymentMethods as method}
                      {@const methodInfo = getDuitkuPaymentMethodInfo(method)}
                      <label
                        class="duitku-method"
                        class:selected={selectedDuitkuPaymentMethod === method}
                      >
                        <input
                          type="radio"
                          name="duitku-method"
                          value={method}
                          checked={selectedDuitkuPaymentMethod === method}
                          onchange={() => (selectedDuitkuPaymentMethod = method)}
                        />
                        <span class="duitku-method-copy">
                          <span class="duitku-method-title">
                            <strong>{methodInfo.name}</strong>
                            <small>Kode {method}</small>
                          </span>
                          <span class="duitku-method-description">{methodInfo.description}</span>
                        </span>
                      </label>
                    {/each}
                  </div>
                {:else if duitkuPaymentMethods.length === 1}
                  {@const methodInfo = getDuitkuPaymentMethodInfo(selectedDuitkuPaymentMethod)}
                  <div class="selected-method">
                    <span>
                      <small>{$t('payment.checkout.payment_method')}</small>
                      <strong>{methodInfo.name}</strong>
                      <em>{methodInfo.description}</em>
                    </span>
                    <small>Kode {selectedDuitkuPaymentMethod}</small>
                  </div>
                {/if}
                <div class="actions">
                  <button class="btn btn-primary w-full" onclick={handlePayOnline}>
                    {$t('payment.checkout.online.pay_now')}
                  </button>
                  <button
                    class="btn btn-secondary w-full"
                    onclick={() => checkPaymentStatus({ silent: false, notifyOnChange: true })}
                    disabled={autoChecking}
                  >
                    {#if autoChecking}
                      {$t('payment.checkout.online.checking')}
                    {:else}
                      {$t('payment.checkout.online.check_status')}
                    {/if}
                  </button>
                </div>
              {:else if paymentMethod === 'manual' && manualEnabled}
                <p class="helper">{$t('payment.checkout.manual.instructions')}</p>

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
                  <p>{$t('payment.checkout.manual.upload_hint')}</p>
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
                      {$t('payment.checkout.manual.uploading')}
                    {:else}
                      <Icon name="upload" size={16} />
                      {$t('payment.checkout.manual.upload')}
                    {/if}
                  </button>
                </div>
              {/if}
            </div>
          </section>
        {:else if invoice.status === 'verification_pending'}
          <div class="state-card">
            <div class="icon-circle pending">
              <Icon name="clock" size={26} />
            </div>
            <h3>{$t('payment.checkout.pending.title')}</h3>
            <p>{$t('payment.checkout.pending.message')}</p>
            <button class="btn btn-secondary" onclick={() => goto(returnPath)}>
              {$t('payment.checkout.pending.back')}
            </button>
          </div>
        {:else if invoice.status === 'paid'}
          <div class="state-card">
            <div class="icon-circle success">
              <Icon name="check" size={26} />
            </div>
            <h3>{$t('payment.checkout.success.title')}</h3>
            <p>{$t('payment.checkout.success.message')}</p>
            <button class="btn btn-primary" onclick={() => goto(returnPath)}>
              {$t('payment.checkout.success.cta')}
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="state">{$t('payment.checkout.not_found')}</div>
    {/if}
  </div>
</div>
<style>
  .checkout-page {
    min-height: 100vh;
    background: var(--bg-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(1rem, 3vw, 2rem);
  }

  .invoice-shell {
    width: 100%;
    max-width: 720px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg, 12px);
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }

  .invoice-head {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    display: grid;
    gap: 0.75rem;
  }

  .header-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: var(--radius-md, 8px);
    padding: 0.4rem 0.5rem;
    font-weight: 600;
  }

  .back-link:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .header-main h1 {
    margin: 0;
    line-height: 1.2;
    font-size: clamp(1.4rem, 2.5vw, 1.75rem);
    font-weight: 800;
  }

  .invoice-number {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
    display: block;
    margin-top: 0.25rem;
  }

  .invoice-body {
    padding: 1.5rem;
    display: grid;
    gap: 1.5rem;
  }

  .section {
    display: grid;
    gap: 0.75rem;
  }

  .section-title {
    margin: 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    font-weight: 700;
  }

  .party-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .party-card {
    display: grid;
    gap: 0.25rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    padding: 0.85rem;
    background: var(--bg-secondary);
  }

  .party-k {
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
    margin-bottom: 0.25rem;
  }

  .party-card strong {
    font-size: 1rem;
  }

  .party-card span {
    color: var(--text-secondary);
    font-size: 0.86rem;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .meta-item {
    display: grid;
    gap: 0.35rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    padding: 0.75rem;
    background: var(--bg-secondary);
  }

  .meta-item .k {
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .meta-item .v {
    color: var(--text-primary);
    font-size: 0.98rem;
    font-weight: 700;
    line-height: 1.35;
  }

  .table-wrap {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    overflow: hidden;
  }

  .invoice-table {
    width: 100%;
    border-collapse: collapse;
  }

  .invoice-table th {
    text-align: left;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    padding: 0.85rem 0.9rem;
  }

  .invoice-table td {
    padding: 0.85rem 0.9rem;
    border-bottom: 1px solid var(--border-color);
    font-size: 0.95rem;
  }

  .invoice-table tr:last-child td {
    border-bottom: none;
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
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-secondary);
    padding: 1rem;
    display: grid;
    gap: 0.65rem;
  }

  .totals-box > div {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    font-size: 0.95rem;
  }

  .totals-box > div > span {
    color: var(--text-secondary);
  }

  .grand-total {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.65rem;
    font-size: 1rem;
  }

  .grand-total strong {
    font-size: 1.15rem;
  }

  .status-pill {
    padding: 0.35rem 0.85rem;
    border-radius: 999px;
    font-size: 0.75rem;
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
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .method-tab {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.55rem 1rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .method-tab.active {
    background: var(--accent-primary);
    color: #fff;
    border-color: var(--accent-primary);
  }

  .payment-block {
    display: grid;
    gap: 1rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-secondary);
    padding: 1.1rem;
  }

  .helper {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.92rem;
    line-height: 1.5;
  }

  .actions {
    display: grid;
    gap: 0.65rem;
  }

  .duitku-methods {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 0.55rem;
  }

  .duitku-method {
    display: flex;
    align-items: flex-start;
    gap: 0.55rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-surface);
    padding: 0.75rem;
    cursor: pointer;
    color: var(--text-primary);
    min-width: 0;
    transition: border-color 0.18s ease, background 0.18s ease, box-shadow 0.18s ease;
  }

  .duitku-method.selected {
    border-color: var(--accent-primary);
    background: var(--bg-surface);
    box-shadow: 0 0 0 2px var(--accent-primary);
  }

  .duitku-method input {
    flex: 0 0 auto;
    margin-top: 0.18rem;
  }

  .duitku-method-copy {
    display: grid;
    gap: 0.28rem;
    min-width: 0;
  }

  .duitku-method-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.55rem;
    min-width: 0;
  }

  .duitku-method-title strong {
    font-size: 0.94rem;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .duitku-method-title small,
  .selected-method > small {
    flex: 0 0 auto;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 700;
    line-height: 1;
    padding: 0.24rem 0.42rem;
    white-space: nowrap;
  }

  .duitku-method-description {
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .selected-method {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-surface);
    padding: 0.75rem;
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    align-items: flex-start;
  }

  .selected-method span {
    display: grid;
    gap: 0.22rem;
    min-width: 0;
  }

  .selected-method span small {
    color: var(--text-secondary);
    font-size: 0.86rem;
  }

  .selected-method strong {
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .selected-method em {
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-style: normal;
    line-height: 1.35;
  }

  .bank-list {
    display: grid;
    gap: 0.65rem;
  }

  .bank-item {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-surface);
    padding: 0.75rem;
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
    font-size: 0.84rem;
  }

  .number {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
    font-weight: 700;
    font-size: 0.92rem;
    color: var(--text-primary);
  }

  .upload-card {
    border-top: 1px dashed var(--border-color);
    padding-top: 0.9rem;
    display: grid;
    gap: 0.65rem;
  }

  .upload-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .btn {
    border: 1px solid transparent;
    border-radius: var(--radius-md, 8px);
    padding: 0.85rem 1rem;
    font-weight: 700;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    font-size: 0.95rem;
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent-primary);
    color: #fff;
  }

  .btn-secondary {
    background: var(--bg-surface);
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
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-secondary);
    padding: 1.5rem;
    display: grid;
    justify-items: center;
    text-align: center;
    gap: 0.65rem;
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
    border: 1px solid var(--color-danger);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-surface);
    padding: 0.75rem;
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
      padding: 1rem;
    }

    .header-top {
      align-items: flex-start;
      flex-direction: column;
    }

    .invoice-body {
      padding: 1rem;
      gap: 1.25rem;
    }

    .party-grid {
      grid-template-columns: 1fr;
    }

    .meta-grid {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>