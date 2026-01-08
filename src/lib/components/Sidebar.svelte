<script lang="ts">
    import { page } from '$app/stores';
    import { user, isAdmin, logout } from '$lib/stores/auth';
    import { appName } from '$lib/stores/settings';
    import { appLogo } from '$lib/stores/logo';
    import { isSidebarCollapsed } from '$lib/stores/ui';
    import { goto } from '$app/navigation';
    import { convertFileSrc } from '@tauri-apps/api/core';
    import Icon from './Icon.svelte';

    const appMenu = [
        { label: 'Dashboard', icon: 'dashboard', href: '/dashboard' }
    ];

    const adminMenu = [
        { label: 'Overview', icon: 'shield', href: '/admin' },
        { label: 'Users', icon: 'users', href: '/admin/users' },
        { label: 'Settings', icon: 'settings', href: '/admin/settings' }
    ];

    let isDropdownOpen = false;

    $: isUrlAdmin = $page.url.pathname.startsWith('/admin');
    $: currentMenu = isUrlAdmin ? adminMenu : appMenu;

    function handleLogout() {
        logout();
        goto('/login');
    }

    function navigate(href: string) {
        goto(href);
        isDropdownOpen = false;
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

<aside class="sidebar" class:collapsed={$isSidebarCollapsed}>
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

    <!-- Navigation -->
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

    <!-- Footer / Profile -->
    <div class="sidebar-footer">
        {#if $isAdmin}
            <button 
                class="context-btn" 
                on:click={() => goto(isUrlAdmin ? '/dashboard' : '/admin')}
                title={isUrlAdmin ? "Switch to User Dashboard" : "Switch to Admin Panel"}
            >
                <Icon name={isUrlAdmin ? 'arrow-left' : 'shield'} size={16} />
                <span class="label">{isUrlAdmin ? 'User View' : 'Admin Panel'}</span>
            </button>
        {/if}

        <div class="profile-section">
            {#if isDropdownOpen}
                <div class="dropdown-menu" on:click|stopPropagation>
                    <button class="menu-item" on:click={() => navigate('/profile')}>
                        <Icon name="profile" size={16} />
                        Profile
                    </button>
                    <div class="divider"></div>
                    <button class="menu-item danger" on:click={handleLogout}>
                        <Icon name="logout" size={16} />
                        Logout
                    </button>
                </div>
            {/if}

            <button class="profile-btn" on:click={toggleDropdown}>
                <div class="avatar">{$user?.name?.charAt(0).toUpperCase() || "?"}</div>
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
        transition: width 0.3s ease;
    }

    .sidebar.collapsed {
        width: 72px;
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

    .app-logo { width: 24px; height: 24px; object-fit: contain; }
    
    .logo-placeholder {
        width: 24px; height: 24px;
        display: flex; align-items: center; justify-content: center;
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
        transition: opacity 0.2s, width 0.2s;
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

    .sidebar.collapsed .user-meta,
    .sidebar.collapsed .chevron {
        display: none;
    }

    .profile-btn:hover {
        background: var(--bg-hover);
    }

    .avatar {
        width: 28px; height: 28px;
        border-radius: 6px;
        background: var(--bg-active);
        color: var(--text-primary);
        display: flex; align-items: center; justify-content: center;
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

    .name { font-size: 0.85rem; font-weight: 600; color: var(--text-primary); }
    .role { font-size: 0.7rem; color: var(--text-secondary); text-transform: capitalize; }

    /* Dropdown */
    .dropdown-menu {
        position: absolute;
        bottom: 100%;
        left: 0; right: 0;
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
        display: flex; align-items: center; gap: 8px;
        padding: 8px 12px;
        border: none; background: transparent;
        color: var(--text-secondary);
        font-size: 0.9rem;
        border-radius: var(--radius-sm);
        cursor: pointer;
        text-align: left;
    }

    .menu-item:hover { background: var(--bg-hover); color: var(--text-primary); }
    .menu-item.danger:hover { color: var(--color-danger); background: rgba(239, 68, 68, 0.1); }
    .divider { height: 1px; background: var(--border-color); margin: 4px 0; }

    @keyframes slideUp {
        from { opacity: 0; transform: translateY(5px); }
        to { opacity: 1; transform: translateY(0); }
    }
</style>