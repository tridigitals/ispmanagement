<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    buildMixradiusSourceSummaryCards,
    type MixradiusImportBatch,
  } from './mixradiusImportTypes';

  let {
    batch,
    onNext,
    onBack,
  }: {
    batch: MixradiusImportBatch | null;
    onNext: () => void;
    onBack: () => void;
  } = $props();

  const cards = $derived.by(() => buildMixradiusSourceSummaryCards(batch));
  const unsupportedCount = $derived.by(
    () => cards.find((card) => card.key === 'customersUnsupported')?.value ?? 0
  );
</script>

<section class="mix-step">
  <div class="section-head">
    <div class="section-copy">
      <span class="section-eyebrow">Source review</span>
      <h2>Ringkasan sumber</h2>
      <p>Pastikan backup yang dipilih memang batch MixRadius yang ingin dimigrasikan.</p>
    </div>
    {#if batch}
      <div class="summary-chip">
        <span>Status parse</span>
        <strong>{batch.parseStatus}</strong>
      </div>
    {/if}
  </div>

  <div class="cards">
    {#each cards as card}
      <article class:warn={card.key === 'customersUnsupported' && card.value > 0} class="card">
        <div class="card-top">
          <span class="card-label">{card.label}</span>
          <Icon name={card.icon} size={16} />
        </div>
        <strong>{card.value.toLocaleString()}</strong>
      </article>
    {/each}
  </div>

  {#if unsupportedCount > 0}
    <div class="notice">
      <Icon name="alert-triangle" size={16} />
      <span>
        {unsupportedCount.toLocaleString()} customer non-PPP/hotspot terdeteksi dan tidak akan
        dimigrasikan pada fase ini. Import hanya memproses domain PPP agar lifecycle ISP
        Management tetap aman.
      </span>
    </div>
  {/if}

  <div class="meta-box">
    <div class="meta-head">
      <div>
        <span class="section-eyebrow">Source file</span>
        <strong>Detail backup yang akan diproses</strong>
      </div>
    </div>
    <div class="meta-grid">
      <div class="meta-item meta-item-wide">
        <span>Filename</span>
        <strong>{batch?.sourceFilename || '-'}</strong>
      </div>
      <div class="meta-item">
        <span>Size</span>
        <strong>{batch?.sourceSizeBytes?.toLocaleString() || 0} bytes</strong>
      </div>
      <div class="meta-item meta-item-wide">
        <span>Checksum</span>
        <code>{batch?.sourceSha256 || '-'}</code>
      </div>
    </div>
  </div>

  <div class="step-actions">
    <button class="btn ghost" type="button" onclick={onBack}>Back</button>
    <button class="btn primary" type="button" onclick={onNext}>
      Lanjut ke mapping
      <Icon name="arrow-right" size={16} />
    </button>
  </div>
</section>

<style>
  .mix-step,
  .meta-box {
    display: grid;
    gap: 18px;
  }

  .section-head,
  .card-top,
  .step-actions {
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

  .section-copy {
    display: grid;
    gap: 6px;
  }

  .section-eyebrow {
    color: var(--text-secondary);
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h2 {
    font-size: clamp(1.6rem, 2vw, 2rem);
    line-height: 1.05;
  }

  p,
  .meta-box span,
  code {
    color: var(--text-secondary);
  }

  .summary-chip,
  .card,
  .meta-box,
  .notice {
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-card, rgba(15, 23, 42, 0.72));
    border-radius: var(--radius-lg);
  }

  .summary-chip {
    min-width: 148px;
    padding: 12px 16px;
    display: grid;
    gap: 4px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 14px;
  }

  .card {
    min-height: 112px;
    padding: 16px 18px;
    display: grid;
    gap: 12px;
    align-content: space-between;
  }

  .card.warn {
    border-color: rgba(251, 191, 36, 0.35);
    background: rgba(251, 191, 36, 0.08);
  }

  .card-label {
    max-width: 12ch;
    line-height: 1.3;
  }

  .card strong {
    font-size: 2rem;
    line-height: 1;
  }

  .meta-box {
    padding: 18px;
  }

  .notice {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 14px 16px;
    color: #fde68a;
  }

  .meta-head strong {
    display: block;
    margin-top: 4px;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .meta-item {
    display: grid;
    gap: 6px;
    padding: 14px 0 0;
    border-top: 1px solid rgba(148, 163, 184, 0.14);
  }

  .meta-item-wide {
    grid-column: 1 / -1;
  }

  .meta-item strong {
    font-size: 1.05rem;
    line-height: 1.45;
  }

  code {
    word-break: break-all;
    font-size: 0.85rem;
    line-height: 1.5;
  }

  @media (max-width: 1100px) {
    .cards {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 720px) {
    .cards,
    .meta-grid {
      grid-template-columns: 1fr;
    }

    .meta-item-wide {
      grid-column: auto;
    }

    .card {
      min-height: unset;
    }
  }
</style>
