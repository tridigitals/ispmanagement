<script lang="ts">
    import Icon from "./Icon.svelte";
    import { createEventDispatcher } from "svelte";

    export let searchQuery = "";
    export let placeholder = "Search...";
    export let showSearch = true;

    const dispatch = createEventDispatcher();

    function handleInput(e: Event) {
        const target = e.target as HTMLInputElement;
        searchQuery = target.value;
        dispatch("search", searchQuery);
    }

    function clearSearch() {
        searchQuery = "";
        dispatch("search", "");
        dispatch("clear");
    }
</script>

<div class="table-toolbar">
    <div class="search-section">
        {#if showSearch}
            <div class="search-input-wrapper">
                <div class="search-icon">
                    <Icon name="search" size={18} />
                </div>
                <input
                    type="text"
                    bind:value={searchQuery}
                    on:input={handleInput}
                    {placeholder}
                />
                {#if searchQuery}
                    <button class="clear-btn" on:click={clearSearch}>
                        <Icon name="x" size={14} />
                    </button>
                {/if}
            </div>
        {/if}
        <div class="filters">
            <slot name="filters" />
        </div>
    </div>

    <div class="actions">
        <slot name="actions" />
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
        min-width: 300px;
    }

    .search-input-wrapper {
        position: relative;
        flex: 1;
        max-width: 400px;
    }

    .search-icon {
        position: absolute;
        left: 0.75rem;
        top: 50%;
        transform: translateY(-50%);
        color: var(--text-secondary);
        pointer-events: none;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    input {
        width: 100%;
        padding: 0.6rem 1rem 0.6rem 2.5rem;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        background: var(--bg-surface);
        color: var(--text-primary);
        font-size: 0.9rem;
        transition: all 0.2s;
    }

    input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    }

    .clear-btn {
        position: absolute;
        right: 0.5rem;
        top: 50%;
        transform: translateY(-50%);
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0.25rem;
        display: flex;
        align-items: center;
        border-radius: 50%;
    }

    .clear-btn:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .filters {
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .actions {
        display: flex;
        gap: 0.75rem;
    }

    @media (max-width: 768px) {
        .table-toolbar {
            flex-direction: column;
            align-items: stretch;
            gap: 1rem;
        }

        .search-section {
            flex-direction: column;
            align-items: stretch;
            min-width: 100%;
        }

        .search-input-wrapper {
            max-width: 100%;
        }

        .actions {
            justify-content: flex-end;
        }
    }
</style>
