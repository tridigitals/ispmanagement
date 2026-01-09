<script lang="ts">
    import { user, isSuperAdmin, token } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fade } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";

    let authorized = false;
    let isCollapsed = false;

    // Strict auth check
    onMount(() => {
        // 1. Check if logged in at all
        if (!$token) {
            goto('/login');
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
                    goto('/dashboard');
                }
            }
        }, 50);
        
        // Timeout backup (if user store never populates)
        setTimeout(() => {
            clearInterval(check);
            if (!authorized) goto('/login');
        }, 3000);

        return () => clearInterval(check);
    });
</script>

{#if authorized}
    <div class="sa-layout">
        <!-- Super Admin Sidebar (Mini) -->
        <aside class="sa-sidebar" class:collapsed={isCollapsed}>
            <div class="logo-area">
                <Icon name="server" size={24} />
                {#if !isCollapsed}
                    <span class="logo-text">SuperAdmin</span>
                {/if}
            </div>
            <nav>
                <a href="/superadmin" class="nav-item active" title="Dashboard">
                    <Icon name="grid" size={20} />
                    {#if !isCollapsed}<span>Dashboard</span>{/if}
                </a>
                <a href="/superadmin/logs" class="nav-item" title="System Logs">
                    <Icon name="activity" size={20} />
                    {#if !isCollapsed}<span>Logs</span>{/if}
                </a>
                <a href="/superadmin/users" class="nav-item" title="Global Users">
                    <Icon name="users" size={20} />
                    {#if !isCollapsed}<span>Users</span>{/if}
                </a>
                <div class="spacer"></div>
                <a href="/dashboard" class="nav-item back" title="Back to App">
                    <Icon name="arrow-left" size={20} />
                    {#if !isCollapsed}<span>Exit</span>{/if}
                </a>
            </nav>
        </aside>

        <div class="main-wrapper">
            <!-- Topbar -->
            <header class="sa-topbar">
                <div class="topbar-left">
                    <button class="hamburger-btn" on:click={() => isCollapsed = !isCollapsed}>
                        <Icon name="sidebar-toggle" size={20} />
                    </button>
                    <div class="breadcrumb">
                        <span class="root">Super Admin</span>
                        <span class="sep">/</span>
                        <span class="current">Dashboard</span>
                    </div>
                </div>

                <div class="actions">
                    <div class="search-box">
                        <Icon name="search" size={16} />
                        <input type="text" placeholder="Search tenants or users..." />
                    </div>
                    
                    <div class="profile-pill">
                        <div class="avatar">{$user?.name?.charAt(0) || 'A'}</div>
                        <span class="role">ROOT</span>
                    </div>
                </div>
            </header>

            <!-- Content -->
            <main class="sa-content" in:fade>
                <slot />
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
        background: #0f172a;
    }
    .spinner {
        width: 40px;
        height: 40px;
        border: 3px solid rgba(255,255,255,0.1);
        border-top-color: #6366f1;
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }

    .sa-layout {
        display: flex;
        min-height: 100vh;
        background: #0f172a;
        color: #f1f5f9;
        font-family: 'Inter', sans-serif;
    }

    /* Sidebar */
    .sa-sidebar {
        width: 240px;
        background: #1e293b;
        border-right: 1px solid rgba(255,255,255,0.05);
        display: flex;
        flex-direction: column;
        align-items: center; /* Center items when collapsed, but specific alignment handled below */
        padding: 1.5rem 0;
        z-index: 50;
        transition: width 0.3s ease;
    }

    .sa-sidebar.collapsed {
        width: 72px;
    }

    .logo-area {
        color: #6366f1;
        margin-bottom: 2rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0 1.5rem;
        width: 100%;
        justify-content: flex-start;
        overflow: hidden;
        white-space: nowrap;
    }

    .sa-sidebar.collapsed .logo-area {
        justify-content: center;
        padding: 0;
    }

    .logo-text {
        font-weight: 700;
        font-size: 1.1rem;
        color: white;
    }

    .sa-sidebar nav {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        width: 100%;
        align-items: center;
        flex: 1;
        padding: 0 1rem;
    }

    .nav-item {
        width: 100%;
        height: 44px;
        display: flex;
        align-items: center;
        padding: 0 0.75rem;
        gap: 0.75rem;
        border-radius: 10px;
        color: #94a3b8;
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
        padding: 0;
        justify-content: center;
    }

    .nav-item:hover {
        background: rgba(255,255,255,0.05);
        color: white;
    }

    .nav-item.active {
        background: #6366f1;
        color: white;
        box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
    }

    .nav-item.back {
        color: #ef4444;
        margin-top: auto;
    }
    .nav-item.back:hover {
        background: rgba(239, 68, 68, 0.1);
    }
    
    .nav-item.toggle-btn {
        margin-top: auto;
    }

    .spacer { flex: 1; }

    /* Main Wrapper */
    .main-wrapper {
        flex: 1;
        display: flex;
        flex-direction: column;
    }

    /* Topbar */
    .sa-topbar {
        height: 64px;
        background: rgba(15, 23, 42, 0.8);
        backdrop-filter: blur(10px);
        border-bottom: 1px solid rgba(255,255,255,0.05);
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 2rem;
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
        color: #94a3b8;
        cursor: pointer;
        padding: 0.5rem;
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    .hamburger-btn:hover {
        background: rgba(255,255,255,0.05);
        color: white;
    }

    .breadcrumb {
        font-size: 0.9rem;
        font-weight: 500;
        color: #94a3b8;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .breadcrumb .current { color: white; }
    .breadcrumb .root { font-weight: 700; letter-spacing: 0.05em; color: #6366f1; text-transform: uppercase; font-size: 0.8rem; }

    .actions {
        display: flex;
        align-items: center;
        gap: 1.5rem;
    }

    .search-box {
        background: #1e293b;
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 8px;
        padding: 0.4rem 0.8rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: #94a3b8;
    }

    .search-box input {
        background: transparent;
        border: none;
        outline: none;
        color: white;
        font-size: 0.9rem;
        width: 200px;
    }

    .profile-pill {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.25rem;
        padding-right: 0.75rem;
        background: #1e293b;
        border-radius: 30px;
        border: 1px solid rgba(255,255,255,0.05);
    }

    .profile-pill .avatar {
        width: 28px;
        height: 28px;
        background: linear-gradient(135deg, #6366f1, #ec4899);
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
        color: #94a3b8;
        letter-spacing: 0.05em;
    }

    /* Content */
    .sa-content {
        flex: 1;
        overflow-y: auto;
        padding: 2.5rem; /* Added significant padding */
    }
</style>
