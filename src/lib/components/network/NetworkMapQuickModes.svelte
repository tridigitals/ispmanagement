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

<section class="quick-modes" aria-label="Workspace quick modes">
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
    display: flex;
    gap: 10px;
    overflow-x: auto;
    padding-bottom: 2px;
    scrollbar-width: thin;
  }

  .quick-mode {
    flex: 0 0 auto;
    min-width: 156px;
    display: grid;
    gap: 8px;
    text-align: left;
    padding: 12px 14px;
    border-radius: 16px;
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
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-primary) 14%, var(--bg-card)),
        var(--bg-card)
      ),
      var(--bg-card);
    box-shadow: 0 16px 32px rgba(15, 23, 42, 0.18);
  }

  .quick-mode-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .quick-mode-label {
    font-weight: 800;
    font-size: 0.9rem;
  }

  .quick-mode-count {
    min-width: 28px;
    border-radius: 999px;
    padding: 4px 8px;
    text-align: center;
    font-size: 0.74rem;
    font-weight: 900;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
  }

  .quick-mode-hint {
    color: var(--text-secondary);
    font-size: 0.79rem;
    line-height: 1.35;
  }
</style>
