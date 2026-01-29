<script lang="ts">
    import { user, isSuperAdmin, token } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    import { fade } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import { isSidebarCollapsed } from "$lib/stores/ui";

    let authorized = $state(false);
    let isCollapsed = $state(false);
    let { children } = $props();

    // Strict auth check
    onMount(() => {
        // Sync collapse state to shared store and auto-collapse on mobile
        const isMobile = window.innerWidth < 900;
        isCollapsed = isMobile || $isSidebarCollapsed;
        $isSidebarCollapsed = isCollapsed;

        // 1. Check if logged in at all
        if (!$token) {
            goto("/login");
            return;
        }

        // 2. Check permission
        // We might need to wait for user store to hydrate if it's coming from localStorage
        const check = setInterval(() => {
            if ($user) {
                clearInterval(check);
                if ($isSuperAdmin) {
                    authorized = true;
                } else {
                    // Logged in but not super admin
                    goto("/dashboard");
                }
            }
        }, 50);

        // Timeout backup (if user store never populates)
        setTimeout(() => {
            clearInterval(check);
            if (!authorized) goto("/login");
        }, 3000);

        return () => clearInterval(check);
    });

    function handleNavClick() {
        if (window.innerWidth < 900) {
            isCollapsed = true;
            $isSidebarCollapsed = true;
        }
    }

    function toggleSidebar() {
        const isDesktop = window.innerWidth >= 900;
        if (!isDesktop) {
            isCollapsed = !isCollapsed;
            $isSidebarCollapsed = isCollapsed;
            return;
        }
        isCollapsed = !isCollapsed;
        $isSidebarCollapsed = isCollapsed;
    }
</script>

