<script lang="ts">
  import { appSettings } from '$lib/stores/settings';
  import { tenant } from '$lib/stores/auth';
  import { formatMoney } from '$lib/utils/money';
  import type { Invoice, BankAccount, Customer, CustomerSubscriptionView } from '$lib/api/types';

  type LineItem = {
    description: string;
    qty?: number;
    unit_price?: number;
    amount: number;
  };

  let {
    invoice,
    customer = null,
    subscription = null,
    bankAccounts = [],
    lineItems = null,
    notes = '',
    /**
     * If true, renders without surrounding card/shadow so it can be embedded
     * into the print viewport cleanly. The component always exposes a stable
     * `.invoice-doc` root for print CSS targeting.
     */
    embed = false,
  } = $props<{
    invoice: Invoice;
    customer?: Customer | null;
    subscription?: CustomerSubscriptionView | null;
    bankAccounts?: BankAccount[];
    lineItems?: LineItem[] | null;
    notes?: string;
    embed?: boolean;
  }>();

  // ---- derived data ---------------------------------------------------------

  const tenantName = $derived(
    $tenant?.name ||
      ($appSettings as any).organization_name ||
      $appSettings.app_name ||
      'ISP Management',
  );
  const tenantLogo = $derived($tenant?.logo_url || ($appSettings as any).company_logo || null);
  const tenantAddress = $derived(($appSettings as any).company_address || '');
  const tenantPhone = $derived(($appSettings as any).company_phone || '');
  const tenantEmail = $derived(
    ($appSettings as any).company_email || ($appSettings as any).support_email || '',
  );
  const tenantNpwp = $derived(($appSettings as any).company_npwp || '');

  const invoiceFooterNote = $derived(
    notes ||
      ($appSettings as any).invoice_footer_note ||
      'Terima kasih atas kepercayaan Anda. Silakan hubungi kami bila ada pertanyaan tentang tagihan ini.',
  );

  const currency = $derived(invoice?.currency_code || ($appSettings as any).currency_code || 'IDR');

  // ---- line item resolution -------------------------------------------------

  const items = $derived.by<LineItem[]>(() => {
    if (lineItems && lineItems.length > 0) return lineItems;
    const desc =
      invoice?.description ||
      (subscription?.package_name
        ? `Subscription · ${subscription.package_name}`
        : 'Service Charge');
    return [
      {
        description: desc,
        qty: 1,
        unit_price: Number(invoice?.amount || 0),
        amount: Number(invoice?.amount || 0),
      },
    ];
  });

  const subtotal = $derived(items.reduce((acc, it) => acc + Number(it.amount || 0), 0));
  const total = $derived(Number(invoice?.amount ?? subtotal));
  // If a tax has been pre-applied at the invoice level (future), the difference
  // between subtotal sum and invoice.amount surfaces as adjustment.
  const adjustment = $derived(Number((total - subtotal).toFixed(2)));

  // ---- formatting helpers ---------------------------------------------------

  function fmtMoney(n: number): string {
    return formatMoney(Number(n || 0), { currency });
  }

  function fmtDate(value?: string | null): string {
    if (!value) return '-';
    try {
      const d = new Date(value);
      return d.toLocaleDateString($appSettings.default_locale || 'en-US', {
        year: 'numeric',
        month: 'short',
        day: '2-digit',
        timeZone: $appSettings.app_timezone,
      });
    } catch {
      return value;
    }
  }

  function statusLabel(s: string): string {
    const map: Record<string, string> = {
      pending: 'PENDING',
      verification_pending: 'VERIFICATION PENDING',
      paid: 'PAID',
      failed: 'FAILED / REJECTED',
      cancelled: 'CANCELLED',
    };
    return map[s] || s.toUpperCase();
  }

  // amount in words is intentionally omitted to avoid locale ambiguity for IDR.
</script>

