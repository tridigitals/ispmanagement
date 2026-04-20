<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    buildMixradiusBatchReport,
    type MixradiusImportBatch,
  } from './mixradiusImportTypes';

  let {
    open = false,
    batch = null,
    loading = false,
    onClose,
  }: {
    open?: boolean;
    batch?: MixradiusImportBatch | null;
    loading?: boolean;
    onClose: () => void;
  } = $props();

  const report = $derived.by(() => buildMixradiusBatchReport(batch));
</script>

{#if open}
  <button class="drawer-backdrop" type="button" onclick={onClose} aria-label="Close report"></button>
  <aside class="drawer" aria-label="MixRadius batch report">
    <div class="drawer-head">
      <div>
        <div class="drawer-title">Batch report</div>
        <div class="drawer-sub">{batch?.sourceFilename || 'MixRadius import'}</div>
      </div>
      <button class="icon-btn" type="button" onclick={onClose} disabled={loading}>
        <Icon name="x" size={16} />
      </button>
    </div>

    <div class="drawer-body">
      {#if loading}
        <div class="empty">Memuat report batch...</div>
      {:else if !batch}
        <div class="empty">Batch report belum tersedia.</div>
      {:else}
        <div class="status-line">
          <span>Status</span>
          <strong class={`tone-${report.status.tone}`}>{report.status.label}</strong>
        </div>

        <section class="section">
          <h3>Source summary</h3>
          <div class="source-grid">
            {#each report.source as card}
              <article class="metric-card">
                <span>{card.label}</span>
                <strong>{card.value.toLocaleString()}</strong>
              </article>
            {/each}
          </div>
        </section>

        <section class="section">
          <h3>Phase report</h3>
          <div class="phase-list">
            {#each report.phases as phase}
              <article class="phase-card">
                <div class="phase-head">
                  <strong>{phase.label}</strong>
                  <span>{phase.status}</span>
                </div>
                <small>Imported {phase.imported}</small>
                <small>Updated {phase.updated}</small>
              </article>
            {/each}
          </div>
        </section>

        <section class="section">
          <h3>Billing lifecycle</h3>
          <div class="source-grid">
            <article class="metric-card">
              <span>Legacy transactions</span>
              <strong>{report.billing.legacyTransactionCount.toLocaleString()}</strong>
            </article>
            <article class="metric-card">
              <span>Production invoices</span>
              <strong>{report.billing.productionInvoiceCount.toLocaleString()}</strong>
            </article>
          </div>
        </section>

        {#if report.errors.length}
          <section class="section">
            <h3>Errors</h3>
            <div class="error-list">
              {#each report.errors as item}
                <article class="error-card">
                  <strong>{String(item.phase ?? 'unknown')}</strong>
                  <p>{String(item.message ?? 'Unknown error')}</p>
                </article>
              {/each}
            </div>
          </section>
        {/if}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 50;
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: min(560px, 92vw);
    height: 100vh;
    background: var(--bg-surface);
    border-left: 1px solid var(--border-color);
    z-index: 51;
    display: grid;
    grid-template-rows: auto 1fr;
  }

  .drawer-head {
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .drawer-title {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .drawer-sub {
    margin-top: 6px;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .drawer-body {
    padding: 16px;
    display: grid;
    gap: 18px;
    overflow: auto;
  }

  .section,
  .source-grid,
  .phase-list,
  .error-list {
    display: grid;
    gap: 12px;
  }

  .source-grid {
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  }

  .metric-card,
  .phase-card,
  .error-card,
  .status-line,
  .empty {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-card);
    padding: 12px 14px;
  }

  .phase-head,
  .status-line {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: center;
  }

  .metric-card span,
  .phase-card small,
  .phase-head span,
  .error-card p,
  .status-line span,
  .empty {
    color: var(--text-secondary);
  }

  .tone-success {
    color: #86efac;
  }

  .tone-warning {
    color: #fde68a;
  }

  .tone-danger {
    color: #fca5a5;
  }

  .tone-muted {
    color: #cbd5e1;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  h3,
  p {
    margin: 0;
  }
</style>