{#if authorized}
    <div class="sa-layout">
        <!-- Mobile Overlay -->
        {#if !isCollapsed && window.innerWidth < 900}
            <button
                class="mobile-overlay"
                onclick={() => (isCollapsed = true)}
                onkeydown={(e) => {
                    if (e.key === "Enter" || e.key === " ") isCollapsed = true;
                }}
                tabindex="0"
                aria-label="Close menu"
                in:fade={{ duration: 200 }}
            ></button>
        {/if}

        <!-- Super Admin Sidebar -->
        <aside class="sa-sidebar" class:collapsed={isCollapsed}>
            <div class="logo-area">
                <div class="logo-icon">
                    <Icon name="server" size={24} />
                </div>
                {#if !isCollapsed}
                    <span class="logo-text" in:fade>SuperAdmin</span>
                {/if}
            </div>
            <nav>
                <a
                    href="/superadmin"
                    class="nav-item"
                    class:active={$page.url.pathname === "/superadmin"}
                    title="Dashboard"
                    onclick={handleNavClick}
                >
                    <Icon name="grid" size={20} />
                    {#if !isCollapsed}<span in:fade>Dashboard</span>{/if}
                </a>
                <a
                    href="/superadmin/users"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/users",
                    )}
                    title="Global Users"
                    onclick={handleNavClick}
                >
                    <Icon name="users" size={20} />
                    {#if !isCollapsed}<span in:fade>Users</span>{/if}
                </a>
                <a
                    href="/superadmin/plans"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/plans",
                    )}
                    title="Subscription Plans"
                    onclick={handleNavClick}
                >
                    <Icon name="credit-card" size={20} />
                    {#if !isCollapsed}<span in:fade>Plans</span>{/if}
                </a>
                <a
                    href="/superadmin/invoices"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/invoices",
                    )}
                    title="Invoices & Payments"
                    onclick={handleNavClick}
                >
                    <Icon name="credit-card" size={20} />
                    {#if !isCollapsed}<span in:fade>Invoices</span>{/if}
                </a>
                <a
                    href="/superadmin/storage"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/storage",
                    )}
                    title="Storage Manager"
                    onclick={handleNavClick}
                >
                    <Icon name="folder" size={20} />
                    {#if !isCollapsed}<span in:fade>Storage</span>{/if}
                </a>
                <a
                    href="/superadmin/audit-logs"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/audit-logs",
                    )}
                    title="Audit Logs"
                    onclick={handleNavClick}
                >
                    <Icon name="activity" size={20} />
                    {#if !isCollapsed}<span in:fade>Audit Logs</span>{/if}
                </a>
                <a
                    href="/superadmin/settings"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/settings",
                    )}
                    title="Platform Settings"
                    onclick={handleNavClick}
                >
                    <Icon name="settings" size={20} />
                    {#if !isCollapsed}<span in:fade>Settings</span>{/if}
                </a>
                <a
                    href="/superadmin/system"
                    class="nav-item"
                    class:active={$page.url.pathname.startsWith(
                        "/superadmin/system",
                    )}
                    title="System Health"
                    onclick={handleNavClick}
                >
                    <Icon name="server" size={20} />
                    {#if !isCollapsed}<span in:fade>System</span>{/if}
                </a>
                <div class="spacer"></div>
                <a
                    href="/dashboard"
                    class="nav-item back"
                    title="Back to App"
                    onclick={handleNavClick}
                >
                    <Icon name="arrow-left" size={20} />
                    {#if !isCollapsed}<span in:fade>Exit</span>{/if}
                </a>
            </nav>
        </aside>

        <div class="main-wrapper">
            <!-- Topbar -->
            <header class="sa-topbar">
                <div class="topbar-left">
                    <button class="hamburger-btn" onclick={toggleSidebar} title={$isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}>
                        <Icon name="menu" size={20} />
                    </button>
                    <div class="breadcrumb">
                        <span class="root">Super Admin</span>
                        <span class="sep">/</span>
                        <span class="current">
                            {#if $page.url.pathname === "/superadmin"}
                                Dashboard
                            {:else if $page.url.pathname.includes("/users")}
                                Users
                            {:else if $page.url.pathname.includes("/audit-logs")}
                                Audit Logs
                            {:else if $page.url.pathname.includes("/settings")}
                                Settings
                            {:else if $page.url.pathname.includes("/system")}
                                System
                            {:else if $page.url.pathname.includes("/plans")}
                                Plans
                            {:else if $page.url.pathname.includes("/tenants")}
                                Tenants
                            {:else}
                                {$page.url.pathname.split("/").pop()}
                            {/if}
                        </span>
                    </div>
                </div>

                <div class="actions">
                    <div class="search-box hide-mobile">
                        <Icon name="search" size={16} />
                        <input type="text" placeholder="Search..." />
                    </div>

                    <div class="profile-pill">
                        <div class="avatar">
                            {$user?.name?.charAt(0) || "A"}
                        </div>
                        <span class="role hide-mobile">ROOT</span>
                    </div>
                </div>
            </header>

            <!-- Content -->
            <main class="sa-content" in:fade>
                {@render children()}
            </main>
        </div>
    </div>
{:else}
    <!-- Loading state while checking auth -->
    <div class="auth-checking">
        <div class="spinner"></div>
    </div>
{/if}

<style>
    .auth-checking {
        height: 100vh;
        width: 100vw;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--bg-app);
    }
    .spinner {
        width: 40px;
        height: 40px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }
    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .sa-layout {
        display: flex;
        min-height: 100vh;
        background: var(--bg-app);
        color: var(--text-primary);
        font-family: var(--font-family);
    }

    /* Mobile Overlay */
    .mobile-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        backdrop-filter: blur(2px);
        z-index: 45;
        display: none;
        border: none;
        cursor: pointer;
        padding: 0;
    }

    /* Sidebar */
    .sa-sidebar {
        width: 240px;
        background: var(--bg-app);
        border-right: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 12px;
        z-index: 50;
        transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        height: 100vh;
        position: sticky;
        top: 0;
    }

    .sa-sidebar.collapsed {
        width: 72px;
    }

    .logo-area {
        margin-bottom: 2rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0 1rem;
        width: 100%;
        overflow: hidden;
        white-space: nowrap;
        height: 40px;
    }

    .logo-icon {
        color: var(--color-primary);
        min-width: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .sa-sidebar.collapsed .logo-area {
        justify-content: center;
        padding: 0;
    }

    .logo-text {
        font-weight: 700;
        font-size: 1.1rem;
        color: var(--text-primary);
    }

    .sa-sidebar nav {
        display: flex;
        flex-direction: column;
        gap: 8px;
        width: 100%;
        align-items: center;
        flex: 1;
        padding: 0;
    }

    .nav-item {
        width: 100%;
        height: 44px;
        display: flex;
        align-items: center;
        padding: 8px 12px;
        gap: 10px;
        border-radius: var(--radius-sm);
        color: var(--text-secondary);
        transition: all 0.2s;
        text-decoration: none;
        border: none;
        background: transparent;
        cursor: pointer;
        font-size: 0.95rem;
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
    }

    .sa-sidebar.collapsed .nav-item {
        width: 44px;
        padding: 8px;
        justify-content: center;
    }

    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--bg-active);
        color: var(--text-primary);
    }

    .nav-item.back {
        color: var(--color-danger);
        margin-top: auto;
    }
    .nav-item.back:hover {
        background: rgba(239, 68, 68, 0.1);
    }

    .spacer {
        flex: 1;
    }

    /* Main Wrapper */
    .main-wrapper {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-width: 0; /* Prevent flex blowout */
    }

    /* Topbar */
    .sa-topbar {
        height: 64px;
        background: var(--bg-primary);
        border-bottom: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 clamp(12px, 4vw, 32px);
        position: sticky;
        top: 0;
        z-index: 40;
    }

    .topbar-left {
        display: flex;
        align-items: center;
        gap: 1.5rem;
    }

    .hamburger-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0.5rem;
        border-radius: var(--radius-md);
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    .hamburger-btn:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .breadcrumb {
        font-size: 0.9rem;
        font-weight: 500;
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .breadcrumb .current {
        color: var(--text-primary);
    }
    .breadcrumb .root {
        font-weight: 700;
        letter-spacing: 0.05em;
        color: var(--color-primary);
        text-transform: uppercase;
        font-size: 0.8rem;
    }

    .actions {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .profile-pill {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.25rem 0.75rem;
        background: var(--bg-surface);
        border-radius: 30px;
        border: 1px solid var(--border-color);
    }

    .profile-pill .avatar {
        width: 28px;
        height: 28px;
        background: linear-gradient(135deg, var(--color-primary), #ec4899);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        font-size: 0.8rem;
        color: white;
    }

    .profile-pill .role {
        font-size: 0.7rem;
        font-weight: 800;
        color: var(--text-secondary);
        letter-spacing: 0.05em;
    }

    /* Content */
    .sa-content {
        flex: 1;
        overflow-y: auto;
        padding: 2.5rem;
    }

    /* Responsive Design */
    @media (max-width: 768px) {
        .sa-sidebar {
            position: fixed;
            left: 0;
            top: 0;
            bottom: 0;
            transform: translateX(-100%);
            width: var(
                --sidebar-width
            ) !important; /* Full width menu on mobile or standard width */
        }

        .sa-sidebar:not(.collapsed) {
            transform: translateX(0);
        }

        .sa-sidebar.collapsed {
            width: var(
                --sidebar-width
            ); /* Override the 72px collapsed width on mobile */
        }

        .mobile-overlay {
            display: block;
        }

        .sa-topbar {
            padding: 0 1rem;
        }

        .sa-content {
            padding: 1rem;
        }

        /* Adjust internal sidebar elements for mobile when "not collapsed" (which means open) */
        .sa-sidebar .logo-area,
        .sa-sidebar .nav-item span {
            display: flex; /* Ensure these are visible when menu is open */
        }
    }
</style>
