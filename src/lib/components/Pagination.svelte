<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import Icon from "./Icon.svelte";
    import Select from "./Select.svelte";

    export let count = 0;
    export let page = 0; // 0-indexed
    export let pageSize = 10;
    export let pageSizeOptions = [5, 10, 25, 50, 100];

    const dispatch = createEventDispatcher();

    $: totalPages = Math.ceil(count / pageSize);
    $: startRow = count === 0 ? 0 : page * pageSize + 1;
    $: endRow = Math.min((page + 1) * pageSize, count);

    function handlePageChange(newPage: number) {
        if (newPage >= 0 && newPage < totalPages) {
            dispatch("change", newPage);
        }
    }

    function handlePageSizeChange(event: CustomEvent) {
        const newSize = parseInt(event.detail, 10);
        dispatch("pageSizeChange", newSize);
        // Reset to first page to avoid out of bounds
        dispatch("change", 0);
    }
</script>

<div class="pagination-container">
    <div class="rows-per-page">
        <label for="page-size">Rows per page:</label>
        <div class="select-width">
            <Select
                value={pageSize}
                options={pageSizeOptions}
                placement="top"
                on:change={handlePageSizeChange}
            />
        </div>
    </div>

    <div class="page-controls">
        <span class="range-text">
            {startRow}-{endRow} of {count}
        </span>
        <div class="nav-buttons">
            <button
                class="icon-btn"
                disabled={page === 0}
                on:click={() => handlePageChange(page - 1)}
                aria-label="Previous page"
            >
                <Icon name="chevron-left" size={20} />
            </button>
            <button
                class="icon-btn"
                disabled={page >= totalPages - 1}
                on:click={() => handlePageChange(page + 1)}
                aria-label="Next page"
            >
                <Icon name="chevron-right" size={20} />
            </button>
        </div>
    </div>
</div>

<style>
    .pagination-container {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        padding: 0.75rem 1rem;
        border-top: 1px solid var(--border-color);
        color: var(--text-secondary);
        font-size: 0.85rem;
        gap: 1.5rem;
        flex-wrap: wrap; /* Wrap on small screens */
    }

    .rows-per-page {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .select-width {
        width: 80px;
    }

    .page-controls {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .nav-buttons {
        display: flex;
        align-items: center;
        gap: 0.25rem;
    }

    .icon-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0.25rem;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    .icon-btn:hover:not(:disabled) {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .icon-btn:disabled {
        opacity: 0.3;
        cursor: not-allowed;
    }

    @media (max-width: 600px) {
        .pagination-container {
            justify-content: space-between;
            gap: 1rem;
        }
    }
</style>
