<script lang="ts">
  import type { NetworkMapInsightCard } from '$lib/components/network/networkMapInsights';

  let {
    cards,
    scopeLabel,
    emptyLabel,
  }: {
    cards: NetworkMapInsightCard[];
    scopeLabel: string;
    emptyLabel: string;
  } = $props();

  function toneClass(tone: NetworkMapInsightCard['tone']) {
    if (tone === 'ok') return 'tone-ok';
    if (tone === 'warn') return 'tone-warn';
    return 'tone-muted';
  }
</script>

<section class="insight-strip" aria-label="Operational insight strip">
  {#if cards.length}
    {#each cards as card (card.key)}
      <article class={`insight-card ${toneClass(card.tone)}`}>
        <div class="insight-meta">
          <span class="insight-label">{card.label}</span>
          <span class="scope-pill">{scopeLabel}</span>
        </div>
        <div class="insight-value">{card.value}</div>
        <p class="insight-detail">{card.detail}</p>
      </article>
    {/each}
  {:else}
    <div class="insight-empty">{emptyLabel}</div>
  {/if}
</section>

<style>
  .insight-strip {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 12px;
  }

  .insight-card {
    position: relative;
    overflow: hidden;
    min-height: 120px;
    border-radius: 18px;
    padding: 14px 15px;
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    background:
      radial-gradient(circle at top right, rgba(255, 255, 255, 0.08), transparent 32%),
      linear-gradient(180deg, color-mix(in srgb, var(--bg-card) 92%, #0b1220 8%), var(--bg-card));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.05),
      0 18px 38px rgba(2, 6, 23, 0.18);
  }

  .insight-card::after {
    content: '';
    position: absolute;
    inset: auto 0 0 0;
    height: 3px;
    background: color-mix(in srgb, var(--border-color) 75%, transparent);
  }

  .tone-ok::after {
    background: linear-gradient(90deg, #34d399, #10b981);
  }

  .tone-warn::after {
    background: linear-gradient(90deg, #fbbf24, #f97316);
  }

  .tone-muted::after {
    background: linear-gradient(90deg, #94a3b8, #64748b);
  }

  .insight-meta {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .insight-label {
    font-size: 0.76rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .scope-pill {
    flex-shrink: 0;
    border-radius: 999px;
    padding: 4px 8px;
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    color: color-mix(in srgb, var(--text-primary) 78%, white 22%);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--border-color) 72%, transparent);
  }

  .insight-value {
    margin-top: 16px;
    font-size: clamp(1.6rem, 1.2rem + 1vw, 2.2rem);
    line-height: 1;
    font-weight: 950;
    color: var(--text-primary);
  }

  .insight-detail {
    margin: 10px 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .insight-empty {
    border: 1px dashed color-mix(in srgb, var(--border-color) 76%, transparent);
    border-radius: 18px;
    padding: 18px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-card) 96%, transparent);
  }
</style>
