<script lang="ts">
    import TableToolbar from "$lib/components/ui/TableToolbar.svelte";
    import Icon from "$lib/components/ui/Icon.svelte";
    import { t } from "svelte-i18n";

    let {
        searchQuery = $bindable(),
        roleFilter = $bindable(),
        statusFilter = $bindable(),
        viewMode = $bindable(),
        isMobile,
    } = $props<{
        searchQuery: string;
        roleFilter: "all" | "superadmin" | "admin" | "user";
        statusFilter: "all" | "active" | "inactive";
        viewMode: "table" | "cards";
        isMobile: boolean;
    }>();
</script>

<TableToolbar
    bind:searchQuery
    placeholder={$t("superadmin.users.search") || "Search users..."}
>
    {#snippet filters()}
        <div class="filters-row">
            <div class="role-filter">
                <button
                    type="button"
                    class="filter-chip"
                    class:active={roleFilter === "all"}
                    onclick={() => (roleFilter = "all")}
                >
                    {$t("superadmin.users.filters.all_roles") || "All Roles"}
                </button>
                <button
                    type="button"
                    class="filter-chip"
                    class:active={roleFilter === "admin"}
                    onclick={() => (roleFilter = "admin")}
                >
                    {$t("superadmin.users.filters.admin") || "Admin"}
                </button>
                <button
                    type="button"
                    class="filter-chip"
                    class:active={roleFilter === "user"}
                    onclick={() => (roleFilter = "user")}
                >
                    {$t("superadmin.users.filters.user") || "User"}
                </button>
                <button
                    type="button"
                    class="filter-chip"
                    class:active={roleFilter === "superadmin"}
                    onclick={() => (roleFilter = "superadmin")}
                >
                    {$t("superadmin.users.filters.superadmin") || "Super Admin"}
                </button>
            </div>

            <div class="status-filter">
                <button
                    type="button"
                    class="filter-chip"
                    class:active={statusFilter === "all"}
                    onclick={() => (statusFilter = "all")}
                >
                    {$t("superadmin.users.filters.all") || "All"}
                </button>
                <button
                    type="button"
                    class="filter-chip"
                    class:active={statusFilter === "active"}
                    onclick={() => (statusFilter = "active")}
                >
                    {$t("superadmin.users.filters.active") || "Active"}
                </button>
                <button
                    type="button"
                    class="filter-chip"
                    class:active={statusFilter === "inactive"}
                    onclick={() => (statusFilter = "inactive")}
                >
                    {$t("superadmin.users.filters.inactive") || "Inactive"}
                </button>
            </div>
        </div>
    {/snippet}

    {#snippet actions()}
        {#if !isMobile}
            <div
                class="view-toggle"
                aria-label={$t("superadmin.users.view.aria") || "View mode"}
            >
                <button
                    type="button"
                    class="view-btn"
                    class:active={viewMode === "table"}
                    onclick={() => (viewMode = "table")}
                    title={$t("superadmin.users.view.table") || "Table view"}
                    aria-label={$t("superadmin.users.view.table") ||
                        "Table view"}
                >
                    <Icon name="list" size={18} />
                </button>
                <button
                    type="button"
                    class="view-btn"
                    class:active={viewMode === "cards"}
                    onclick={() => (viewMode = "cards")}
                    title={$t("superadmin.users.view.cards") || "Card view"}
                    aria-label={$t("superadmin.users.view.cards") ||
                        "Card view"}
                >
                    <Icon name="grid" size={18} />
                </button>
            </div>
        {/if}
    {/snippet}
</TableToolbar>

<style>
    .filters-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
        align-items: center;
        width: 100%;
    }

    .status-filter,
    .role-filter {
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 12px;
        padding: 0.35rem;
        flex-wrap: wrap;
    }

    :global([data-theme="light"]) .status-filter,
    :global([data-theme="light"]) .role-filter {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .filter-chip {
        border: none;
        background: transparent;
        color: var(--text-secondary);
        padding: 0.45rem 0.75rem;
        border-radius: 10px;
        cursor: pointer;
        font-weight: 650;
        font-size: 0.85rem;
        transition: all 0.2s;
        white-space: nowrap;
    }

    .filter-chip:hover {
        color: var(--text-primary);
        background: rgba(255, 255, 255, 0.05);
    }

    :global([data-theme="light"]) .filter-chip:hover {
        background: rgba(0, 0, 0, 0.04);
    }

    .filter-chip.active {
        background: rgba(99, 102, 241, 0.18);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--text-primary);
    }

    .view-toggle {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
    }

    :global([data-theme="light"]) .view-toggle {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
    }

    .view-btn {
        width: 38px;
        height: 38px;
        border-radius: 10px;
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
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-primary);
    }

    :global([data-theme="light"]) .view-btn:hover {
        background: rgba(0, 0, 0, 0.04);
    }

    .view-btn.active {
        background: rgba(99, 102, 241, 0.18);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--text-primary);
    }
</style>
