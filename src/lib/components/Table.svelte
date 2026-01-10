<script lang="ts">
    import Icon from "./Icon.svelte";
    import { fade } from "svelte/transition";
    import Pagination from "./Pagination.svelte";

    export let columns: {
        key: string;
        label: string;
        class?: string;
        align?: "left" | "center" | "right";
        width?: string;
    }[] = [];

    export let data: any[] = [];
    export let keyField = "id";
    export let loading = false;
    export let emptyText = "No data found";
    export let mobileView: "card" | "scroll" = "card";

    // Pagination props
    export let pagination = false;
    export let pageSize = 10;
    export let pageSizeOptions = [5, 10, 25, 50, 100];

    let currentPage = 0;
    let currentSize = pageSize;

    // Reset page when data changes length significantly (optional but good UX)
    $: if (data.length < currentPage * currentSize) {
        currentPage = 0;
    }

    $: paginatedData = pagination
        ? data.slice(currentPage * currentSize, (currentPage + 1) * currentSize)
        : data;

    function handlePageChange(e: CustomEvent<number>) {
        currentPage = e.detail;
    }

    function handlePageSizeChange(e: CustomEvent<number>) {
        currentSize = e.detail;
        currentPage = 0;
    }
</script>

<div class="table-container" class:mobile-scroll={mobileView === "scroll"}>
    {#if loading}
        <div class="loading-state">
            <div class="spinner"></div>
            <p>Loading...</p>
        </div>
    {:else if data.length === 0}
        <div class="empty-state">
            <slot name="empty">
                <div class="empty-icon">
                    <Icon name="search" size={32} />
                </div>
                <p>{emptyText}</p>
            </slot>
        </div>
    {:else}
        <table
            class="responsive-table"
            class:mobile-card={mobileView === "card"}
        >
            <thead>
                <tr>
                    {#each columns as col}
                        <th
                            class={col.class || ""}
                            style:text-align={col.align || "left"}
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
                                class={col.class || ""}
                                style:text-align={col.align || "left"}
                            >
                                <slot
                                    name="cell"
                                    {item}
                                    column={col}
                                    key={col.key}
                                >
                                    <!-- Default text renderer -->
                                    {item[col.key] ?? ""}
                                </slot>
                            </td>
                        {/each}
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}

    {#if pagination && data.length > 0}
        <Pagination
            count={data.length}
            page={currentPage}
            pageSize={currentSize}
            {pageSizeOptions}
            on:change={handlePageChange}
            on:pageSizeChange={handlePageSizeChange}
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
    @media (max-width: 768px) {
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
</style>
