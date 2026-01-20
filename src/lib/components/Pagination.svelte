<script lang="ts">
    import Icon from "./Icon.svelte";
    import Select from "./Select.svelte";

    let {
        count = 0,
        page = 0,
        pageSize = 10,
        pageSizeOptions = [5, 10, 25, 50, 100],
        onchange,
        onpageSizeChange,
    } = $props();

    let totalPages = $derived(Math.ceil(count / pageSize));
    let startRow = $derived(count === 0 ? 0 : page * pageSize + 1);
    let endRow = $derived(Math.min((page + 1) * pageSize, count));

    function handlePageChange(newPage: number) {
        if (newPage >= 0 && newPage < totalPages) {
            if (onchange) onchange(newPage);
        }
    }

    function handlePageSizeChange(newSize: number) {
        // detail from Select is the value
        if (onpageSizeChange) onpageSizeChange(newSize);
        if (onchange) onchange(0);
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
                onchange={(e) => handlePageSizeChange(e.detail)}
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
                onclick={() => handlePageChange(page - 1)}
                aria-label="Previous page"
            >
                <Icon name="chevron-left" size={20} />
            </button>
            <button
                class="icon-btn"
                disabled={page >= totalPages - 1}
                onclick={() => handlePageChange(page + 1)}
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
