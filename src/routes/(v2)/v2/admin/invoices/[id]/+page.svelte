<script lang="ts">
  /*
    Detail invoice v2 — gelombang 24c.

    Versi lama: (app)/admin/invoices/[id]/+page.svelte (874 baris).
    Perilaku identik: muat invoice paket (guard external_id pkgsub:),
    relasi pelanggan, cek status online, verifikasi lunas/gagal (+modal
    tolak beralasan), bukti bayar + lightbox, cetak/PDF. Label status/
    klasifikasi manual-online kini dari helper murni
    invoiceDetailInsights (3 tes).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { api, type Invoice } from '$lib/api/client';
  import type { BankAccount, Customer, CustomerSubscriptionView } from '$lib/api/types';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { can, token } from '$lib/stores/auth';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import { getAdminBillingNavigation } from '$lib/utils/adminBillingNavigation';
  import {
    buildCustomerPackageInvoiceRelationFromSubscription,
    getCustomerPackageSubscriptionId,
  } from '$lib/utils/customerPackageInvoice';
  import {
    INVOICE_REJECT_REASON_KEYS,
    invoicePaymentMethodLabel,
    invoiceStatusLabel,
    invoiceStatusTone,
    isManualPaymentInvoice,
  } from '$lib/utils/invoiceDetailInsights';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { loadLightboxModule } from '$lib/components/ui/lightboxModule';
  import InvoicePrintModal from '$lib/components/invoice/InvoicePrintModal.svelte';
  import { t } from 'svelte-i18n';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DetailHeader,
    Field,
  } from '$lib/components/ds';

  let invoice = $state<Invoice | null>(null);
  let loading = $state(true);
  let checking = $state(false);
  let processing = $state(false);
  let loadSequence = 0;
  let error = $state('');
  let showConfirm = $state(false);
  let pendingVerifyStatus = $state<'paid' | 'failed'>('paid');
  let showRejectModal = $state(false);
  let rejectReason = $state('');
  let showLightbox = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let LightboxComponent = $state<any>(null);
  let relatedCustomerId = $state<string | null>(null);

  let showPrintModal = $state(false);
  let printCustomer = $state<Customer | null>(null);
  let printSubscription = $state<CustomerSubscriptionView | null>(null);
  let printBankAccounts = $state<BankAccount[]>([]);
  let printPreparing = $state(false);

  const paymentMutationBusy = $derived(checking || processing || printPreparing);
  const canManageBilling = $derived($can('manage', 'billing'));

  const billingNav = $derived.by(() =>
    getAdminBillingNavigation({
      hostname: $page.url.hostname,
      userTenantSlug: undefined,
      tenantSlug: undefined,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const invoiceId = $derived($page.params.id ?? '');

  $effect(() => {
    if (!showLightbox) return;
    void loadLightboxModule().then(({ LightboxComponent: Lightbox }) => {
      LightboxComponent = Lightbox;
    });
  });

  onMount(() => {
    if (!$can('read', 'billing') && !canManageBilling) {
      goto('/unauthorized');
      return;
    }
    void loadInvoice();
  });

  async function loadInvoice() {
    if (!invoiceId) return;
    const seq = ++loadSequence;
    loading = true;
    error = '';
    try {
      const row = await api.payment.getInvoice(invoiceId);
      if (!String(row.external_id || '').startsWith('pkgsub:')) {
        throw new Error('Invoice ini bukan invoice layanan pelanggan.');
      }
      if (seq !== loadSequence) return;
      invoice = row;
      const relatedId = await resolveRelatedCustomerId(row);
      if (seq !== loadSequence) return;
      relatedCustomerId = relatedId;
    } catch (e) {
      if (seq !== loadSequence) return;
      error = 'Gagal memuat invoice.';
      toast.error(extractApiErrorMessage(e) || error);
      invoice = null;
      relatedCustomerId = null;
    } finally {
      if (seq === loadSequence) loading = false;
    }
  }

  async function resolveRelatedCustomerId(row: Invoice): Promise<string | null> {
    const subscriptionId = getCustomerPackageSubscriptionId(row.external_id);
    if (!subscriptionId) return null;
    try {
      const subscription = await api.customers.subscriptions.get(subscriptionId);
      const relation = buildCustomerPackageInvoiceRelationFromSubscription(row, subscription);
      return relation?.customerId || null;
    } catch {
      return null;
    }
  }

  function openCustomerDetail() {
    if (!relatedCustomerId) return;
    void goto(`/v2/admin/customers/${relatedCustomerId}`);
  }

  async function checkStatus() {
    if (!invoice || paymentMutationBusy) return;
    checking = true;
    try {
      await api.payment.checkStatus(invoice.id);
      await loadInvoice();
      toast.success($t('admin.invoices.detail.t_updated'));
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || $t('admin.invoices.detail.t_check_fail'));
    } finally {
      checking = false;
    }
  }

  function getProofUrl(fileId: string) {
    const API_BASE = getApiBaseUrl();
    const authParam = $token ? `?token=${encodeURIComponent($token)}` : '';
    return `${API_BASE}/storage/files/${fileId}/content${authParam}`;
  }

  function openProofLightbox() {
    const fileId = invoice?.proof_attachment;
    if (!fileId) {
      toast.error($t('admin.invoices.detail.t_no_proof'));
      return;
    }
    lightboxFiles = [
      {
        id: fileId,
        original_name: 'Bukti bayar',
        content_type: 'image/jpeg',
        size: 0,
        created_at: new Date().toISOString(),
      },
    ];
    showLightbox = true;
  }

  async function markPayment(status: 'paid' | 'failed', rejectionReason?: string): Promise<boolean> {
    if (!canManageBilling || !invoice || paymentMutationBusy) return false;
    processing = true;
    try {
      await api.payment.verifyCustomerPackagePayment(invoice.id, status, rejectionReason);
      await loadInvoice();
      toast.success(status === 'paid' ? $t('admin.invoices.detail.t_marked_paid') : $t('admin.invoices.detail.t_marked_failed'));
      return true;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal verifikasi.');
      return false;
    } finally {
      processing = false;
    }
  }

  function requestMarkPayment(status: 'paid' | 'failed') {
    if (!canManageBilling || !invoice || paymentMutationBusy) return;
    if (status === 'failed') {
      rejectReason = '';
      showRejectModal = true;
      return;
    }
    pendingVerifyStatus = status;
    showConfirm = true;
  }

  async function submitRejectPayment() {
    const reason = rejectReason.trim();
    if (!reason) {
      toast.error($t('admin.invoices.detail.t_reason_req'));
      return;
    }
    if (await markPayment('failed', reason)) {
      showRejectModal = false;
      rejectReason = '';
    }
  }

  async function openPrintModal() {
    if (!invoice || paymentMutationBusy) return;
    printPreparing = true;
    try {
      const subscriptionId = getCustomerPackageSubscriptionId(invoice.external_id);
      const [subRes, banksRes] = await Promise.allSettled([
        subscriptionId ? api.customers.subscriptions.get(subscriptionId) : Promise.resolve(null),
        api.payment.listBanks(),
      ]);
      const subscription = subRes.status === 'fulfilled' ? (subRes.value as CustomerSubscriptionView | null) : null;
      printSubscription = subscription;
      printBankAccounts = banksRes.status === 'fulfilled' ? (banksRes.value as BankAccount[]) : [];
      const customerId = subscription?.customer_id || relatedCustomerId;
      if (customerId) {
        try {
          printCustomer = await api.customers.get(customerId);
        } catch {
          printCustomer = null;
        }
      } else {
        printCustomer = null;
      }
      showPrintModal = true;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal siapkan cetakan.');
    } finally {
      printPreparing = false;
    }
  }
</script>
<AppShell title={invoice ? $t('admin.invoices.detail.title', { values: { n: invoice.invoice_number } }) : $t('admin.invoices.detail.title_fallback')}>
  {#if loading}
    <Card><p class="py-10 text-center text-sm text-ink-500">{ $t('admin.invoices.detail.loading') }</p></Card>
  {:else if error || !invoice}
    <Card>
      <div class="py-10 text-center">
        <p class="text-sm font-medium text-ink-900">{error || $t('admin.invoices.detail.not_found')}</p>
        <div class="mt-3">
          <Button variant="ghost" href="/v2/admin/invoices">{ $t('admin.invoices.detail.back_list') }</Button>
        </div>
      </div>
    </Card>
  {:else}
    <DetailHeader
      title={ $t('admin.invoices.detail.title', { values: { n: invoice.invoice_number } }) }
      subtitle={invoice.description || '-'}
      status={invoice.status}
      statusTone={invoiceStatusTone(invoice.status)}
      statusLabel={invoiceStatusLabel(invoice.status, (k) => $t(k))}
      backHref="/v2/admin/invoices"
      meta={[
        { label: $t('admin.invoices.detail.amount'), value: formatMoney(invoice.amount, { currency: invoice.currency_code }) },
        { label: $t('admin.invoices.detail.due'), value: formatDateTime(invoice.due_date, { timeZone: $appSettings.app_timezone }) },
        { label: $t('admin.invoices.detail.method'), value: invoicePaymentMethodLabel(invoice, (k) => $t(k)) },
      ]}
    >
      {#snippet actions()}
        {#if invoice}
        {@const inv = invoice}
        {#if relatedCustomerId}
          <Button variant="ghost" onclick={openCustomerDetail}>{ $t('admin.invoices.detail.open_customer') }</Button>
        {/if}
        <Button variant="ghost" href="/v2/admin/invoices/collection">Log penagihan</Button>
        <Button variant="ghost" icon="refresh" onclick={() => void loadInvoice()} disabled={loading || paymentMutationBusy}>
          { $t('common.refresh') }
        </Button>
        <Button variant="ghost" onclick={() => void openPrintModal()} disabled={loading || paymentMutationBusy}>
          {printPreparing ? 'Menyiapkan…' : 'Cetak / PDF'}
        </Button>
        {#if canManageBilling && inv.status === 'pending'}
          <Button variant="primary" onclick={() => window.open(`/pay/${inv.id}`, '_blank')}>
            Bayar sekarang
          </Button>
        {/if}
        {/if}
      {/snippet}
    </DetailHeader>

    <div class="grid gap-3 lg:grid-cols-2">
      <Card title={ $t('admin.invoices.details_title') }>
        <dl class="grid grid-cols-2 gap-3 text-sm">
          <div><dt class="text-xs text-ink-500">Dibuat</dt><dd class="font-medium">{invoice.created_at ? formatDateTime(invoice.created_at, { timeZone: $appSettings.app_timezone }) : '-'}</dd></div>
          <div><dt class="text-xs text-ink-500">Diperbarui</dt><dd class="font-medium">{invoice.updated_at ? formatDateTime(invoice.updated_at, { timeZone: $appSettings.app_timezone }) : '-'}</dd></div>
          {#if invoice.status === 'failed' && invoice.rejection_reason}
            <div class="col-span-2"><dt class="text-xs text-ink-500">{ $t('admin.invoices.detail.reject_reason') }</dt><dd class="font-medium text-red-700">{invoice.rejection_reason}</dd></div>
          {/if}
        </dl>
      </Card>

      {#if invoice.proof_attachment}
        <Card title={ $t('admin.invoices.detail.proof_title') }>
          <button type="button" class="block overflow-hidden rounded-xl ring-1 ring-ink-200" onclick={openProofLightbox}>
            <img src={getProofUrl(invoice.proof_attachment)} alt="Bukti bayar" class="max-h-80 w-full object-contain bg-ink-50" />
          </button>
        </Card>
      {/if}
    </div>

    <Card title={ $t('admin.invoices.detail.verify_title') }>
      <div class="flex flex-wrap gap-2">
        {#if !isManualPaymentInvoice(invoice)}
          <Button variant="ghost" onclick={() => void checkStatus()} disabled={paymentMutationBusy}>
            {checking ? $t('admin.invoices.detail.checking') : $t('admin.invoices.detail.check')}
          </Button>
        {/if}
        {#if canManageBilling && (invoice.status === 'pending' || invoice.status === 'verification_pending')}
          <Button variant="primary" onclick={() => requestMarkPayment('paid')} disabled={paymentMutationBusy}>
            {processing ? $t('admin.invoices.detail.processing') : $t('admin.invoices.detail.mark_paid')}
          </Button>
          <Button variant="danger" onclick={() => requestMarkPayment('failed')} disabled={paymentMutationBusy}>
            {processing ? $t('admin.invoices.detail.processing') : $t('admin.invoices.detail.mark_failed')}
          </Button>
        {/if}
      </div>
    </Card>
  {/if}
</AppShell>

<ConfirmDialog
  bind:show={showConfirm}
  title={pendingVerifyStatus === 'paid' ? $t('admin.invoices.detail.confirm_paid_q') : $t('admin.invoices.detail.confirm_failed_q')}
  message={pendingVerifyStatus === 'paid'
    ? $t('admin.invoices.detail.confirm_paid')
    : $t('admin.invoices.detail.confirm_failed')}
  type={pendingVerifyStatus === 'paid' ? 'info' : 'danger'}
  confirmText={pendingVerifyStatus === 'paid' ? $t('admin.invoices.detail.mark_paid') : $t('admin.invoices.detail.mark_failed')}
  cancelText={ $t('common.cancel') }
  loading={processing}
  onconfirm={() => { void (async () => { if (await markPayment(pendingVerifyStatus)) showConfirm = false; })(); }}
  oncancel={() => {}}
/>

<Modal
  bind:show={showRejectModal}
  title={ $t('admin.invoices.detail.reject_title') }
  onclose={() => (showRejectModal = false)}
>
  <p class="text-sm text-ink-500">{ $t('admin.invoices.detail.reject_hint') }</p>
  <div class="mt-3 flex flex-wrap gap-1.5">
    {#each INVOICE_REJECT_REASON_KEYS as rk}
      {@const opt = $t(rk)}
      <button
        type="button"
        class="rounded-lg px-2.5 py-1.5 text-xs ring-1 ring-ink-200 hover:bg-ink-50 {rejectReason === opt ? 'bg-ink-900 text-white' : ''}"
        onclick={() => (rejectReason = opt)}
      >
        {opt}
      </button>
    {/each}
  </div>
  <div class="mt-3">
    <Field id="inv-reject-reason" label={ $t('admin.invoices.detail.reject_reason') } type="textarea" stacked rows={3} value={rejectReason} onchange={(v) => (rejectReason = v)} />
  </div>
  <div class="mt-4 flex justify-end gap-2">
    <Button variant="ghost" onclick={() => (showRejectModal = false)}>{ $t('common.cancel') }</Button>
    <Button variant="danger" onclick={() => void submitRejectPayment()} disabled={!canManageBilling || paymentMutationBusy}>
      {processing ? $t('admin.invoices.detail.processing') : $t('admin.invoices.detail.mark_failed')}
    </Button>
  </div>
</Modal>

{#if showLightbox && LightboxComponent}
  <LightboxComponent files={lightboxFiles} onclose={() => (showLightbox = false)} />
{/if}

{#if invoice}
  <InvoicePrintModal
    bind:show={showPrintModal}
    {invoice}
    customer={printCustomer}
    subscription={printSubscription}
    bankAccounts={printBankAccounts}
  />
{/if}
