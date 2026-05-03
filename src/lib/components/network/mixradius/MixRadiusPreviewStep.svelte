<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    buildMixradiusPreviewCounts,
    getMixradiusConflictBadge,
    type MixradiusImportPreview,
    type MixradiusImportPreviewRow,
  } from './mixradiusImportTypes';

  let {
    preview,
    activeTab = $bindable<'all' | MixradiusImportPreviewRow['conflictState']>('all'),
    onBack,
    onNext,
  }: {
    preview: MixradiusImportPreview | null;
    activeTab?: 'all' | MixradiusImportPreviewRow['conflictState'];
    onBack: () => void;
    onNext: () => void;
  } = $props();

  const counts = $derived.by(() => buildMixradiusPreviewCounts(preview?.rows ?? []));
  const tabs: Array<'all' | MixradiusImportPreviewRow['conflictState']> = [
    'all',
    'blocked',
    'conflict',
    'needs_review',
    'auto_matched',
    'skipped',
  ];
  const filteredRows = $derived.by(() =>
    (preview?.rows ?? []).filter((row) => activeTab === 'all' || row.conflictState === activeTab)
  );
  const sourceKindCards = $derived.by(() =>
    Object.entries(counts.bySourceKind)
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([sourceKind, total]) => ({ sourceKind, total }))
  );
</script>

<section class="mix-step">
  <div class="section-head">
    <div>
      <h2>Preview hasil import</h2>
      <p>Cek blocked/conflict lebih dulu sebelum eksekusi ke data produksi.</p>
    </div>
  </div>

  <div class="cards">
    <article class="card"><span>Total</span><strong>{counts.total}</strong></article>
    <article class="card"><span>Auto matched</span><strong>{counts.autoMatched}</strong></article>
    <article class="card tone-warn"><span>Needs review</span><strong>{counts.needsReview}</strong></article>
    <article class="card tone-danger"><span>Conflicts</span><strong>{counts.conflicts}</strong></article>
    <article class="card tone-danger"><span>Blocked</span><strong>{counts.blocked}</strong></article>
  </div>

  {#if sourceKindCards.length}
    <div class="kind-grid">
      {#each sourceKindCards as item}
        <article class="kind-card">
          <span>{item.sourceKind}</span>
          <strong>{item.total}</strong>
        </article>
      {/each}
    </div>
  {/if}

  <div class="tabs">
    {#each tabs as tab}
      <button
        class:active={activeTab === tab}
        type="button"
        onclick={() => (activeTab = tab)}
      >
        {tab === 'all' ? 'All' : getMixradiusConflictBadge(tab).label}
      </button>
    {/each}
  </div>

  <div class="rows">
    {#if filteredRows.length === 0}
      <div class="empty">Tidak ada row untuk tab ini.</div>
    {:else}
      {#each filteredRows as row}
        {@const badge = getMixradiusConflictBadge(row.conflictState)}
        <article class="row-card">
          <div class="row-head">
            <div>
              <strong>{row.displayName || row.sourceRef}</strong>
              <span>{row.sourceKind} · {row.sourceRef}</span>
            </div>
            <span class={`badge ${badge.tone}`}>{badge.label}</span>
          </div>
          {#if row.notes}
            <p>{row.notes}</p>
          {/if}
          {#if row.targetId || row.targetKind}
            <code>{row.targetKind || 'target'}: {row.targetId || 'pending'}</code>
          {/if}
        </article>
      {/each}
    {/if}
  </div>

  <div class="step-actions">
    <button class="btn ghost" type="button" onclick={onBack}>Back</button>
    <button class="btn primary" type="button" onclick={onNext}>
      Lanjut ke execute
      <Icon name="arrow-right" size={16} />
    </button>
  </div>
</section>

<style>
  .mix-step,
  .rows {
    display: grid;
    gap: 16px;
  }

  .section-head,
  .step-actions,
  .row-head {
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

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 12px;
  }

  .kind-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 10px;
  }

  .card,
  .kind-card,
  .row-card,
  .empty {
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-card, rgba(15, 23, 42, 0.72));
    border-radius: var(--radius-lg);
    padding: 14px 16px;
  }

  .card {
    display: grid;
    gap: 8px;
  }

  .kind-card {
    display: grid;
    gap: 6px;
  }

  .card span,
  .kind-card span,
  .row-head span,
  p,
  code {
    color: var(--text-secondary);
  }

  .card strong {
    font-size: 1.3rem;
  }

  .tabs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .tabs button {
    border-radius: 999px;
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: transparent;
    color: var(--text-secondary);
    padding: 8px 12px;
  }

  .tabs button.active {
    background: rgba(14, 165, 233, 0.18);
    color: var(--text-primary);
  }

  .badge.success {
    color: #86efac;
  }

  .badge.warning {
    color: #fde68a;
  }

  .badge.danger {
    color: #fca5a5;
  }

  .badge.muted {
    color: #cbd5e1;
  }
</style>
