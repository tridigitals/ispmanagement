<script lang="ts">
  import { t } from 'svelte-i18n';
  import type { GlobalSearchResultGroup, GlobalSearchResult } from '$lib/search/globalSearchModel';

  let {
    groups,
    loading,
    query,
    highlightedIndex = -1,
    onSelect,
  }: {
    groups: GlobalSearchResultGroup[];
    loading: boolean;
    query: string;
    highlightedIndex?: number;
    onSelect: (item: GlobalSearchResult) => void;
  } = $props();

  const flatItems = $derived.by(() =>
    groups.flatMap((group) =>
      group.items.map((item) => ({
        groupKey: group.key,
        item,
      })),
    ),
  );
</script>

<div class="search-results-panel">
  {#if loading}
    <div class="search-panel-state">Searching…</div>
  {:else if !query.trim()}
    <div class="search-panel-state">{$t('components.topbar_global_search.hint')}</div>
  {:else if !groups.length}
    <div class="search-panel-state">{$t('components.topbar_global_search.no_results')}</div>
  {:else}
    {#each groups as group (group.key)}
      <section class="search-group">
        <div class="search-group-label">{group.label}</div>
        <div class="search-group-items">
          {#each group.items as item}
            {@const itemIndex = flatItems.findIndex(
              (entry) => entry.groupKey === group.key && entry.item.id === item.id,
            )}
            <button
              class:active={itemIndex === highlightedIndex}
              class="search-result-item"
              type="button"
              onmousedown={(event) => event.preventDefault()}
              onclick={() => onSelect(item)}
            >
              <span class="search-result-title">{item.title}</span>
              <span class="search-result-subtitle">{item.subtitle}</span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</div>

<style>
  .search-results-panel {
    position: absolute;
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    width: min(640px, calc(100vw - 32px));
    max-height: min(70vh, 520px);
    overflow: auto;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 96%, var(--bg-primary));
    box-shadow: 0 24px 50px rgba(2, 6, 23, 0.28);
    padding: 12px;
    z-index: 60;
  }

  .search-panel-state {
    color: var(--text-secondary);
    font-size: 0.88rem;
    padding: 8px 4px;
  }

  .search-group {
    display: grid;
    gap: 8px;
  }

  .search-group + .search-group {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border-subtle);
  }

  .search-group-label {
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .search-group-items {
    display: grid;
    gap: 6px;
  }

  .search-result-item {
    display: grid;
    gap: 4px;
    width: 100%;
    text-align: left;
    border: 1px solid color-mix(in srgb, var(--border-color) 78%, transparent);
    border-radius: 12px;
    padding: 11px 12px;
    background: color-mix(in srgb, var(--bg-primary) 74%, transparent);
    color: var(--text-primary);
    cursor: pointer;
    transition:
      border-color 0.16s ease,
      background 0.16s ease,
      transform 0.16s ease;
  }

  .search-result-item.active,
  .search-result-item:hover {
    border-color: color-mix(in srgb, var(--color-primary) 46%, var(--border-color));
    background: color-mix(in srgb, var(--bg-secondary) 86%, var(--bg-surface));
    transform: translateY(-1px);
  }

  .search-result-title {
    font-weight: 700;
  }

  .search-result-subtitle {
    color: var(--text-secondary);
    font-size: 0.83rem;
  }
  @media (max-width: 900px) {
    .search-results-panel {
      width: min(420px, calc(100vw - 24px));
    }
  }
</style>
