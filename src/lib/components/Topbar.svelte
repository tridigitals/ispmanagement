<script lang="ts">
    import { page } from '$app/stores';
    import { user } from '$lib/stores/auth';
    import Icon from './Icon.svelte';

    // Helper to get page title based on path
    function getPageTitle(path: string) {
        if (path === '/dashboard') return 'Dashboard';
        if (path === '/profile') return 'My Profile';
        if (path === '/admin') return 'Admin Overview';
        if (path === '/admin/users') return 'User Management';
        if (path === '/admin/settings') return 'Global Settings';
        return 'SaaS App';
    }

    $: title = getPageTitle($page.url.pathname);
</script>

<header class="topbar">
    <div class="left-section">
        <h2 class="page-title">{title}</h2>
    </div>

    <div class="right-section">
        <!-- Optional: Search Bar -->
        <div class="search-bar">
            <Icon name="search" size={16} />
            <input type="text" placeholder="Search..." />
        </div>

        <!-- Actions -->
        <button class="icon-btn" title="Notifications">
            <Icon name="bell" size={18} />
            <span class="badge-dot"></span>
        </button>
        
        <button class="icon-btn" title="Help">
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

    .badge-dot {
        position: absolute;
        top: 6px;
        right: 6px;
        width: 6px;
        height: 6px;
        background: var(--color-danger);
        border-radius: 50%;
        border: 1px solid var(--bg-primary);
    }
</style>