<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { clickOutside } from '$lib/actions/clickOutside';
  import Icon from './Icon.svelte';

  type ActionItem = {
    id: string;
    label: string;
    icon?: string;
    tone?: 'default' | 'primary' | 'warning' | 'danger';
    disabled?: boolean;
  };

  let {
    items = [],
    primaryIds = [],
    isMobile = false,
    moreLabel = 'More',
  }: {
    items?: ActionItem[];
    primaryIds?: string[];
    isMobile?: boolean;
    moreLabel?: string;
  } = $props();

  const dispatch = createEventDispatcher<{ select: string }>();
  let menuOpen = $state(false);

  const visibleItems = $derived(items.filter((item) => !!item));
  const primaryItems = $derived.by(() => {
    if (!isMobile) return visibleItems;
    const mapped = primaryIds
      .map((id) => visibleItems.find((item) => item.id === id))
      .filter(Boolean) as ActionItem[];
    return mapped;
  });
  const secondaryItems = $derived.by(() =>
    isMobile
      ? visibleItems.filter((item) => !primaryItems.some((primary) => primary.id === item.id))
      : [],
  );

  function toneClass(item: ActionItem) {
    switch (item.tone) {
      case 'primary':
        return 'is-primary';
      case 'warning':
        return 'is-warning';
      case 'danger':
        return 'is-danger';
      default:
        return '';
    }
  }

  function handleSelect(item: ActionItem) {
    if (item.disabled) return;
    menuOpen = false;
    dispatch('select', item.id);
  }
</script>

<div class="actions-shell" class:is-mobile={isMobile}>
  <div class="primary-actions">
    {#each primaryItems as item (item.id)}
      <button
        type="button"
        class={`action-btn ${toneClass(item)}`.trim()}
        disabled={item.disabled}
        onclick={() => handleSelect(item)}
      >
        {#if item.icon}
          <Icon name={item.icon} size={16} />
        {/if}
        <span>{item.label}</span>
      </button>
    {/each}

    {#if isMobile && secondaryItems.length > 0}
      <div class="more-wrap" use:clickOutside={{ callback: () => (menuOpen = false) }}>
        <button
          type="button"
          class={`action-btn more-trigger ${menuOpen ? 'is-open' : ''}`.trim()}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={() => (menuOpen = !menuOpen)}
        >
          <Icon name="chevron-down" size={16} />
          <span>{moreLabel}</span>
        </button>

        {#if menuOpen}
          <div class="more-menu" role="menu">
            {#each secondaryItems as item (item.id)}
              <button
                type="button"
                role="menuitem"
                class={`menu-item ${toneClass(item)}`.trim()}
                disabled={item.disabled}
                onclick={() => handleSelect(item)}
              >
                {#if item.icon}
                  <Icon name={item.icon} size={16} />
                {/if}
                <span>{item.label}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .actions-shell {
    min-width: 0;
  }

  .primary-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.55rem;
    justify-content: flex-end;
  }

  .action-btn,
  .menu-item {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    min-height: 42px;
    padding: 0.7rem 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    cursor: pointer;
    font: inherit;
    font-weight: 650;
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }

  .action-btn:hover,
  .menu-item:hover {
    background: var(--bg-hover);
  }

  .action-btn:disabled,
  .menu-item:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .is-primary {
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 58%, var(--border-color));
    color: white;
  }

  .is-warning {
    border-color: color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-warning);
  }

  .is-danger {
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    color: var(--color-danger);
  }

  .more-wrap {
    position: relative;
  }

  .more-trigger.is-open {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .more-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 0.45rem);
    z-index: 30;
    min-width: 210px;
    padding: 0.45rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg, 0 18px 48px rgba(0, 0, 0, 0.18));
    display: grid;
    gap: 0.3rem;
  }

  .menu-item {
    width: 100%;
    justify-content: flex-start;
    min-height: 40px;
    padding: 0.72rem 0.78rem;
  }

  @media (max-width: 900px) {
    .actions-shell.is-mobile,
    .actions-shell.is-mobile .primary-actions {
      width: 100%;
    }

    .actions-shell.is-mobile .primary-actions {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      justify-content: stretch;
    }

    .actions-shell.is-mobile .action-btn {
      width: 100%;
      min-width: 0;
    }

    .actions-shell.is-mobile .more-wrap {
      min-width: 0;
    }

    .actions-shell.is-mobile .more-menu {
      left: 0;
      right: 0;
      min-width: 0;
    }
  }
</style>
