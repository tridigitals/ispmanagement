<script lang="ts">
    import { page } from "$app/stores";
    import { user } from "$lib/stores/auth";
    import { isSidebarCollapsed } from "$lib/stores/ui";
    import { t } from "svelte-i18n";
    import Icon from "./Icon.svelte";
    import NotificationDropdown from "./NotificationDropdown.svelte";

    let { onMobileMenuClick }: { onMobileMenuClick: () => void } = $props();

    // Helper to get page title based on path
    function getPageTitle(path: string) {
        if (path.includes("/dashboard")) return $t("topbar.titles.dashboard");
        if (path.includes("/profile")) return $t("topbar.titles.profile");
        if (path.includes("/admin/users"))
            return $t("topbar.titles.user_management");
        if (path.includes("/admin/settings"))
            return $t("topbar.titles.global_settings");
        if (path.includes("/admin")) return $t("topbar.titles.admin_overview");
        return "SaaS App";
    }

    let title = $derived(getPageTitle($page.url.pathname));
</script>

<header class="topbar">
    <div class="left-section">
        <!-- Desktop Sidebar Toggle -->
        <button
            class="icon-btn toggle-btn hide-mobile"
            onclick={() => ($isSidebarCollapsed = !$isSidebarCollapsed)}
        >
            <Icon name="sidebar-toggle" size={20} />
        </button>
        <!-- Mobile Menu Toggle -->
        <button
            class="icon-btn toggle-btn hide-desktop"
            onclick={onMobileMenuClick}
        >
            <Icon name="menu" size={20} />
        </button>
        <h2 class="page-title">{title}</h2>
    </div>

    <div class="right-section">
        <!-- Optional: Search Bar -->
        <div class="search-bar hide-mobile">
            <Icon name="search" size={16} />
            <input type="text" placeholder={$t("topbar.search_placeholder")} />
        </div>

        <!-- Actions -->
        <!-- Notification Dropdown -->
        <NotificationDropdown />

        <button class="icon-btn" title={$t("topbar.help")}>
            <Icon name="help-circle" size={18} />
        </button>
    </div>
</header>

<style>
    .topbar {
        height: 64px;
        background: var(--bg-primary);
        border-bottom: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 2rem;
        flex-shrink: 0;
        z-index: 40;
    }

    .left-section {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .page-title {
        font-size: 1.1rem;
        font-weight: 700;
        color: var(--text-primary);
        margin: 0;
    }

    .right-section {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    /* Search Bar */
    .search-bar {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        background: var(--bg-tertiary);
        border: 1px solid transparent;
        padding: 0.4rem 0.8rem;
        border-radius: 6px;
        transition: all 0.2s;
        margin-right: 1rem;
    }

    .search-bar:focus-within {
        border-color: var(--color-primary);
        background: var(--bg-primary);
    }

    .search-bar input {
        background: transparent;
        border: none;
        outline: none;
        color: var(--text-primary);
        font-size: 0.9rem;
        width: 150px;
    }

    /* Icon Buttons */
    .icon-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 6px;
        cursor: pointer;
        position: relative;
        transition: all 0.2s;
    }

    .icon-btn:hover {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }
</style>
