<script lang="ts">
  import Icon from './Icon.svelte';
  import { fade } from 'svelte/transition';
  import Pagination from './Pagination.svelte';
  import { t } from 'svelte-i18n';

  let {
    columns = [],
    data = [],
    keyField = 'id',
    loading = false,
    emptyText = '',
    mobileView = 'card',
    pagination = false,
    pageSize = 10,
    pageSizeOptions = [5, 10, 25, 50, 100],
    count = 0,
    onchange,
    onpageSizeChange,
    searchable = false,
    searchPlaceholder = '',
    children,
    empty,
    cell,
    serverSide = false,
  }: {
    columns?: {
      key: string;
      label: string;
      class?: string;
      align?: string;
      width?: string;
    }[];
    data?: any[];
    keyField?: string;
    loading?: boolean;
    emptyText?: string;
    mobileView?: 'card' | 'scroll';
    pagination?: boolean;
    pageSize?: number;
    pageSizeOptions?: number[];
    count?: number;
    onchange?: (page: number) => void;
    onpageSizeChange?: (size: number) => void;
    searchable?: boolean;
    searchPlaceholder?: string;
    children?: import('svelte').Snippet;
    empty?: import('svelte').Snippet;
    cell?: import('svelte').Snippet<[any]>; // { item, column, key }
    serverSide?: boolean;
  } = $props();

  let currentPage = $state(0);
  let currentSize = $state(10);
  let searchQuery = $state('');

  // Reset page when data changes length significantly (optional but good UX)
  $effect(() => {
    if (!serverSide && data.length < currentPage * currentSize) {
      currentPage = 0;
    }
  });

  // Reset page when search changes
  $effect(() => {
    if (searchQuery) currentPage = 0;
  });

  // Sync currentSize with pageSize prop
  $effect(() => {
    currentSize = pageSize;
  });

  function handlePageChange(newPage: number) {
    currentPage = newPage;
    if (onchange) onchange(newPage);
  }

  function handlePageSizeChange(newSize: number) {
    currentSize = newSize;
    currentPage = 0;
    if (onpageSizeChange) onpageSizeChange(newSize);
  }

  let filteredData = $derived(
    searchable && searchQuery
      ? data.filter((item: any) =>
          Object.values(item).some((val) =>
            String(val).toLowerCase().includes(searchQuery.toLowerCase()),
          ),
        )
      : data,
  );

  let paginatedData = $derived(
    pagination && !serverSide
      ? filteredData.slice(currentPage * currentSize, (currentPage + 1) * currentSize)
      : filteredData,
  );
</script>

