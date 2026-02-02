<script lang="ts">
    import { page } from "$app/stores";
    import { user } from "$lib/stores/auth";
    import { isSidebarCollapsed } from "$lib/stores/ui";
    import { theme } from "$lib/stores/theme";
    import { t } from "svelte-i18n";
    import Icon from "../ui/Icon.svelte";
    import NotificationDropdown from "./NotificationDropdown.svelte";

    let { onMobileMenuClick }: { onMobileMenuClick: () => void } = $props();
    const DESKTOP_BP = 900; // Keep in sync with --bp-lg

    // Helper to get page title based on path (ordered by specificity)
    function getPageTitle(path: string) {
        const map: [string, string][] = [
            [
                "/notifications",
                $t("topbar.titles.notifications") || "Notifications",
            ],
            ["/superadmin/tenants", $t("topbar.titles.tenants") || "Tenants"],
            ["/superadmin/users", $t("topbar.titles.users") || "Users"],
            ["/superadmin/plans", $t("topbar.titles.plans") || "Plans"],
            [
                "/superadmin/invoices",
                $t("topbar.titles.invoices") || "Invoices",
            ],
            ["/superadmin/storage", $t("topbar.titles.storage") || "Storage"],
            [
                "/superadmin/audit-logs",
                $t("topbar.titles.audit_logs") || "Audit Logs",
            ],
            [
                "/superadmin/settings",
                $t("topbar.titles.settings") || "Settings",
            ],
            ["/superadmin/system", $t("topbar.titles.system") || "System"],
            [
                "/superadmin",
                $t("topbar.titles.superadmin_dashboard") || "Super Admin",
            ],
            ["/admin/team", $t("topbar.titles.team") || "Team"],
            ["/admin/roles", $t("topbar.titles.roles") || "Roles"],
            [
                "/admin/settings",
                $t("topbar.titles.global_settings") || "Settings",
            ],
            ["/admin/storage", $t("topbar.titles.storage") || "Storage"],
            [
                "/admin/subscription",
                $t("topbar.titles.subscription") || "Subscription",
            ],
            ["/admin/invoices", $t("topbar.titles.invoices") || "Invoices"],
            ["/admin", $t("topbar.titles.admin_overview") || "Admin"],
            ["/profile", $t("topbar.titles.profile") || "Profile"],
            ["/dashboard", $t("topbar.titles.dashboard") || "Dashboard"],
        ];

        for (const [route, label] of map) {
            if (path.includes(route)) return label;
        }
        return $t("topbar.titles.default") || "SaaS App";
    }

    let title = $derived(getPageTitle($page.url.pathname));

    function handleSidebarToggle() {
        const isDesktop =
            typeof window !== "undefined" && window.innerWidth >= DESKTOP_BP;
        if (!isDesktop) {
            onMobileMenuClick();
            // Always keep desktop state expanded when coming from mobile
            $isSidebarCollapsed = false;
            return;
        }
        $isSidebarCollapsed = !$isSidebarCollapsed;
    }

    let toggleLabel = $derived(
        $isSidebarCollapsed
            ? $t("sidebar.expand") || "Expand sidebar"
            : $t("sidebar.collapse") || "Collapse sidebar",
    );

    // Theme toggle
    function toggleTheme() {
        theme.toggle();
    }

    let themeLabel = $derived(
        $theme === "light"
            ? $t("topbar.toggle_dark") || "Switch to dark mode"
            : $t("topbar.toggle_light") || "Switch to light mode",
    );

    let themeIcon = $derived($theme === "light" ? "moon" : "sun");
</script>

<header class="topbar">
    <div class="left-section">
        <button
            class="icon-btn toggle-btn"
            onclick={handleSidebarToggle}
            title={toggleLabel}
            aria-label={toggleLabel}
            data-tooltip={toggleLabel}
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

        <button
            class="icon-btn"
            aria-label={themeLabel}
            data-tooltip={themeLabel}
            onclick={toggleTheme}
        >
            <Icon name={themeIcon} size={18} />
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
        padding: 0 clamp(12px, 4vw, 32px);
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

    /* Lightweight tooltip for desktop hover */
    @media (min-width: 900px) {
        .icon-btn[data-tooltip] {
            position: relative;
        }

        .icon-btn[data-tooltip]:hover::after {
            content: attr(data-tooltip);
            position: absolute;
            top: calc(100% + 8px);
            left: 50%;
            transform: translateX(-50%);
            padding: 6px 10px;
            background: var(--bg-surface);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
            border-radius: 6px;
            white-space: nowrap;
            box-shadow: var(--shadow-md);
            font-size: 0.85rem;
            z-index: 10;
        }
    }

    @media (max-width: 900px) {
        .topbar {
            padding: 0 clamp(12px, 5vw, 20px);
        }

        .search-bar {
            margin-right: 0;
        }
    }
</style>
