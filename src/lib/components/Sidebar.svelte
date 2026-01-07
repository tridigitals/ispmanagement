<script lang="ts">
    import { page } from '$app/stores';
    import { user, isAdmin, logout } from '$lib/stores/auth';
    import { appSettings } from '$lib/stores/settings';
    import { appLogo } from '$lib/stores/logo';
    import { theme } from '$lib/stores/theme';
    import { goto } from '$app/navigation';
    import Icon from './Icon.svelte';

    // Navigation Structure
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
        const currentPath = $page.url.pathname;
        return currentPath === item.href;
    }
</script>

<svelte:window on:click={handleWindowClick} />

<aside class="sidebar" class:admin-mode={isUrlAdmin}>
    <div class="sidebar-header">
        <div class="logo-container">
            <div class="logo-icon">
                {#if $appLogo}
                    <img src={$appLogo} alt="Logo" class="custom-logo" />
                {:else}
                    <Icon name="app" size={24} strokeWidth={2.5} />
                {/if}
            </div>
            <div class="logo-text-wrapper">
                <h2 class="logo-text">{$appSettings.app_name || 'SaaS App'}</h2>
            </div>
        </div>
    </div>

    <!-- Main Navigation -->
    <nav class="sidebar-nav">
        {#each currentMenu as item}
            <button
                class="nav-item"
                class:active={isActive(item)}
                on:click={() => navigate(item.href)}
            >
                <span class="icon">
                    <Icon name={item.icon} size={18} />
                </span>
                <span class="label">{item.label}</span>
            </button>
        {/each}
    </nav>

    {#if $isAdmin}
        <div class="context-switcher">
            {#if isUrlAdmin}
                <button class="switcher-btn" on:click={() => goto('/dashboard')}>
                    <Icon name="arrow-left" size={16} />
                    <span>User Dashboard</span>
                </button>
            {:else}
                <button class="switcher-btn admin-link" on:click={() => goto('/admin')}>
                    <Icon name="shield" size={16} />
                    <span>Admin Panel</span>
                </button>
            {/if}
        </div>
    {/if}

    <div class="sidebar-footer">
        <div class="user-menu-container">
            {#if isDropdownOpen}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <div class="user-dropdown fade-in" on:click|stopPropagation>
                    <button class="dropdown-item" on:click={() => navigate('/profile')}>
                        <Icon name="profile" size={16} />
                        My Profile
                    </button>
                    <div class="divider"></div>
                    <button class="dropdown-item danger" on:click={handleLogout}>
                        <Icon name="logout" size={16} />
                        Logout
                    </button>
                </div>
            {/if}

            <button class="user-profile-btn" class:active={isDropdownOpen} on:click={toggleDropdown}>
                <div class="avatar">
                    {$user?.name?.charAt(0).toUpperCase() || "?"}
                </div>
                <div class="user-info">
                    <span class="name">{$user?.name}</span>
                    <span class="role">{$user?.role}</span>
                </div>
                <div class="chevron">
                    <Icon name={isDropdownOpen ? 'chevron-down' : 'chevron-up'} size={14} />
                </div>
            </button>
        </div>
    </div>
</aside>

<style>
    .sidebar {
        width: 260px;
        height: 100vh;
        background: var(--bg-secondary);
        border-right: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
        z-index: 50;
        position: relative;
    }

    .sidebar-header {
        padding: 1.5rem;
        border-bottom: 1px solid rgba(255,255,255,0.03);
    }

    .logo-container {
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .logo-icon {
        color: var(--color-primary);
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .custom-logo {
        width: 32px;
        height: 32px;
        object-fit: contain;
    }

    .logo-text-wrapper {
        display: flex;
        flex-direction: column;
    }

    .logo-text {
        font-size: 1rem;
        font-weight: 800;
        letter-spacing: 1px;
        color: var(--text-primary);
        text-transform: uppercase;
    }

    .sidebar-nav {
        padding: 1.5rem 1rem;
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.65rem 1rem;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        border-radius: var(--border-radius-sm);
        transition: all 0.2s ease;
        text-align: left;
    }

    .nav-item:hover {
        background: rgba(255, 255, 255, 0.03);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--bg-tertiary);
        color: var(--color-primary-light);
        border: 1px solid var(--border-color);
    }

    .icon {
        display: flex;
        align-items: center;
        justify-content: center;
        opacity: 0.7;
    }

    .nav-item.active .icon {
        opacity: 1;
        color: var(--color-primary);
    }

    .context-switcher {
        padding: 1rem;
        border-top: 1px solid rgba(255,255,255,0.03);
    }

    .switcher-btn {
        width: 100%;
        display: flex;
        align-items: center;
        gap: 0.6rem;
        padding: 0.6rem 1rem;
        background: rgba(255,255,255,0.03);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius-sm);
        color: var(--text-secondary);
        font-size: 0.8rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s ease;
    }

    .switcher-btn:hover {
        background: rgba(255,255,255,0.05);
        color: var(--text-primary);
    }

    .switcher-btn.admin-link {
        color: #fbbf24;
        border-color: rgba(245, 158, 11, 0.2);
    }

    .sidebar-footer {
        padding: 1rem;
        position: relative;
        z-index: 60;
    }

    .user-profile-btn {
        width: 100%;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.6rem;
        background: transparent;
        border: 1px solid transparent;
        border-radius: var(--border-radius-sm);
        cursor: pointer;
        text-align: left;
        color: var(--text-primary);
    }

    .user-profile-btn:hover, .user-profile-btn.active {
        background: rgba(255,255,255,0.03);
    }

    .avatar {
        width: 32px;
        height: 32px;
        border-radius: 8px;
        background: var(--bg-tertiary);
        color: var(--text-primary);
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        font-size: 0.8rem;
        border: 1px solid var(--border-color);
    }

    .user-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .user-info .name {
        font-size: 0.85rem;
        font-weight: 600;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .user-info .role {
        font-size: 0.7rem;
        color: var(--text-muted);
        text-transform: capitalize;
    }

    .chevron {
        color: var(--text-muted);
    }

    .user-dropdown {
        position: absolute;
        bottom: 100%;
        margin-bottom: 12px;
        left: 0;
        right: 0;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius-sm);
        padding: 0.4rem;
        box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);
        display: flex;
        flex-direction: column;
        gap: 0.1rem;
        z-index: 999;
    }

    .dropdown-item {
        display: flex;
        align-items: center;
        gap: 0.6rem;
        padding: 0.6rem 0.8rem;
        width: 100%;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.85rem;
        font-weight: 500;
        cursor: pointer;
        border-radius: 4px;
        text-align: left;
    }

    .dropdown-item:hover {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .dropdown-item.danger:hover {
        background: rgba(239, 68, 68, 0.05);
        color: var(--color-danger);
    }

    .divider {
        height: 1px;
        background: var(--border-color);
        margin: 0.2rem 0.4rem;
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(5px); }
        to { opacity: 1; transform: translateY(0); }
    }
    .fade-in { animation: fadeIn 0.2s ease-out; }
</style>