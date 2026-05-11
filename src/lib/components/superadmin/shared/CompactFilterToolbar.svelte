<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    searchQuery = $bindable(''),
    placeholder = 'Search...',
    filterPanelOpen = $bindable(false),
    activeFilterCount = 0,
    onReset,
    isMobile = false,
    viewMode = $bindable<'table' | 'cards'>('table'),
    showViewToggle = true,
    advancedFilters,
    actions,
  } = $props<{
    searchQuery?: string;
    placeholder?: string;
    filterPanelOpen?: boolean;
    activeFilterCount?: number;
    onReset?: () => void;
    isMobile?: boolean;
    viewMode?: 'table' | 'cards';
    showViewToggle?: boolean;
    advancedFilters?: import('svelte').Snippet;
    actions?: import('svelte').Snippet;
  }>();

  function clearSearch() {
    searchQuery = '';
  }
</script>

<div class="toolbar-shell">
  <div class="toolbar-main">
    <div class="search-input-wrapper">
      <span class="search-icon">
        <Icon name="search" size={18} />
      </span>
      <input type="text" bind:value={searchQuery} {placeholder} />
      {#if searchQuery}
        <button class="clear-btn" type="button" onclick={clearSearch}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </div>

    <button
      class="filter-toggle"
      type="button"
      class:open={filterPanelOpen}
      onclick={() => (filterPanelOpen = !filterPanelOpen)}
      aria-expanded={filterPanelOpen}
    >
      <Icon name="settings" size={16} />
      <span>Filter{activeFilterCount > 0 ? ` (${activeFilterCount})` : ''}</span>
      <Icon name={filterPanelOpen ? 'chevron-up' : 'chevron-down'} size={16} />
    </button>

    {#if showViewToggle && !isMobile}
      <div class="view-toggle">
        <button
          type="button"
          class="view-btn"
          class:active={viewMode === 'table'}
          onclick={() => (viewMode = 'table')}
        >
          <Icon name="list" size={18} />
        </button>
        <button
          type="button"
          class="view-btn"
          class:active={viewMode === 'cards'}
          onclick={() => (viewMode = 'cards')}
        >
          <Icon name="grid" size={18} />
        </button>
      </div>
    {/if}

    {#if actions}
      <div class="toolbar-actions">
        {@render actions()}
      </div>
    {/if}
  </div>

  {#if filterPanelOpen}
    <div class="filter-panel">
      <div class="filter-panel-body">
        {#if advancedFilters}
          {@render advancedFilters()}
        {/if}
      </div>
      {#if onReset}
        <div class="filter-panel-footer">
          <button class="reset-link" type="button" onclick={onReset}>Reset filter</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .toolbar-shell {
    display: grid;
    gap: 0.85rem;
  }

  .toolbar-main {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) auto auto auto;
    gap: 0.75rem;
    align-items: center;
  }

  .search-input-wrapper,
  .filter-toggle {
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .search-input-wrapper {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.48rem 0.68rem;
  }

  .search-icon {
    color: var(--text-secondary);
    display: flex;
    align-items: center;
  }

  .search-input-wrapper input {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    font-size: 0.9rem;
    outline: none;
    padding: 0;
  }

  .search-input-wrapper:focus-within,
  .filter-toggle.open {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .clear-btn {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex: 0 0 auto;
  }

  .clear-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .filter-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.85rem;
    cursor: pointer;
    font-weight: 600;
    white-space: nowrap;
  }

  .view-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.24rem;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .view-btn {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    padding: 0;
  }

  .view-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .view-btn.active {
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
    color: var(--text-primary);
  }

  .toolbar-actions {
    display: flex;
    gap: 0.65rem;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .toolbar-actions :global(.btn),
  .toolbar-actions :global(.btn-primary),
  .toolbar-actions :global(.btn-secondary) {
    min-height: 40px;
  }

  .filter-panel {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: var(--bg-surface);
    padding: 0.9rem 1rem;
    display: grid;
    gap: 0.85rem;
    box-shadow: var(--shadow-sm);
  }

  .filter-panel-body {
    display: grid;
    gap: 0.8rem;
  }

  .filter-panel-footer {
    display: flex;
    justify-content: flex-end;
  }

  .reset-link {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.86rem;
    font-weight: 600;
  }

  .reset-link:hover {
    color: var(--text-primary);
  }

  @media (max-width: 980px) {
    .toolbar-main {
      grid-template-columns: minmax(220px, 1fr) auto auto;
    }

    .toolbar-actions {
      grid-column: 1 / -1;
      justify-content: flex-start;
    }
  }

  @media (max-width: 760px) {
    .toolbar-main {
      grid-template-columns: 1fr;
    }

    .toolbar-actions :global(.btn),
    .toolbar-actions :global(.btn-primary),
    .toolbar-actions :global(.btn-secondary) {
      width: 100%;
      justify-content: center;
    }

    .filter-toggle {
      justify-content: space-between;
    }
  }
</style>
