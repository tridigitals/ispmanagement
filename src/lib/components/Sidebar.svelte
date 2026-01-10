<script lang="ts">
    import { page } from "$app/stores";
    import {
        user,
        isAdmin,
        isSuperAdmin,
        logout,
        can,
        authVersion,
    } from "$lib/stores/auth";
    import { appName } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { isSidebarCollapsed } from "$lib/stores/ui";
    import { goto } from "$app/navigation";
    import { t } from "svelte-i18n";
    import { convertFileSrc } from "@tauri-apps/api/core";
    import Icon from "./Icon.svelte";

    // Mobile menu state
    export let isMobileOpen = false;
    export function openMobileMenu() {
        isMobileOpen = true;
    }
    export function closeMobileMenu() {
        isMobileOpen = false;
    }

    $: appMenu = [
        {
            label: $t("sidebar.dashboard"),
            icon: "dashboard",
            href: "/dashboard",
        },
    ];

    // Add $user as explicit dependency to force reactivity when user permissions change
    $: adminMenu = (() => {
        // Access $user to create dependency
        const _ = $user?.permissions;
        return [
            {
                label: $t("sidebar.overview"),
                icon: "shield",
                href: "/admin",
                show: true,
            },
            {
                label: $t("sidebar.team"),
                icon: "users",
                href: "/admin/team",
                show: $can("read", "team"),
            },
            {
                label: $t("sidebar.roles"),
                icon: "lock",
                href: "/admin/roles",
                show: $can("read", "roles"),
            },
            {
                label: $t("sidebar.settings"),
                icon: "settings",
                href: "/admin/settings",
                show: $can("read", "settings"),
            },
        ].filter((i) => i.show);
    })();

    let isDropdownOpen = false;

    $: isUrlAdmin = $page.url.pathname.startsWith("/admin");
    $: currentMenu = isUrlAdmin ? adminMenu : appMenu;

    function handleLogout() {
        logout();
        goto("/login");
    }

    function navigate(href: string) {
        goto(href);
        isDropdownOpen = false;
        isMobileOpen = false; // Close mobile menu on navigate
    }

    function toggleDropdown(event: MouseEvent) {
        event.stopPropagation();
        isDropdownOpen = !isDropdownOpen;
    }

    function handleWindowClick() {
        if (isDropdownOpen) isDropdownOpen = false;
    }

    function isActive(item: { href: string }) {
        return $page.url.pathname === item.href;
    }
</script>

<svelte:window on:click={handleWindowClick} />

