<script lang="ts">
  export type NetworkMapQuickModeOption = {
    key: string;
    label: string;
    hint: string;
    count: number;
  };

  let {
    modes,
    activeKey,
    onSelect,
  }: {
    modes: readonly NetworkMapQuickModeOption[];
    activeKey: string;
    onSelect: (key: string) => void;
  } = $props();
</script>

<section class="quick-modes" aria-label={$t('network.map.quick_modes_label') || 'Workspace quick modes'}>
  {#each modes as mode (mode.key)}
    <button
      type="button"
      class={`quick-mode ${activeKey === mode.key ? 'active' : ''}`}
      onclick={() => onSelect(mode.key)}
    >
      <span class="quick-mode-top">
        <span class="quick-mode-label">{mode.label}</span>
        <span class="quick-mode-count">{mode.count}</span>
      </span>
      <span class="quick-mode-hint">{mode.hint}</span>
    </button>
  {/each}
</section>

<style>
  .quick-modes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    align-items: stretch;
    overflow-x: auto;
    padding-bottom: 2px;
    scrollbar-width: thin;
  }

  .quick-mode {
    min-width: 0;
    display: grid;
    gap: 6px;
    text-align: left;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color) 82%, transparent);
    background: color-mix(in srgb, var(--bg-card) 95%, #07111f 5%);
    color: var(--text-primary);
    cursor: pointer;
    transition:
      transform 140ms ease,
      border-color 140ms ease,
      background 140ms ease,
      box-shadow 140ms ease;
  }

  .quick-mode:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--color-primary) 32%, var(--border-color));
  }

  .quick-mode.active {
    border-color: color-mix(in srgb, var(--color-primary) 60%, var(--border-color));
    background: var(--bg-surface);
    box-shadow: 0 12px 24px rgba(15, 23, 42, 0.16);
  }

  .quick-mode-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .quick-mode-label {
    font-weight: 800;
    font-size: 0.84rem;
  }

  .quick-mode-count {
    min-width: 24px;
    border-radius: 999px;
    padding: 3px 7px;
    text-align: center;
    font-size: 0.7rem;
    font-weight: 900;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
  }

  .quick-mode-hint {
    color: var(--text-secondary);
    font-size: 0.73rem;
    line-height: 1.28;
    line-clamp: 2;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  @media (max-width: 980px) {
    .quick-modes {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .quick-modes {
      grid-template-columns: 1fr;
    }
  }
</style>
