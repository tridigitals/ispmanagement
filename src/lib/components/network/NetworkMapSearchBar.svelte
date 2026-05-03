<script lang="ts">
  import type {
    NetworkMapSearchResultGroup,
    NetworkMapSearchResultItem,
  } from '$lib/components/network/networkMapInsights';
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    query,
    groups,
    summary,
    placeholder,
    emptyTitle,
    emptyHint,
    onQueryChange,
    onSelect,
  }: {
    query: string;
    groups: NetworkMapSearchResultGroup[];
    summary: string;
    placeholder: string;
    emptyTitle: string;
    emptyHint: string;
    onQueryChange: (value: string) => void;
    onSelect: (item: NetworkMapSearchResultItem) => void;
  } = $props();

  let highlightedIndex = $state(-1);
  let closeResultsTimer: ReturnType<typeof setTimeout> | null = null;
  let isFocused = $state(false);

  const flatItems = $derived.by(() =>
    groups.flatMap((group) =>
      group.items.map((item) => ({
        groupKey: group.key,
        item,
      })),
    ),
  );

  const showResults = $derived(isFocused && query.trim().length > 0);

  function resetHighlight() {
    highlightedIndex = flatItems.length ? 0 : -1;
  }

  function handleInput(value: string) {
    onQueryChange(value);
    highlightedIndex = 0;
  }

  function handleSelect(item: NetworkMapSearchResultItem) {
    onSelect(item);
    isFocused = false;
  }

  function onFocus() {
    if (closeResultsTimer) clearTimeout(closeResultsTimer);
    isFocused = true;
    resetHighlight();
  }

  function onBlur() {
    closeResultsTimer = setTimeout(() => {
      isFocused = false;
    }, 120);
  }

  function onKeydown(event: KeyboardEvent) {
    if (!showResults) return;
    if (!flatItems.length) {
      if (event.key === 'Escape') isFocused = false;
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      highlightedIndex = (highlightedIndex + 1 + flatItems.length) % flatItems.length;
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      highlightedIndex = (highlightedIndex - 1 + flatItems.length) % flatItems.length;
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      const activeItem = flatItems[Math.max(0, highlightedIndex)]?.item;
      if (activeItem) handleSelect(activeItem);
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      isFocused = false;
    }
  }

  function toneClass(item: NetworkMapSearchResultItem) {
    if (item.tone === 'ok') return 'tone-ok';
    if (item.tone === 'warn') return 'tone-warn';
    return 'tone-muted';
  }
</script>

<section class="search-shell">
  <div class="search-input-wrap">
    <Icon name="search" size={16} />
    <input
      class="search-input"
      type="text"
      value={query}
      {placeholder}
      autocomplete="off"
      oninput={(event) => handleInput((event.currentTarget as HTMLInputElement).value)}
      onfocus={onFocus}
      onblur={onBlur}
      onkeydown={onKeydown}
    />
  </div>
  <div class="search-summary">{summary}</div>

  {#if showResults}
    <div class="search-results">
      {#if groups.length}
        {#each groups as group (group.key)}
          <div class="search-group">
            <div class="search-group-label">{group.label}</div>
            <div class="search-group-items">
              {#each group.items as item}
                {@const itemIndex = flatItems.findIndex(
                  (entry) => entry.groupKey === group.key && entry.item.id === item.id,
                )}
                <button
                  type="button"
                  class={`search-item ${toneClass(item)} ${itemIndex === highlightedIndex ? 'active' : ''}`}
                  onmousedown={(event) => event.preventDefault()}
                  onclick={() => handleSelect(item)}
                >
                  <span class="search-item-main">
                    <span class="search-item-label">{item.label}</span>
                    <span class="search-item-kind">{item.kind}</span>
                  </span>
                  <span class="search-item-subtitle">{item.subtitle}</span>
                </button>
              {/each}
            </div>
          </div>
        {/each}
      {:else}
        <div class="search-empty">
          <div class="search-empty-title">{emptyTitle}</div>
          <div class="search-empty-hint">{emptyHint}</div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .search-shell {
    position: relative;
    display: grid;
    gap: 8px;
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    gap: 9px;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, transparent);
    padding: 0 12px;
    background: var(--bg-surface);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  }

  .search-input-wrap :global(svg) {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .search-input {
    width: 100%;
    min-height: 44px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.92rem;
  }

  .search-summary {
    color: var(--text-secondary);
    font-size: 0.76rem;
  }

  .search-results {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 8px);
    z-index: 15;
    display: grid;
    gap: 10px;
    max-height: min(48vh, 460px);
    overflow: auto;
    border-radius: var(--radius-lg);
    border: 1px solid color-mix(in srgb, var(--border-color) 78%, transparent);
    padding: 14px;
    background: color-mix(in srgb, var(--bg-card) 95%, #06101b 5%);
    box-shadow: 0 28px 50px rgba(2, 6, 23, 0.26);
      }

  .search-group {
    display: grid;
    gap: 8px;
  }

  .search-group-label {
    font-size: 0.73rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
    font-weight: 900;
  }

  .search-group-items {
    display: grid;
    gap: 6px;
  }

  .search-item {
    display: grid;
    gap: 4px;
    text-align: left;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color) 76%, transparent);
    padding: 10px 12px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    cursor: pointer;
  }

  .search-item.active {
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 12%, var(--bg-surface));
  }

  .search-item-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .search-item-label {
    font-weight: 800;
  }

  .search-item-kind {
    text-transform: capitalize;
    font-size: 0.72rem;
    font-weight: 900;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  .search-item-subtitle {
    color: var(--text-secondary);
    font-size: 0.83rem;
  }

  .tone-ok {
    box-shadow: inset 2px 0 0 #10b981;
  }

  .tone-warn {
    box-shadow: inset 2px 0 0 #f59e0b;
  }

  .tone-muted {
    box-shadow: inset 2px 0 0 #64748b;
  }

  .search-empty {
    display: grid;
    gap: 4px;
    padding: 8px 4px;
  }

  .search-empty-title {
    font-weight: 800;
    color: var(--text-primary);
  }

  .search-empty-hint {
    color: var(--text-secondary);
    font-size: 0.84rem;
  }
</style>