<!-- Mobile Overlay Backdrop -->
{#if isMobileOpen}
    <div
        class="sidebar-overlay"
        on:click={closeMobileMenu}
        on:keydown={(e) => e.key === "Escape" && closeMobileMenu()}
        role="button"
        tabindex="0"
        aria-label="Close menu"
    ></div>
{/if}

<aside
    class="sidebar"
    class:collapsed={$isSidebarCollapsed}
    class:mobile-open={isMobileOpen}
>
    <!-- Header -->
    <div class="sidebar-header">
        <div class="logo-wrapper">
            {#if $appLogo}
                <img src={$appLogo} alt="Logo" class="app-logo" />
            {:else}
                <div class="logo-placeholder">
                    <Icon name="app" size={20} />
                </div>
            {/if}
            <span class="app-name">{$appName}</span>
        </div>
    </div>

    <!-- Navigation - use {#key} to force re-render when authVersion changes -->
    {#key $authVersion}
        <nav class="sidebar-nav">
            {#each currentMenu as item}
                <button
                    class="nav-item"
                    class:active={isActive(item)}
                    on:click={() => navigate(item.href)}
                >
                    <Icon name={item.icon} size={18} />
                    <span class="label">{item.label}</span>
                </button>
            {/each}
        </nav>
    {/key}

    <!-- Footer / Profile -->
    <div class="sidebar-footer">
        {#if $isSuperAdmin}
            <button
                class="context-btn super-admin"
                on:click={() => goto("/superadmin")}
                title="Super Admin Dashboard"
            >
                <Icon name="server" size={16} />
                <span class="label">Super Admin</span>
            </button>
        {/if}

        {#if $isAdmin}
            <button
                class="context-btn"
                on:click={() => goto(isUrlAdmin ? "/dashboard" : "/admin")}
                title={isUrlAdmin
                    ? "Switch to User Dashboard"
                    : "Switch to Admin Panel"}
            >
                <Icon name={isUrlAdmin ? "arrow-left" : "shield"} size={16} />
                <span class="label"
                    >{isUrlAdmin
                        ? $t("sidebar.user_view")
                        : $t("sidebar.admin_panel")}</span
                >
            </button>
        {/if}

        <div class="profile-section">
            {#if isDropdownOpen}
                <div
                    class="dropdown-menu"
                    on:click|stopPropagation
                    on:keydown|stopPropagation
                    role="menu"
                    tabindex="-1"
                >
                    <button
                        class="menu-item"
                        on:click={() => navigate("/profile")}
                    >
                        <Icon name="profile" size={16} />
                        {$t("sidebar.profile")}
                    </button>
                    <div class="divider"></div>
                    <button class="menu-item danger" on:click={handleLogout}>
                        <Icon name="logout" size={16} />
                        {$t("sidebar.logout")}
                    </button>
                </div>
            {/if}

            <button class="profile-btn" on:click={toggleDropdown}>
                <div class="avatar">
                    {$user?.name?.charAt(0).toUpperCase() || "?"}
                </div>
                <div class="user-meta">
                    <span class="name">{$user?.name}</span>
                    <span class="role">{$user?.role}</span>
                </div>
                <Icon name="chevron-up" size={14} class="chevron" />
            </button>
        </div>
    </div>
</aside>

<style>
    .sidebar {
        width: 240px; /* Sedikit lebih ramping */
        display: flex;
        flex-direction: column;
        padding: 12px;
        color: var(--text-secondary);
        transition:
            transform 0.3s ease,
            width 0.3s ease;
        background: var(--bg-app); /* Ensure background is solid for overlay */
        border-right: 1px solid var(--border-color);
        height: 100vh;

        /* Mobile defaults */
        position: fixed;
        left: 0;
        top: 0;
        z-index: 50;
        transform: translateX(-100%);
    }

    /* Desktop styles */
    @media (min-width: 768px) {
        .sidebar {
            position: sticky;
            transform: none;
            height: 100vh;
        }
    }

    .sidebar.mobile-open {
        transform: translateX(0);
        box-shadow: var(--shadow-md);
    }

    .sidebar.collapsed {
        width: 72px;
    }

    /* Mobile Overlay */
    .sidebar-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        z-index: 40;
        backdrop-filter: blur(2px);
        animation: fadeIn 0.2s ease-out;
    }

    @keyframes fadeIn {
        from {
            opacity: 0;
        }
        to {
            opacity: 1;
        }
    }

    /* Header */
    .sidebar-header {
        padding: 8px 12px;
        margin-bottom: 24px;
        white-space: nowrap;
        overflow: hidden;
    }

    .logo-wrapper {
        display: flex;
        align-items: center;
        gap: 10px;
        font-weight: 600;
        color: var(--text-primary);
        transition: justify-content 0.3s;
    }

    .sidebar.collapsed .logo-wrapper {
        justify-content: center;
    }

    .app-logo {
        width: 24px;
        height: 24px;
        object-fit: contain;
    }

    .logo-placeholder {
        width: 24px;
        height: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--color-primary);
        color: white;
        border-radius: 6px;
        flex-shrink: 0;
    }

    .app-name {
        font-size: 0.95rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        transition:
            opacity 0.2s,
            width 0.2s;
    }

    .sidebar.collapsed .app-name {
        display: none;
    }

    /* Nav */
    .sidebar-nav {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 12px;
        border-radius: var(--radius-sm);
        border: none;
        background: transparent;
        color: var(--text-secondary);
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.15s ease;
        text-align: left;
        white-space: nowrap;
    }

    .sidebar.collapsed .nav-item {
        justify-content: center;
        padding: 8px;
    }

    .sidebar.collapsed .nav-item .label {
        display: none;
    }

    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--bg-active);
        color: var(--text-primary);
    }

    /* Footer */
    .sidebar-footer {
        display: flex;
        flex-direction: column;
        gap: 8px;
        margin-top: auto;
    }

    .context-btn {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 12px;
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--color-primary);
        background: var(--color-primary-subtle);
        border: 1px solid transparent;
        border-radius: var(--radius-sm);
        cursor: pointer;
        transition: all 0.2s;
        white-space: nowrap;
    }

    .sidebar.collapsed .context-btn {
        justify-content: center;
        padding: 8px;
    }

    .sidebar.collapsed .context-btn .label {
        display: none;
    }

    .context-btn:hover {
        border-color: var(--color-primary);
    }

    .context-btn.super-admin {
        color: white;
        background: linear-gradient(
            135deg,
            #4f46e5,
            #ec4899
        ); /* Indigo to Pink */
        border: none;
    }

    .context-btn.super-admin:hover {
        filter: brightness(1.1);
        transform: translateY(-1px);
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
    }

    .profile-section {
        position: relative;
    }

    .profile-btn {
        display: flex;
        align-items: center;
        gap: 10px;
        width: 100%;
        padding: 8px;
        background: transparent;
        border: 1px solid transparent;
        border-radius: var(--radius-sm);
        cursor: pointer;
        transition: all 0.2s;
    }

    .sidebar.collapsed .profile-btn {
        justify-content: center;
    }

    .sidebar.collapsed .user-meta {
        display: none;
    }

    .profile-btn:hover {
        background: var(--bg-hover);
    }

    .avatar {
        width: 28px;
        height: 28px;
        border-radius: 6px;
        background: var(--bg-active);
        color: var(--text-primary);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.8rem;
        font-weight: 700;
        flex-shrink: 0;
    }

    .user-meta {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        overflow: hidden;
    }

    .name {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-primary);
    }
    .role {
        font-size: 0.7rem;
        color: var(--text-secondary);
        text-transform: capitalize;
    }

    /* Dropdown */
    .dropdown-menu {
        position: absolute;
        bottom: 100%;
        left: 0;
        right: 0;
        margin-bottom: 8px;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        padding: 4px;
        box-shadow: var(--shadow-md);
        display: flex;
        flex-direction: column;
        z-index: 100;
        animation: slideUp 0.2s ease-out;
        width: max-content;
        min-width: 100%;
    }

    .sidebar.collapsed .dropdown-menu {
        left: 100%;
        bottom: 0;
        margin-left: 8px;
        margin-bottom: 0;
    }

    .menu-item {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px 12px;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        font-size: 0.9rem;
        border-radius: var(--radius-sm);
        cursor: pointer;
        text-align: left;
    }

    .menu-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }
    .menu-item.danger:hover {
        color: var(--color-danger);
        background: rgba(239, 68, 68, 0.1);
    }
    .divider {
        height: 1px;
        background: var(--border-color);
        margin: 4px 0;
    }

    @keyframes slideUp {
        from {
            opacity: 0;
            transform: translateY(5px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
</style>