<div class="table-container" class:mobile-scroll={mobileView === 'scroll'}>
  {#if searchable}
    <div class="table-search">
      <div class="search-input-wrapper">
        <Icon name="search" size={18} />
        <input
          type="text"
          placeholder={searchPlaceholder ||
            $t('components.table.search_placeholder') ||
            'Search...'}
          bind:value={searchQuery}
          class="search-input"
        />
        {#if searchQuery}
          <button class="clear-btn" onclick={() => (searchQuery = '')}>
            <Icon name="x" size={14} />
          </button>
        {/if}
      </div>
    </div>
  {/if}
  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{$t('common.loading') || 'Loading...'}</p>
    </div>
  {:else if data.length === 0}
    <div class="empty-state">
      {#if empty}
        {@render empty()}
      {:else}
        <div class="empty-icon">
          <Icon name="search" size={32} />
        </div>
        <p>
          {emptyText || $t('components.table.empty') || 'No data found'}
        </p>
      {/if}
    </div>
  {:else}
    <table class="responsive-table" class:mobile-card={mobileView === 'card'}>
      <thead>
        <tr>
          {#each columns as col}
            <th
              class={col.class || ''}
              style:text-align={col.align || 'left'}
              style:width={col.width}
            >
              {col.label}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each paginatedData as item (item[keyField] || Math.random())}
          <tr transition:fade={{ duration: 200 }}>
            {#each columns as col}
              <td
                data-label={col.label}
                class={col.class || ''}
                style:text-align={col.align || 'left'}
              >
                {#if cell}
                  {@render cell({
                    item,
                    column: col, // Fix scope: col -> column
                    key: col.key,
                  })}
                {:else}
                  <!-- Default text renderer -->
                  {item[col.key] ?? ''}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#if pagination && data.length > 0}
    <Pagination
      count={count || data.length}
      page={currentPage}
      pageSize={currentSize}
      {pageSizeOptions}
      onchange={handlePageChange}
      onpageSizeChange={handlePageSizeChange}
    />
  {/if}
</div>

<style>
  .table-container {
    width: 100%;
    overflow-x: auto; /* Allow scroll by default on container */
  }

  .mobile-scroll {
    -webkit-overflow-scrolling: touch;
  }

  .responsive-table {
    width: 100%;
    border-collapse: collapse;
    color: var(--text-primary);
  }

  .responsive-table th {
    text-align: left;
    padding: 1rem;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    font-weight: 600;
    border-bottom: 1px solid var(--border-color);
    white-space: nowrap;
  }

  .responsive-table td {
    padding: 1rem;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: middle;
    font-size: 0.95rem;
  }

  .responsive-table tr:last-child td {
    border-bottom: none;
  }

  .responsive-table tr:hover td {
    background: var(--bg-hover);
  }

  /* Zebra Striping */
  .responsive-table tbody tr:nth-child(even) td {
    background: rgba(255, 255, 255, 0.02);
  }

  /* Ensure hover stays distinct */
  .responsive-table tbody tr:hover td {
    background: var(--bg-hover);
  }

  /* Loading & Empty States */
  .loading-state,
  .empty-state {
    padding: 3rem;
    text-align: center;
    color: var(--text-secondary);
    background: var(--bg-surface);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    margin: 0 auto 1rem;
    animation: spin 1s linear infinite;
  }

  .empty-icon {
    margin-bottom: 0.5rem;
    opacity: 0.5;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* =========================================
       Mobile Responsive Card Transformation
       ========================================= */
  @media (max-width: 1024px) {
    .responsive-table.mobile-card,
    .responsive-table.mobile-card tbody,
    .responsive-table.mobile-card tr,
    .responsive-table.mobile-card td {
      display: block;
      width: 100%;
    }

    .responsive-table.mobile-card thead {
      display: none;
    }

    .responsive-table.mobile-card tr {
      margin-bottom: 1rem;
      border: 1px solid var(--border-color);
      border-radius: var(--radius-md);
      background: var(--bg-surface);
      box-shadow: var(--shadow-sm);
      overflow: hidden;
    }

    .responsive-table.mobile-card td {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--border-subtle);
      text-align: right !important; /* Force content to right */
      min-height: 3rem;
      white-space: normal; /* Allow wrap in card mode */
    }

    .responsive-table.mobile-card td:last-child {
      border-bottom: none;
      justify-content: flex-end; /* Usually actions */
      background: var(--bg-active); /* Slight highlight for action row */
    }

    /* Label Pseudo-element */
    .responsive-table.mobile-card td::before {
      content: attr(data-label);
      font-weight: 600;
      color: var(--text-muted);
      font-size: 0.85rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      margin-right: 1.5rem;
      text-align: left;
    }

    /* Special handling for First Column (Primary Identifier like Name) */
    .responsive-table.mobile-card td:first-child {
      background: var(--bg-hover);
      border-bottom: 1px solid var(--border-color);
      font-weight: 600;
      font-size: 1rem;
    }
  }

  .table-search {
    padding: 0 0 1rem 0;
  }

  .search-input-wrapper {
    position: relative;
    width: 100%;
    max-width: 420px;
    display: flex;
    align-items: center;
    background: var(--bg-app, #f1f5f9);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 0.5rem 0.75rem;
    gap: 0.5rem;
    color: var(--text-secondary);
  }

  @media (max-width: 768px) {
    .table-search {
      padding: 0 0 0.75rem 0;
    }

    .search-input-wrapper {
      max-width: none;
    }
  }

  .search-input-wrapper:focus-within {
    border-color: var(--color-primary);
    color: var(--text-primary);
  }

  .search-input {
    border: none;
    background: transparent;
    outline: none;
    width: 100%;
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .clear-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    color: var(--text-secondary);
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }
</style>
