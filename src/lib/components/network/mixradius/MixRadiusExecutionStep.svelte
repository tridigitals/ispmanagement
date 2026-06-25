<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    buildMixradiusExecutionHighlights,
    getMixradiusExecutionModeLabel,
    getMixradiusSafeModeExecuteState,
    type MixradiusImportExecutionMode,
    type MixradiusImportExecutionResult,
    type MixradiusImportPreviewRow,
  } from './mixradiusImportTypes';

  let {
    rows = [],
    executionMode = $bindable<MixradiusImportExecutionMode>('safe_import'),
    executing = false,
    result = null,
    onBack,
    onCancel,
    onExecute,
  }: {
    rows?: MixradiusImportPreviewRow[];
    executionMode?: MixradiusImportExecutionMode;
    executing?: boolean;
    result?: MixradiusImportExecutionResult | null;
    onBack: () => void;
    onCancel: () => void | Promise<void>;
    onExecute: () => void | Promise<void>;
  } = $props();

  const executeState = $derived.by(() => getMixradiusSafeModeExecuteState(executionMode, rows));
  const modes: MixradiusImportExecutionMode[] = ['preview_only', 'safe_import', 'force_sync'];
  const highlights = $derived.by(() => buildMixradiusExecutionHighlights(result));
</script>

<section class="mix-step">
  <div class="section-head">
    <div>
      <h2>{$t('mixradius.import_wizard.execution.title')}</h2>
      <p>{$t('mixradius.import_wizard.execution.description')}</p>
    </div>
  </div>

  <div class="mode-list">
    {#each modes as mode}
      {@const meta = getMixradiusExecutionModeLabel(mode)}
      <label class:active={executionMode === mode} class="mode-card">
        <input type="radio" bind:group={executionMode} value={mode} />
        <div>
          <strong>{meta.label}</strong>
          <p>{meta.description}</p>
        </div>
      </label>
    {/each}
  </div>

  {#if executeState.disabled}
    <div class="warn-line"><Icon name="shield-alert" size={16} /> {executeState.reason}</div>
  {/if}

  {#if result}
    <article class="report-card">
      <div class="report-grid">
        {#each highlights.totals as item}
          <div>
            <span>{item.label}</span>
            <strong>{item.value}</strong>
          </div>
        {/each}
      </div>

      <div class="phase-grid">
        {#each highlights.phases as phase}
          <article class="phase-card">
            <strong>{phase.label}</strong>
            <span>Imported {phase.imported}</span>
            <span>Updated {phase.updated}</span>
          </article>
        {/each}
      </div>

      <div class="billing-box">
        <div>
          <span>{$t('mixradius.import_wizard.execution.legacy_transactions')}</span>
          <strong>{highlights.billing.legacyTransactionCount}</strong>
        </div>
        <div>
          <span>{$t('mixradius.import_wizard.execution.production_invoices')}</span>
          <strong>{highlights.billing.productionInvoiceCount}</strong>
        </div>
      </div>

      {#if highlights.billing.warnings.length}
        <ul class="warnings">
          {#each highlights.billing.warnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}
    </article>
  {/if}

  <div class="step-actions">
    <button class="btn ghost" type="button" onclick={onBack}>{$t('mixradius.import_wizard.execution.back')}</button>
    <div class="right-actions">
      <button class="btn ghost" type="button" onclick={onCancel}>{$t('mixradius.import_wizard.execution.cancel_batch')}</button>
      <button
        class="btn primary"
        type="button"
        onclick={onExecute}
        disabled={executing || executeState.disabled}
      >
        {executing ? 'Executing...' : 'Execute import'}
        <Icon name="play" size={16} />
      </button>
    </div>
  </div>
</section>

<style>
  .mix-step,
  .mode-list,
  .report-card,
  .report-grid,
  .phase-grid {
    display: grid;
    gap: 16px;
  }

  .section-head,
  .step-actions,
  .right-actions {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }

  h2,
  p {
    margin: 0;
  }

  .mode-card,
  .report-card,
  .phase-card,
  .billing-box {
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-card, rgba(15, 23, 42, 0.72));
    border-radius: var(--radius-lg);
    padding: 16px;
  }

  .mode-card {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .mode-card.active {
    border-color: rgba(56, 189, 248, 0.5);
    box-shadow: inset 0 0 0 1px rgba(56, 189, 248, 0.18);
  }

  p,
  .warn-line,
  .report-grid span {
    color: var(--text-secondary);
  }

  .warn-line {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #fde68a;
  }

  .report-grid {
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  }

  .phase-grid {
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  }

  .phase-card {
    display: grid;
    gap: 6px;
  }

  .phase-card strong {
    font-size: 0.98rem;
  }

  .report-grid strong {
    display: block;
    margin-top: 4px;
    font-size: 1.25rem;
  }

  .billing-box {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 12px;
  }

  .billing-box span,
  .phase-card span {
    color: var(--text-secondary);
  }

  .billing-box strong {
    display: block;
    margin-top: 6px;
    font-size: 1.1rem;
  }

  .warnings {
    margin: 0;
    padding-left: 18px;
    color: var(--text-secondary);
  }
</style>
