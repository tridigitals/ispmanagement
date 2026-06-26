<script lang="ts">
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Input from '$lib/components/ui/Input.svelte';
  import InvoicePrintable from '$lib/components/invoice/InvoicePrintable.svelte';
  import type { Invoice, BankAccount, Customer } from '$lib/api/types';

  let {
    localSettings,
    handleChange,
  }: {
    localSettings: Record<string, string>;
    handleChange: (key: string, value: any) => void;
  } = $props();

  function tt(key: string, fallback: string) {
    const value = get(t)(key);
    return value && value !== key ? value : fallback;
  }

  // ---- Live preview synthesis --------------------------------------------
  // We render the same InvoicePrintable component with a synthetic invoice
  // so the operator sees exactly what the customer will receive when they
  // press "Cetak / Unduh PDF". The settings flow through `appSettings` store
  // which the component reads directly — operators don't need to save first
  // because handleChange already mutates `localSettings` and the global
  // appSettings reactive store updates from the parent settings page.

  const sampleInvoice = $derived<Invoice>({
    id: 'preview-001',
    tenant_id: 'preview',
    invoice_number: 'INV-2026-0001',
    amount: 350000,
    currency_code: 'IDR',
    base_currency_code: 'IDR',
    fx_rate: null,
    fx_source: null,
    fx_fetched_at: null,
    status: 'pending',
    description: 'Subscription · Paket Internet 20 Mbps',
    due_date: new Date(Date.now() + 7 * 86400000).toISOString(),
    paid_at: null,
    payment_method: null,
    external_id: 'pkgsub:preview',
    merchant_id: null,
    proof_attachment: null,
    rejection_reason: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  });

  const sampleCustomer: Customer = {
    id: 'preview-cust',
    tenant_id: 'preview',
    name: 'Budi Santoso',
    email: 'budi@example.com',
    phone: '+62 812-3456-7890',
    notes: null,
    is_active: true,
    created_at: '',
    updated_at: '',
  };

  const sampleBanks: BankAccount[] = [
    {
      id: 'b1',
      bank_name: 'BCA',
      account_number: '1234567890',
      account_holder: 'PT ISP Demo',
      is_active: true,
    },
    {
      id: 'b2',
      bank_name: 'Mandiri',
      account_number: '9876543210',
      account_holder: 'PT ISP Demo',
      is_active: true,
    },
  ];

  // Logo upload (data URL inline so preview reflects it without going through
  // the storage backend). If the operator wants persistence they can plug in
  // an existing storage upload later — for now the path can be a public URL.
  let logoFileInput = $state<HTMLInputElement | null>(null);

  function pickLogo() {
    logoFileInput?.click();
  }

  function onLogoSelected(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) return;
    if (file.size > 800 * 1024) {
      alert(tt('admin.settings.company.logo_too_big', 'Logo harus < 800KB'));
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = String(reader.result || '');
      if (dataUrl) handleChange('company_logo', dataUrl);
    };
    reader.readAsDataURL(file);
  }

  function clearLogo() {
    handleChange('company_logo', '');
  }
</script>

