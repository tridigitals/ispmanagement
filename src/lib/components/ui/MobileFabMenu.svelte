<script lang="ts">
    import Icon from "./Icon.svelte";
    import { createEventDispatcher } from "svelte";
    import { fade, fly } from "svelte/transition";
    import { t } from "svelte-i18n";

    export let items: Array<{ id: string; label: string; icon: string }> = [];
    export let activeTab: string = "";
    export let title: string = "Menu";

    let isOpen = false;

    const dispatch = createEventDispatcher();

    function selectTab(id: string) {
        activeTab = id;
        dispatch("change", id);
        isOpen = false;
    }
</script>

<div class="mobile-fab-container">
    <button
        class="mobile-fab"
        on:click={() => (isOpen = !isOpen)}
        aria-label={$t("components.mobile_fab.open_menu") || "Open Menu"}
        type="button"
    >
        <Icon name={isOpen ? "x" : "settings"} size={24} />
    </button>

    {#if isOpen}
        <div
            class="mobile-menu-overlay"
            on:click={() => (isOpen = false)}
            on:keydown={(e) => e.key === "Escape" && (isOpen = false)}
            role="button"
            tabindex="0"
            transition:fade={{ duration: 200 }}
        >
            <div
                class="mobile-menu card"
                on:click|stopPropagation
                on:keydown|stopPropagation
                role="dialog"
                aria-modal="true"
                tabindex="-1"
                transition:fly={{ y: 300, duration: 300 }}
            >
                <div class="mobile-menu-header">
                    <h3>{title}</h3>
                    <button
                        class="close-btn"
                        on:click={() => (isOpen = false)}
                        aria-label={$t("common.close") || "Close"}
                        type="button"
                    >
                        <Icon name="x" size={20} />
                    </button>
                </div>
                <nav>
                    {#each items as item}
                        <button
                            class="nav-item {activeTab === item.id
                                ? 'active'
                                : ''}"
                            on:click={() => selectTab(item.id)}
                            type="button"
                        >
                            <span class="icon">
                                <Icon name={item.icon} size={18} />
                            </span>
                            {item.label}
                        </button>
                    {/each}
                </nav>
            </div>
        </div>
    {/if}
</div>

<style>
    .mobile-fab-container {
        display: none;
    }

    @media (max-width: 900px) {
        .mobile-fab-container {
            display: block;
        }
    }

    /* Mobile FAB Styles */
    .mobile-fab {
        position: fixed;
        bottom: 2rem;
        right: 2rem;
        width: 3.5rem;
        height: 3.5rem;
        border-radius: 50%;
        background: var(--color-primary);
        color: white;
        border: none;
        box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 100;
        transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
    }

    .mobile-fab:active {
        transform: scale(0.9);
    }

    /* Mobile Menu Overlay Styles */
    .mobile-menu-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        backdrop-filter: blur(4px);
        z-index: 90;
        display: flex;
        align-items: flex-end;
        justify-content: center;
        padding: 1rem;
    }

    .mobile-menu {
        width: 100%;
        max-width: 500px;
        background: var(--bg-surface);
        border-radius: 1rem 1rem 0 0; /* Bottom sheet style */
        padding: 1.5rem;
        box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.2);
        max-height: 80vh;
        overflow-y: auto;
    }

    .mobile-menu-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1.5rem;
    }

    .mobile-menu-header h3 {
        font-size: 1.1rem;
        font-weight: 600;
        margin: 0;
        color: var(--text-primary);
    }

    .close-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        padding: 0.5rem;
        cursor: pointer;
        border-radius: 50%;
    }

    .close-btn:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        width: 100%;
        padding: 0.875rem 1rem;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.95rem;
        font-weight: 500;
        cursor: pointer;
        border-radius: var(--radius-md);
        transition: all 0.2s;
        text-align: left;
        margin-bottom: 0.25rem;
    }

    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--color-primary);
        color: white;
    }

    .icon {
        display: flex;
        align-items: center;
        justify-content: center;
        opacity: 0.8;
    }

    .nav-item.active .icon {
        opacity: 1;
    }
</style>