<article class="invoice-doc" class:embed aria-label="Invoice document">
  <!-- Header --------------------------------------------------------------- -->
  <header class="doc-head">
    <div class="brand">
      {#if tenantLogo}
        <img src={tenantLogo} alt={tenantName} class="brand-logo" />
      {:else}
        <div class="brand-mark">{tenantName.charAt(0).toUpperCase()}</div>
      {/if}
      <div class="brand-meta">
        <h2 class="brand-name">{tenantName}</h2>
        {#if tenantAddress}<p class="brand-line">{tenantAddress}</p>{/if}
        {#if tenantPhone || tenantEmail}
          <p class="brand-line">
            {#if tenantPhone}{tenantPhone}{/if}
            {#if tenantPhone && tenantEmail} · {/if}
            {#if tenantEmail}{tenantEmail}{/if}
          </p>
        {/if}
        {#if tenantNpwp}<p class="brand-line">NPWP: {tenantNpwp}</p>{/if}
      </div>
    </div>

    <div class="doc-title">
      <h1>INVOICE</h1>
      <p class="doc-number">#{invoice?.invoice_number || invoice?.id}</p>
      <span class="status-badge" data-status={invoice?.status}>
        {statusLabel(String(invoice?.status || ''))}
      </span>
    </div>
  </header>

  <!-- Parties + dates ------------------------------------------------------ -->
  <section class="doc-parties">
    <div class="party">
      <span class="party-label">Bill To</span>
      <p class="party-name">{customer?.name || subscription?.location_label || 'Customer'}</p>
      {#if customer?.email}<p class="party-line">{customer.email}</p>{/if}
      {#if customer?.phone}<p class="party-line">{customer.phone}</p>{/if}
      {#if subscription?.location_label}
        <p class="party-line muted">Lokasi: {subscription.location_label}</p>
      {/if}
    </div>

    <div class="party party-meta">
      <div class="meta-row">
        <span>Issue Date</span>
        <strong>{fmtDate(invoice?.created_at)}</strong>
      </div>
      <div class="meta-row">
        <span>Due Date</span>
        <strong>{fmtDate(invoice?.due_date)}</strong>
      </div>
      {#if invoice?.paid_at}
        <div class="meta-row">
          <span>Paid At</span>
          <strong>{fmtDate(invoice.paid_at)}</strong>
        </div>
      {/if}
      {#if subscription?.billing_cycle}
        <div class="meta-row">
          <span>Billing Cycle</span>
          <strong>{subscription.billing_cycle}</strong>
        </div>
      {/if}
    </div>
  </section>

  <!-- Items ---------------------------------------------------------------- -->
  <section class="doc-items">
    <table class="item-table">
      <thead>
        <tr>
          <th class="col-desc">Description</th>
          <th class="col-qty">Qty</th>
          <th class="col-price">Unit Price</th>
          <th class="col-total">Amount</th>
        </tr>
      </thead>
      <tbody>
        {#each items as item}
          <tr>
            <td class="col-desc">{item.description}</td>
            <td class="col-qty">{item.qty ?? 1}</td>
            <td class="col-price">{fmtMoney(item.unit_price ?? item.amount)}</td>
            <td class="col-total">{fmtMoney(item.amount)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  <!-- Totals --------------------------------------------------------------- -->
  <section class="doc-totals">
    <div class="totals-spacer"></div>
    <div class="totals-block">
      <div class="totals-row">
        <span>Subtotal</span>
        <strong>{fmtMoney(subtotal)}</strong>
      </div>
      {#if adjustment !== 0}
        <div class="totals-row">
          <span>{adjustment > 0 ? 'Tax / Adjustment' : 'Discount'}</span>
          <strong>{fmtMoney(Math.abs(adjustment))}</strong>
        </div>
      {/if}
      <div class="totals-row totals-grand">
        <span>Total Due</span>
        <strong>{fmtMoney(total)}</strong>
      </div>
      {#if invoice?.fx_rate && invoice?.base_currency_code && invoice.base_currency_code !== currency}
        <p class="fx-note">
          FX 1 {currency} = {invoice.fx_rate} {invoice.base_currency_code}
        </p>
      {/if}
    </div>
  </section>

  <!-- Payment instructions ------------------------------------------------- -->
  {#if bankAccounts.length > 0 && invoice?.status !== 'paid'}
    <section class="doc-payment">
      <h3>Payment Instructions</h3>
      <p class="payment-hint">
        Lakukan transfer ke salah satu rekening berikut, lalu unggah bukti pembayaran melalui
        portal pelanggan.
      </p>
      <div class="bank-grid">
        {#each bankAccounts as bank}
          <div class="bank-card">
            <div class="bank-name">{bank.bank_name}</div>
            <div class="bank-account">{bank.account_number}</div>
            <div class="bank-holder">a.n. {bank.account_holder}</div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Footer / notes ------------------------------------------------------- -->
  <footer class="doc-foot">
    {#if invoice?.rejection_reason && invoice.status === 'failed'}
      <div class="reject-note">
        <strong>Rejection reason:</strong>
        {invoice.rejection_reason}
      </div>
    {/if}
    <p class="foot-note">{invoiceFooterNote}</p>
    <p class="foot-meta">
      Invoice {invoice?.invoice_number} · Generated by {tenantName}
    </p>
  </footer>
</article>

<style>
  /* Document root - A4 portrait, 210mm × 297mm. We render at fixed widths so
     screen preview matches the printed result. Padding leaves 14mm safe edges
     which most browsers honor without scaling. */
  .invoice-doc {
    --ink: #0f172a;
    --ink-soft: #475569;
    --ink-muted: #94a3b8;
    --line: #e2e8f0;
    --line-strong: #cbd5e1;
    --accent: #1e3a8a;
    --bg: #ffffff;
    --bg-soft: #f8fafc;

    box-sizing: border-box;
    width: 210mm;
    min-height: 297mm;
    margin: 0 auto;
    padding: 14mm 14mm 18mm;
    background: var(--bg);
    color: var(--ink);
    font-family:
      'Inter',
      ui-sans-serif,
      system-ui,
      -apple-system,
      'Segoe UI',
      sans-serif;
    font-size: 10.5pt;
    line-height: 1.45;
    box-shadow: 0 8px 30px rgba(15, 23, 42, 0.12);
    border-radius: 4px;
  }
  .invoice-doc.embed {
    box-shadow: none;
    border-radius: 0;
    margin: 0;
  }

  /* Header */
  .doc-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1.5rem;
    margin-bottom: 1.25rem;
    padding-bottom: 0.9rem;
    border-bottom: 2px solid var(--accent);
  }
  .brand {
    display: flex;
    gap: 0.85rem;
    align-items: flex-start;
    max-width: 60%;
  }
  .brand-logo {
    width: 56px;
    height: 56px;
    object-fit: contain;
    border-radius: 8px;
  }
  .brand-mark {
    width: 56px;
    height: 56px;
    border-radius: 8px;
    background: var(--accent);
    color: #fff;
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 22pt;
  }
  .brand-name {
    margin: 0 0 0.15rem;
    font-size: 13pt;
    font-weight: 700;
    color: var(--ink);
    letter-spacing: -0.01em;
  }
  .brand-line {
    margin: 0;
    color: var(--ink-soft);
    font-size: 9pt;
    line-height: 1.4;
  }
  .doc-title {
    text-align: right;
    flex-shrink: 0;
  }
  .doc-title h1 {
    margin: 0;
    font-size: 22pt;
    letter-spacing: 0.06em;
    color: var(--accent);
    font-weight: 800;
  }
  .doc-number {
    margin: 0.1rem 0 0.4rem;
    color: var(--ink-soft);
    font-size: 10pt;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
  }
  .status-badge {
    display: inline-block;
    padding: 0.18rem 0.55rem;
    font-size: 8pt;
    font-weight: 700;
    letter-spacing: 0.05em;
    border-radius: 4px;
    border: 1px solid currentColor;
    color: #92400e;
    background: #fef3c7;
  }
  .status-badge[data-status='paid'] {
    color: #065f46;
    background: #d1fae5;
  }
  .status-badge[data-status='failed'],
  .status-badge[data-status='cancelled'] {
    color: #991b1b;
    background: #fee2e2;
  }

  /* Parties */
  .doc-parties {
    display: grid;
    grid-template-columns: 1.2fr 1fr;
    gap: 1rem;
    margin-bottom: 1.1rem;
  }
  .party {
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 0.7rem 0.85rem;
    background: var(--bg-soft);
  }
  .party-label {
    display: block;
    font-size: 8pt;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--ink-muted);
    margin-bottom: 0.25rem;
  }
  .party-name {
    margin: 0 0 0.15rem;
    font-weight: 700;
    font-size: 11pt;
  }
  .party-line {
    margin: 0;
    color: var(--ink-soft);
    font-size: 9.5pt;
  }
  .party-line.muted {
    color: var(--ink-muted);
  }
  .party-meta {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .meta-row {
    display: flex;
    justify-content: space-between;
    gap: 0.6rem;
    font-size: 9.5pt;
  }
  .meta-row span {
    color: var(--ink-muted);
  }
  .meta-row strong {
    color: var(--ink);
  }

  /* Items table */
  .doc-items {
    margin-bottom: 0.6rem;
  }
  .item-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 10pt;
  }
  .item-table thead th {
    text-align: left;
    padding: 0.55rem 0.7rem;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
    font-size: 9pt;
    letter-spacing: 0.04em;
  }
  .item-table tbody td {
    padding: 0.6rem 0.7rem;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
  }
  .item-table tbody tr:last-child td {
    border-bottom: 1px solid var(--line-strong);
  }
  .col-qty,
  .col-price,
  .col-total {
    text-align: right;
    white-space: nowrap;
  }
  .col-desc {
    width: 56%;
  }

  /* Totals */
  .doc-totals {
    display: grid;
    grid-template-columns: 1fr auto;
    margin-bottom: 1.2rem;
  }
  .totals-block {
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.6rem 0.8rem;
  }
  .totals-row {
    display: flex;
    justify-content: space-between;
    gap: 1.5rem;
    font-size: 10pt;
  }
  .totals-row span {
    color: var(--ink-soft);
  }
  .totals-grand {
    border-top: 1px solid var(--line-strong);
    padding-top: 0.45rem;
    margin-top: 0.2rem;
    font-size: 11.5pt;
  }
  .totals-grand strong {
    color: var(--accent);
    font-size: 13pt;
  }
  .fx-note {
    margin: 0.3rem 0 0;
    font-size: 8pt;
    color: var(--ink-muted);
    text-align: right;
  }

  /* Payment */
  .doc-payment {
    margin-bottom: 1rem;
    padding: 0.8rem 0.9rem;
    background: var(--bg-soft);
    border: 1px dashed var(--line-strong);
    border-radius: 6px;
  }
  .doc-payment h3 {
    margin: 0 0 0.3rem;
    font-size: 10.5pt;
    color: var(--accent);
    font-weight: 700;
  }
  .payment-hint {
    margin: 0 0 0.55rem;
    font-size: 9pt;
    color: var(--ink-soft);
  }
  .bank-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 0.55rem;
  }
  .bank-card {
    border: 1px solid var(--line);
    background: #fff;
    border-radius: 4px;
    padding: 0.5rem 0.65rem;
    font-size: 9pt;
  }
  .bank-name {
    color: var(--ink-muted);
    font-size: 8pt;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 0.15rem;
  }
  .bank-account {
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    font-weight: 700;
    font-size: 11pt;
    color: var(--ink);
    margin-bottom: 0.15rem;
  }
  .bank-holder {
    color: var(--ink-soft);
    font-size: 8.5pt;
  }

  /* Footer */
  .doc-foot {
    margin-top: 1rem;
    padding-top: 0.7rem;
    border-top: 1px solid var(--line);
    font-size: 9pt;
    color: var(--ink-soft);
  }
  .reject-note {
    margin-bottom: 0.55rem;
    padding: 0.5rem 0.65rem;
    background: #fee2e2;
    border-radius: 4px;
    color: #991b1b;
  }
  .foot-note {
    margin: 0 0 0.45rem;
    line-height: 1.45;
  }
  .foot-meta {
    margin: 0;
    color: var(--ink-muted);
    font-size: 8pt;
    text-align: center;
  }

  /* Print rules - applied when this component is what's being printed.
     The host component (modal) toggles a body class so only the invoice
     prints, not the surrounding chrome. */
  @media print {
    .invoice-doc {
      box-shadow: none;
      border-radius: 0;
      margin: 0;
    }
  }

  /* Mobile screen preview - keep doc width but allow horizontal scroll */
  @media (max-width: 760px) {
    .invoice-doc {
      width: 100%;
      min-height: auto;
      padding: 4mm 4mm 6mm;
      font-size: 9.5pt;
    }
    .doc-head {
      flex-direction: column;
      gap: 0.7rem;
    }
    .doc-title {
      text-align: left;
    }
    .doc-parties {
      grid-template-columns: 1fr;
    }
    .doc-totals {
      grid-template-columns: 1fr;
    }
    .totals-block {
      padding: 0.6rem 0;
    }
  }
</style>