<div class="company-tab">
  <header class="tab-head">
    <h2>{tt('admin.settings.company.title', 'Perusahaan & Invoice')}</h2>
    <p class="tab-subtitle">
      {tt(
        'admin.settings.company.subtitle',
        'Identitas perusahaan ini muncul di header invoice yang dicetak/diunduh PDF oleh pelanggan.',
      )}
    </p>
  </header>

  <div class="layout">
    <!-- Form column ---------------------------------------------------- -->
    <div class="form-col">
      <section class="card">
        <h3>{tt('admin.settings.company.identity', 'Identitas Perusahaan')}</h3>
        <p class="card-hint">
          {tt(
            'admin.settings.company.identity_hint',
            'Nama dan kontak resmi yang ditampilkan pada dokumen invoice.',
          )}
        </p>

        <div class="form-grid">
          <div class="form-row form-row-wide">
            <label for="org-name">{tt('admin.settings.company.fields.name', 'Nama Perusahaan')}</label>
            <Input
              id="org-name"
              type="text"
              value={localSettings['organization_name'] || ''}
              oninput={(e: Event) =>
                handleChange('organization_name', (e.target as HTMLInputElement).value)}
              placeholder={tt('admin.settings.company.name_placeholder', 'PT Internet Service Provider')}
            />
          </div>

          <div class="form-row form-row-wide">
            <label for="org-address">{tt('admin.settings.company.fields.address', 'Alamat')}</label>
            <textarea
              id="org-address"
              class="form-textarea"
              rows="2"
              value={localSettings['company_address'] || ''}
              oninput={(e) =>
                handleChange('company_address', (e.target as HTMLTextAreaElement).value)}
              placeholder={tt('admin.settings.company.address_placeholder', 'Jl. Raya No. 123, Jakarta 12345')}
            ></textarea>
          </div>

          <div class="form-row">
            <label for="org-phone">{tt('admin.settings.company.fields.phone', 'Telepon')}</label>
            <Input
              id="org-phone"
              type="text"
              value={localSettings['company_phone'] || ''}
              oninput={(e: Event) =>
                handleChange('company_phone', (e.target as HTMLInputElement).value)}
              placeholder="+62 21-1234-5678"
            />
          </div>

          <div class="form-row">
            <label for="org-email">{tt('admin.settings.company.fields.email', 'Email')}</label>
            <Input
              id="org-email"
              type="email"
              value={localSettings['company_email'] || ''}
              oninput={(e: Event) =>
                handleChange('company_email', (e.target as HTMLInputElement).value)}
              placeholder="billing@isp.co.id"
            />
          </div>

          <div class="form-row">
            <label for="org-whatsapp">{tt('admin.settings.company.fields.whatsapp', 'WhatsApp')}</label>
            <Input
              id="org-whatsapp"
              type="text"
              value={localSettings['company_whatsapp'] || ''}
              oninput={(e: Event) =>
                handleChange('company_whatsapp', (e.target as HTMLInputElement).value)}
              placeholder="+62 812-3456-7890"
            />
          </div>

          <div class="form-row">
            <label for="org-npwp">{tt('admin.settings.company.fields.npwp', 'NPWP')}</label>
            <Input
              id="org-npwp"
              type="text"
              value={localSettings['company_npwp'] || ''}
              oninput={(e: Event) =>
                handleChange('company_npwp', (e.target as HTMLInputElement).value)}
              placeholder="00.000.000.0-000.000"
            />
          </div>

          <div class="form-row">
            <label for="org-website">{tt('admin.settings.company.fields.website', 'Website')}</label>
            <Input
              id="org-website"
              type="text"
              value={localSettings['company_website'] || ''}
              oninput={(e: Event) =>
                handleChange('company_website', (e.target as HTMLInputElement).value)}
              placeholder="https://isp.co.id"
            />
          </div>
        </div>
      </section>

      <section class="card">
        <h3>{tt('admin.settings.company.logo', 'Logo Invoice')}</h3>
        <p class="card-hint">
          {tt(
            'admin.settings.company.logo_hint',
            'Logo muncul di header invoice. Disarankan PNG transparan, ukuran < 800KB.',
          )}
        </p>

        <div class="logo-row">
          <div class="logo-preview">
            {#if localSettings['company_logo']}
              <img src={localSettings['company_logo']} alt={tt('admin.settings.company.logo_preview_alt', 'Logo preview')} />
            {:else}
              <div class="logo-empty">
                {tt('admin.settings.company.logo_empty', 'Belum ada logo')}
              </div>
            {/if}
          </div>
          <div class="logo-actions">
            <button class="btn btn-secondary" type="button" onclick={pickLogo}>
              {tt('admin.settings.company.logo_pick', 'Pilih Logo')}
            </button>
            {#if localSettings['company_logo']}
              <button class="btn btn-link" type="button" onclick={clearLogo}>
                {tt('admin.settings.company.logo_clear', 'Hapus')}
              </button>
            {/if}
            <input
              bind:this={logoFileInput}
              type="file"
              accept="image/*"
              class="hidden-input"
              onchange={onLogoSelected}
            />
          </div>
        </div>

        <div class="form-row">
          <label for="logo-url">
            {tt('admin.settings.company.logo_url', 'Atau URL logo eksternal')}
          </label>
          <Input
            id="logo-url"
            type="text"
            value={
              localSettings['company_logo']?.startsWith('data:')
                ? ''
                : localSettings['company_logo'] || ''
            }
            oninput={(e: Event) =>
              handleChange('company_logo', (e.target as HTMLInputElement).value)}
            placeholder="https://cdn.isp.co.id/logo.png"
          />
        </div>
      </section>

      <section class="card">
        <h3>{tt('admin.settings.company.invoice_doc', 'Dokumen Invoice')}</h3>
        <p class="card-hint">
          {tt(
            'admin.settings.company.invoice_doc_hint',
            'Catatan kaki dan teks pelengkap pada PDF invoice.',
          )}
        </p>

        <div class="form-row form-row-wide">
          <label for="footer-note">
            {tt('admin.settings.company.footer_note', 'Catatan Kaki Invoice')}
          </label>
          <textarea
            id="footer-note"
            class="form-textarea"
            rows="3"
            value={localSettings['invoice_footer_note'] || ''}
            oninput={(e) =>
              handleChange('invoice_footer_note', (e.target as HTMLTextAreaElement).value)}
            placeholder="Terima kasih atas kepercayaan Anda. Hubungi kami di support@isp.co.id untuk pertanyaan tagihan."
          ></textarea>
          <small class="hint">
            {tt(
              'admin.settings.company.footer_note_help',
              'Muncul di bagian bawah invoice sebagai pesan terima kasih atau instruksi tambahan.',
            )}
          </small>
        </div>
      </section>
    </div>

    <!-- Live preview column ------------------------------------------- -->
    <aside class="preview-col">
      <div class="preview-head">
        <h3>{tt('admin.settings.company.preview', 'Pratinjau Invoice')}</h3>
        <p class="hint">
          {tt(
            'admin.settings.company.preview_hint',
            'Pratinjau menggunakan data contoh. Perubahan disimpan saat klik "Save Changes" pada toolbar atas.',
          )}
        </p>
      </div>
      <div class="preview-frame">
        <div class="preview-scale">
          <InvoicePrintable
            invoice={sampleInvoice}
            customer={sampleCustomer}
            bankAccounts={sampleBanks}
            embed={true}
          />
        </div>
      </div>
    </aside>
  </div>
</div>

<style>
  .company-tab {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
  .tab-head h2 {
    margin: 0 0 0.25rem;
    font-size: 1.2rem;
  }
  .tab-subtitle {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 0.95fr);
    gap: 1.1rem;
    align-items: start;
  }
  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .form-col {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .card {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    border-radius: 12px;
    padding: 1rem 1.05rem;
  }
  .card h3 {
    margin: 0 0 0.25rem;
    font-size: 1rem;
  }
  .card-hint {
    margin: 0 0 0.85rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem 0.9rem;
  }
  .form-row {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .form-row-wide {
    grid-column: 1 / -1;
  }
  .form-row label {
    font-size: 0.82rem;
    color: var(--text-secondary);
    font-weight: 600;
  }
  .form-textarea {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 8px;
    padding: 0.55rem 0.7rem;
    font: inherit;
    resize: vertical;
  }
  .hint {
    color: var(--text-tertiary, var(--text-secondary));
    font-size: 0.78rem;
  }
  @media (max-width: 720px) {
    .form-grid {
      grid-template-columns: 1fr;
    }
  }

  .logo-row {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    flex-wrap: wrap;
    margin-bottom: 0.85rem;
  }
  .logo-preview {
    width: 96px;
    height: 96px;
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    background: var(--bg-secondary);
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .logo-preview img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .logo-empty {
    color: var(--text-tertiary, var(--text-secondary));
    font-size: 0.78rem;
    text-align: center;
    padding: 0 0.4rem;
  }
  .logo-actions {
    display: flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }
  .hidden-input {
    display: none;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.85rem;
    border-radius: 8px;
    border: 1px solid transparent;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .btn-secondary {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--border-color);
  }
  .btn-link {
    background: transparent;
    color: var(--color-danger, #dc2626);
    border-color: transparent;
  }
  .btn-link:hover {
    text-decoration: underline;
  }

  .preview-col {
    position: sticky;
    top: 1rem;
  }
  .preview-head {
    margin-bottom: 0.6rem;
  }
  .preview-head h3 {
    margin: 0 0 0.2rem;
    font-size: 0.95rem;
  }
  .preview-head .hint {
    display: block;
    margin: 0;
    line-height: 1.4;
  }
  .preview-frame {
    border: 1px solid var(--border-color);
    background: #f1f5f9;
    border-radius: 12px;
    padding: 0.8rem;
    overflow: hidden;
  }
  /* Scale the A4 doc down to fit the preview pane while preserving its
     proportions. transform-origin keeps the top-left aligned so the
     scrollable region behaves predictably. */
  .preview-scale {
    transform: scale(0.55);
    transform-origin: top left;
    width: 210mm; /* InvoicePrintable native width */
    margin-bottom: -45%; /* compensate the empty space caused by scale */
  }
  @media (max-width: 1100px) {
    .preview-col {
      position: static;
    }
    .preview-scale {
      transform: scale(0.7);
      margin-bottom: -30%;
    }
  }
  @media (max-width: 760px) {
    .preview-scale {
      transform: scale(0.4);
      margin-bottom: -55%;
    }
  }
</style>
