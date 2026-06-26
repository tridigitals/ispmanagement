<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    title,
    countLabel,
    searchQuery = $bindable(''),
    searchPlaceholder = 'Search...',
    primaryFilterValue = $bindable('all'),
    primaryFilterOptions = [],
    primaryFilterAriaLabel = 'Primary filter',
    filterPanelOpen = $bindable(false),
    activeFilterCount = 0,
    onReset,
    advancedFilters,
    actions,
  } = $props<{
    title: string;
    countLabel: string;
    searchQuery?: string;
    searchPlaceholder?: string;
    primaryFilterValue?: string;
    primaryFilterOptions?: Array<{ value: string; label: string }>;
    primaryFilterAriaLabel?: string;
    filterPanelOpen?: boolean;
    activeFilterCount?: number;
    onReset?: () => void;
    advancedFilters?: import('svelte').Snippet;
    actions?: import('svelte').Snippet;
  }>();
</script>

<div class="toolbar-shell">
  <div class="toolbar-meta">
    <h2>{title}</h2>
    <p>{countLabel}</p>
  </div>

  <div class="toolbar-main">
    <div class="search-input-wrapper">
      <span class="search-icon">
        <Icon name="search" size={18} />
      </span>
      <input type="text" bind:value={searchQuery} placeholder={searchPlaceholder} />
      {#if searchQuery}
        <button class="icon-btn clear-btn" type="button" onclick={() => (searchQuery = '')}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </div>

    <div class="primary-select-wrap">
      <select bind:value={primaryFilterValue} aria-label={primaryFilterAriaLabel}>
        {#each primaryFilterOptions as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
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

      <div class="filter-panel-footer">
        <button class="reset-link" type="button" onclick={onReset}>{$t('common.reset')}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .toolbar-shell {
    display: grid;
    gap: 0.9rem;
    margin-bottom: 1rem;
  }

  .toolbar-meta {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-end;
    flex-wrap: wrap;
  }

  .toolbar-meta h2 {
    margin: 0 0 0.2rem;
  }

  .toolbar-meta p {
    margin: 0;
    color: var(--text-secondary);
  }

  .toolbar-main {
    display: grid;
    grid-template-columns: minmax(220px, 1.5fr) minmax(180px, 0.9fr) auto auto;
    gap: 0.75rem;
    align-items: center;
  }

  .search-input-wrapper,
  .primary-select-wrap select,
  .filter-toggle {
    min-height: 42px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-primary);
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
  .primary-select-wrap select:focus,
  .filter-toggle.open {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .primary-select-wrap select {
    width: 100%;
    padding: 0 0.85rem;
    outline: none;
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

  .toolbar-actions {
    display: flex;
    gap: 0.65rem;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .toolbar-actions :global(.btn),
  .toolbar-actions :global(.btn-primary),
  .toolbar-actions :global(.btn-secondary) {
    min-height: 42px;
  }

  .filter-panel {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: var(--bg-surface);
    padding: 0.9rem 1rem;
    display: grid;
    gap: 0.85rem;
  }

  .filter-panel-body {
    display: grid;
    gap: 0.8rem;
  }

  .filter-panel-footer {
    display: flex;
    justify-content: flex-end;
  }

  .reset-link,
  .icon-btn {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .reset-link {
    font-size: 0.86rem;
    font-weight: 600;
  }

  .reset-link:hover,
  .icon-btn:hover {
    color: var(--text-primary);
  }

  .clear-btn {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
  }

  .clear-btn:hover {
    background: var(--bg-hover);
  }

  @media (max-width: 1080px) {
    .toolbar-main {
      grid-template-columns: minmax(220px, 1fr) minmax(180px, 1fr);
    }

    .toolbar-actions {
      grid-column: 1 / -1;
      justify-content: flex-start;
    }
  }

  @media (max-width: 700px) {
    .toolbar-main {
      grid-template-columns: 1fr;
    }

    .toolbar-actions :global(.btn),
    .toolbar-actions :global(.btn-primary),
    .toolbar-actions :global(.btn-secondary) {
      width: 100%;
      justify-content: center;
    }
  }
</style>
