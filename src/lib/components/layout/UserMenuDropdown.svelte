<script lang="ts">
  import { goto } from '$app/navigation';
  import { logout, user } from '$lib/stores/auth';
  import { t } from 'svelte-i18n';
  import Icon from '../ui/Icon.svelte';

  let {
    variant = 'sidebar',
    onNavigate,
  }: {
    variant?: 'sidebar' | 'topbar';
    onNavigate?: () => void;
  } = $props();

  const tenantPrefix = '';
  let isDropdownOpen = $state(false);
  let rootEl = $state<HTMLDivElement | null>(null);

  function closeDropdown() {
    isDropdownOpen = false;
  }

  function toggleDropdown(event?: Event) {
    event?.stopPropagation();
    isDropdownOpen = !isDropdownOpen;
  }

  async function navigateToProfile() {
    await goto(`${tenantPrefix}/profile`);
    closeDropdown();
    onNavigate?.();
  }

  function handleLogout() {
    logout();
    closeDropdown();
    onNavigate?.();
    goto('/');
  }

  function handleWindowClick(event: MouseEvent) {
    if (event.target instanceof Node && rootEl?.contains(event.target)) return;
    closeDropdown();
  }

  function handleEscape(event: KeyboardEvent) {
    if (event.key === 'Escape') closeDropdown();
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleEscape} />

<div
  bind:this={rootEl}
  class:topbar-variant={variant === 'topbar'}
  class:sidebar-variant={variant === 'sidebar'}
  class="user-menu"
>
  {#if isDropdownOpen}
    <div
      class="dropdown-menu"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
      role="menu"
      tabindex="-1"
    >
      <div class="dropdown-header" aria-hidden="true">
        <div class="dropdown-avatar">
          {$user?.name?.charAt(0).toUpperCase() || '?'}
        </div>
        <div class="dropdown-meta">
          <div class="dropdown-name">
            {$user?.name || $t('profile.fallback.user') || 'User'}
          </div>
          <div class="dropdown-sub">
            {$user?.email || ''}
            {#if $user?.email && $user?.role}
              <span class="dot">·</span>
            {/if}
            {$user?.role || ''}
          </div>
        </div>
      </div>

      <div class="divider"></div>

      <button class="menu-item" role="menuitem" onclick={navigateToProfile}>
        <Icon name="profile" size={16} />
        {$t('sidebar.profile')}
      </button>

      <button class="menu-item danger" role="menuitem" onclick={handleLogout}>
        <Icon name="logout" size={16} />
        {$t('sidebar.logout')}
      </button>
    </div>
  {/if}

  <button
    class="profile-btn"
    onclick={toggleDropdown}
    aria-haspopup="menu"
    aria-expanded={isDropdownOpen}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        toggleDropdown();
      } else if (event.key === 'Escape') {
        closeDropdown();
      }
    }}
  >
    <div class="avatar">
      {$user?.name?.charAt(0).toUpperCase() || '?'}
    </div>
    <div class="user-meta">
      <span class="name">{$user?.name || $t('profile.fallback.user') || 'User'}</span>
      <span class="role">{$user?.role || ''}</span>
    </div>
    <span class="chevron">
      <Icon name="chevron-up" size={14} />
    </span>
  </button>
</div>

<style>
  .user-menu {
    position: relative;
    min-width: 0;
  }

  .profile-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 46px;
    padding: 8px 10px;
    background: color-mix(in srgb, var(--bg-surface) 46%, transparent);
    border: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease;
  }

  .profile-btn:hover {
    background: color-mix(in srgb, var(--bg-hover) 78%, transparent);
    border-color: color-mix(in srgb, var(--border-color) 92%, transparent);
  }

  .profile-btn:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary), white 10%);
    outline-offset: 2px;
  }

  .topbar-variant .profile-btn {
    width: auto;
    min-width: 0;
    max-width: min(240px, 30vw);
    padding-inline: 10px 12px;
    background: color-mix(in srgb, var(--bg-tertiary) 88%, var(--bg-surface));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .avatar {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-primary) 16%, var(--bg-active));
    color: color-mix(in srgb, var(--color-primary) 72%, var(--text-primary));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8rem;
    font-weight: 900;
    flex-shrink: 0;
  }

  .user-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    overflow: hidden;
  }

  .name {
    font-size: 0.85rem;
    font-weight: 820;
    color: var(--text-primary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .role {
    font-size: 0.7rem;
    color: var(--text-secondary);
    text-transform: capitalize;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .topbar-variant .role {
    display: none;
  }

  .chevron {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .dropdown-menu {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 8px);
    background: color-mix(in srgb, var(--bg-surface), transparent 6%);
    border: 1px solid color-mix(in srgb, var(--border-color), white 8%);
    border-radius: var(--radius-md);
    padding: 6px;
    box-shadow: var(--shadow-md);
    display: flex;
    flex-direction: column;
    z-index: 100;
    animation: dropdownPop 0.14s ease-out;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
  }

  .dropdown-menu::after {
    content: '';
    position: absolute;
    left: 18px;
    top: -6px;
    width: 12px;
    height: 12px;
    background: color-mix(in srgb, var(--bg-surface), transparent 6%);
    border-left: 1px solid color-mix(in srgb, var(--border-color), white 8%);
    border-top: 1px solid color-mix(in srgb, var(--border-color), white 8%);
    transform: rotate(45deg);
  }

  .topbar-variant .dropdown-menu {
    left: auto;
    right: 0;
    width: clamp(220px, 26vw, 280px);
  }

  .topbar-variant .dropdown-menu::after {
    left: auto;
    right: 16px;
  }

  .dropdown-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 10px 8px;
    min-width: 0;
  }

  .dropdown-avatar {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    font-size: 0.9rem;
    flex: 0 0 auto;
    box-shadow: 0 0 0 1px var(--border-subtle) inset;
  }

  .dropdown-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  .dropdown-name {
    color: var(--text-primary);
    font-weight: 800;
    font-size: 0.9rem;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .dropdown-sub {
    color: var(--text-secondary);
    font-size: 0.78rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .dot {
    margin: 0 6px;
    opacity: 0.7;
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

  .menu-item:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 55%, transparent);
    outline-offset: 2px;
  }

  .menu-item.danger:hover {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
  }

  .divider {
    height: 1px;
    background: color-mix(in srgb, var(--border-color), transparent 35%);
    margin: 6px 6px;
  }

  @keyframes dropdownPop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }

    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
