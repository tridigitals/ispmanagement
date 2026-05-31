<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import InvoicePrintable from './InvoicePrintable.svelte';
  import { generateInvoicePdf } from '$lib/utils/invoicePdf';
  import type { Invoice, BankAccount, Customer, CustomerSubscriptionView } from '$lib/api/types';
  import { t } from 'svelte-i18n';

  let {
    show = $bindable(false),
    invoice,
    customer = null,
    subscription = null,
    bankAccounts = [],
    onclose,
  } = $props<{
    show: boolean;
    invoice: Invoice;
    customer?: Customer | null;
    subscription?: CustomerSubscriptionView | null;
    bankAccounts?: BankAccount[];
    onclose?: () => void;
  }>();

  // Toggle a body-level class so the print stylesheet can hide everything
  // except the printable invoice. We add the class only while the modal is
  // open so the rest of the app's print behavior stays untouched.
  $effect(() => {
    if (typeof document === 'undefined') return;
    if (show) {
      document.body.classList.add('printing-invoice-active');
    } else {
      document.body.classList.remove('printing-invoice-active');
    }
  });

  onDestroy(() => {
    if (typeof document !== 'undefined') {
      document.body.classList.remove('printing-invoice-active');
    }
  });

  function close() {
    show = false;
    onclose?.();
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) close();
  }

  function handlePrint() {
    // Browsers/Tauri webview both expose `Save as PDF` in the native print
    // dialog, which produces a vector PDF that respects our print CSS. This
    // avoids bundling a heavy PDF generator client-side.
    if (typeof window !== 'undefined') window.print();
  }

  let downloadingPdf = $state(false);

  async function handleDownloadPdf() {
    if (downloadingPdf) return;
    const el = document.getElementById('invoice-print-area');
    if (!el) return;
    downloadingPdf = true;
    try {
      const filename = `Invoice-${invoice?.invoice_number || invoice?.id || 'document'}.pdf`;
      await generateInvoicePdf(el, filename);
    } catch (e: any) {
      console.error('PDF generation failed:', e);
      // Fallback to native print dialog
      if (typeof window !== 'undefined') window.print();
    } finally {
      downloadingPdf = false;
    }
  }

  // Suggested filename hint (browsers may honor in the print dialog title)
  $effect(() => {
    if (typeof document === 'undefined') return;
    if (!show) return;
    const original = document.title;
    const suggested = `Invoice-${invoice?.invoice_number || invoice?.id || 'document'}`;
    document.title = suggested;
    return () => {
      document.title = original;
    };
  });
</script>

{#if show}
  <div
    class="invoice-modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={(e) => e.key === 'Escape' && close()}
    role="presentation"
  >
    <div class="invoice-modal" role="dialog" aria-modal="true" aria-label="Invoice preview">
      <header class="invoice-modal-toolbar">
        <div class="toolbar-title">
          <Icon name="file-text" size={18} />
          <span>{$t('components.invoice_print.preview') || 'Invoice Preview'}</span>
        </div>
        <div class="toolbar-actions">
          <button class="btn-tool" type="button" onclick={handleDownloadPdf} disabled={downloadingPdf}>
            <Icon name="download" size={16} />
            <span>{downloadingPdf ? ($t('components.invoice_print.generating') || 'Generating...') : ($t('components.invoice_print.download_pdf') || 'Download PDF')}</span>
          </button>
          <button class="btn-tool btn-primary" type="button" onclick={handlePrint}>
            <Icon name="printer" size={16} />
            <span>{$t('components.invoice_print.print') || 'Cetak'}</span>
          </button>
          <button class="btn-tool" type="button" onclick={close} aria-label="Close">
            <Icon name="x" size={16} />
          </button>
        </div>
      </header>

      <div class="invoice-modal-body">
        <div class="invoice-stage" id="invoice-print-area">
          <InvoicePrintable
            {invoice}
            {customer}
            {subscription}
            {bankAccounts}
          />
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .invoice-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
    background: rgba(2, 6, 23, 0.7);
    display: grid;
    grid-template-rows: 1fr;
    overflow: hidden;
    padding: 1rem;
  }
  .invoice-modal {
    width: min(900px, 100%);
    max-height: 100%;
    margin: 0 auto;
    display: grid;
    grid-template-rows: auto 1fr;
    background: #f1f5f9;
    border-radius: 14px;
    overflow: hidden;
    box-shadow: 0 30px 70px rgba(2, 6, 23, 0.55);
  }
  .invoice-modal-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.7rem 0.95rem;
    background: var(--bg-surface, #fff);
    border-bottom: 1px solid var(--border-color, #e2e8f0);
  }
  .toolbar-title {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 700;
    color: var(--text-primary, #0f172a);
  }
  .toolbar-actions {
    display: flex;
    gap: 0.4rem;
  }
  .btn-tool {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.42rem 0.7rem;
    border: 1px solid var(--border-color, #cbd5e1);
    background: var(--bg-secondary, #f8fafc);
    color: var(--text-primary, #0f172a);
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .btn-tool:hover {
    background: var(--bg-surface, #fff);
  }
  .btn-tool.btn-primary {
    background: var(--color-primary, #2563eb);
    color: #fff;
    border-color: transparent;
  }
  .btn-tool.btn-primary:hover {
    background: color-mix(in srgb, var(--color-primary, #2563eb) 88%, #000);
  }
  .btn-tool:disabled {
    opacity: 0.55;
    cursor: wait;
  }

  .invoice-modal-body {
    overflow: auto;
    padding: 1.25rem;
    background: #f1f5f9;
  }
  .invoice-stage {
    margin: 0 auto;
    width: fit-content;
  }

  /* ---------- print rules ---------- */
  /* Print stylesheet hides everything except the printable area while
     `printing-invoice-active` is on body. Triggered by handlePrint(). */

  @media print {
    .invoice-modal-backdrop {
      position: static;
      inset: auto;
      background: transparent;
      padding: 0;
      overflow: visible;
    }
    .invoice-modal {
      box-shadow: none;
      border-radius: 0;
      max-height: none;
      background: transparent;
      display: block;
    }
    .invoice-modal-toolbar {
      display: none !important;
    }
    .invoice-modal-body {
      overflow: visible;
      padding: 0;
      background: #fff;
    }

    /* Hide everything in the document EXCEPT the print stage */
    :global(body.printing-invoice-active *) {
      visibility: hidden !important;
    }
    :global(body.printing-invoice-active #invoice-print-area),
    :global(body.printing-invoice-active #invoice-print-area *) {
      visibility: visible !important;
    }
    :global(body.printing-invoice-active #invoice-print-area) {
      position: absolute;
      left: 0;
      top: 0;
      width: 100%;
    }

    @page {
      size: A4 portrait;
      margin: 0;
    }
  }

  @media (max-width: 720px) {
    .invoice-modal-backdrop {
      padding: 0;
    }
    .invoice-modal {
      border-radius: 0;
    }
    .invoice-modal-body {
      padding: 0.5rem;
    }
  }
</style>
