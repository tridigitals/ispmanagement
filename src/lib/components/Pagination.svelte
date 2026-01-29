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
        justify-content: space-between;
        padding: 0.9rem 1rem;
        border-top: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.015);
        border-bottom-left-radius: var(--radius-md, 12px);
        border-bottom-right-radius: var(--radius-md, 12px);
        color: var(--text-secondary);
        font-size: 0.85rem;
        gap: 1rem;
        flex-wrap: wrap; /* Wrap on small screens */
    }

    :global([data-theme="light"]) .pagination-container {
        border-top-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.015);
    }

    .rows-per-page {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .rows-per-page label {
        white-space: nowrap;
        opacity: 0.85;
    }

    .select-width {
        width: 92px;
    }

    .page-controls {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .range-text {
        white-space: nowrap;
    }

    .nav-buttons {
        display: flex;
        align-items: center;
        gap: 0.25rem;
    }

    .icon-btn {
        width: 36px;
        height: 36px;
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    .icon-btn:hover:not(:disabled) {
        background: rgba(99, 102, 241, 0.12);
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.35);
    }

    .icon-btn:disabled {
        opacity: 0.3;
        cursor: not-allowed;
    }

    @media (max-width: 768px) {
        .pagination-container {
            flex-direction: column;
            align-items: stretch;
            gap: 1rem;
            padding: 0.9rem 1rem;
        }

        .rows-per-page {
            width: 100%;
            justify-content: space-between;
        }

        .page-controls {
            width: 100%;
            justify-content: space-between;
        }

        .nav-buttons {
            justify-content: flex-end;
        }
    }

    @media (max-width: 420px) {
        .pagination-container {
            padding: 0.75rem 0.85rem;
            gap: 0.75rem;
        }

        .rows-per-page label {
            font-size: 0.8rem;
        }

        .select-width {
            width: 86px;
        }

        .page-controls {
            gap: 0.75rem;
        }

        .range-text {
            font-size: 0.8rem;
        }
    }
</style>
