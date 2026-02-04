<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    searchQuery = $bindable(''),
    placeholder = 'Search...',
    showSearch = true,
    onsearch = undefined,
    onclear = undefined,
    filters,
    actions,
  }: {
    searchQuery?: string;
    placeholder?: string;
    showSearch?: boolean;
    onsearch?: (query: string) => void;
    onclear?: () => void;
    filters?: import('svelte').Snippet;
    actions?: import('svelte').Snippet;
  } = $props();

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    searchQuery = target.value;
    if (onsearch) onsearch(searchQuery);
  }

  function clearSearch() {
    searchQuery = '';
    if (onsearch) onsearch('');
    if (onclear) onclear();
  }
</script>

<div class="table-toolbar">
  <div class="search-section">
    {#if showSearch}
      <div class="search-input-wrapper">
        <span class="search-icon">
          <Icon name="search" size={18} />
        </span>
        <input type="text" bind:value={searchQuery} oninput={handleInput} {placeholder} />
        {#if searchQuery}
          <button class="clear-btn" onclick={clearSearch}>
            <Icon name="x" size={14} />
          </button>
        {/if}
      </div>
    {/if}
    <div class="filters">
      {#if filters}
        {@render filters()}
      {/if}
    </div>
  </div>

  <div class="actions">
    {#if actions}
      {@render actions()}
    {/if}
  </div>
</div>

<style>
  .table-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .search-section {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex: 1;
    min-width: 0;
    flex-wrap: wrap;
  }

  .search-input-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1 1 auto;
    min-width: 240px;
    max-width: 420px;
    min-width: 0;
    padding: 0.55rem 0.75rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    transition: all 0.2s;
  }

  .search-icon {
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
  }

  input {
    width: auto;
    flex: 1 1 auto;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
  }

  .search-input-wrapper:focus-within {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    color: var(--text-primary);
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0.25rem;
    display: flex;
    align-items: center;
    border-radius: 50%;
    flex: 0 0 auto;
  }

  .clear-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .filters {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1 1 auto;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    margin-left: auto;
  }

  @media (max-width: 768px) {
    .table-toolbar {
      flex-direction: column;
      align-items: stretch;
      gap: 1rem;
      justify-content: flex-start;
      margin-bottom: 0.75rem;
    }

    .search-section {
      flex-direction: column;
      align-items: stretch;
      min-width: 0;
    }

    .filters {
      width: 100%;
      justify-content: flex-start;
      flex-direction: column;
      align-items: stretch;
    }

    .search-input-wrapper {
      max-width: none;
      min-width: 0;
      flex: 0 0 auto;
    }

    .actions {
      width: 100%;
      justify-content: stretch;
      margin-left: 0;
    }

    .filters {
      flex: 0 0 auto;
    }

    .actions :global(.btn),
    .actions :global(.btn-primary),
    .actions :global(.btn-secondary),
    .actions :global(a.btn) {
      width: 100%;
      justify-content: center;
    }
  }

  @media (max-width: 420px) {
    .search-input-wrapper {
      padding: 0.5rem 0.65rem;
    }
  }
</style>
