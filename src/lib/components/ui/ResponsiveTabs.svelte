<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { clickOutside } from '$lib/actions/clickOutside';
  import Icon from './Icon.svelte';

  type TabItem = {
    id: string;
    label: string;
    count?: string | number | null;
    disabled?: boolean;
    panelId?: string;
  };

  let {
    items = [],
    activeId = $bindable(''),
    isMobile = false,
    priorityCount = 4,
    ariaLabel = 'Tabs',
    moreLabel = 'More',
  }: {
    items?: TabItem[];
    activeId?: string;
    isMobile?: boolean;
    priorityCount?: number;
    ariaLabel?: string;
    moreLabel?: string;
  } = $props();

  const dispatch = createEventDispatcher<{ change: string }>();
  let menuOpen = $state(false);

  const tabs = $derived(items.filter((item) => !!item));
  const primaryTabs = $derived.by(() => {
    if (!isMobile || tabs.length <= priorityCount) return tabs;
    const activeIndex = tabs.findIndex((item) => item.id === activeId);
    if (activeIndex < 0 || activeIndex < priorityCount) return tabs.slice(0, priorityCount);
    return [...tabs.slice(0, Math.max(priorityCount - 1, 0)), tabs[activeIndex]];
  });
  const secondaryTabs = $derived.by(() =>
    isMobile ? tabs.filter((item) => !primaryTabs.some((primary) => primary.id === item.id)) : [],
  );
  const secondaryActive = $derived(
    secondaryTabs.some((item) => item.id === activeId),
  );

  function selectTab(id: string, disabled?: boolean) {
    if (disabled) return;
    activeId = id;
    menuOpen = false;
    dispatch('change', id);
  }
</script>

<div class="tabs-shell" role="tablist" aria-label={ariaLabel}>
  <div class="tabs-main">
    <div class="tabs-rail">
      {#each primaryTabs as item (item.id)}
        <button
          type="button"
          role="tab"
          id={`tab-${item.id}`}
          class={`tab-chip ${activeId === item.id ? 'active' : ''}`.trim()}
          aria-selected={activeId === item.id}
          aria-controls={item.panelId}
          disabled={item.disabled}
          onclick={() => selectTab(item.id, item.disabled)}
        >
          <span>{item.label}</span>
          {#if item.count !== undefined && item.count !== null}
            <strong>{item.count}</strong>
          {/if}
        </button>
      {/each}
    </div>

    {#if isMobile && secondaryTabs.length > 0}
      <div class="more-wrap" use:clickOutside={{ callback: () => (menuOpen = false) }}>
        <button
          type="button"
          class={`tab-chip more-trigger ${secondaryActive ? 'active' : ''} ${menuOpen ? 'is-open' : ''}`.trim()}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={() => (menuOpen = !menuOpen)}
        >
          <span>{moreLabel}</span>
          <Icon name="chevron-down" size={14} />
        </button>

        {#if menuOpen}
          <div class="more-menu" role="menu">
            {#each secondaryTabs as item (item.id)}
              <button
                type="button"
                role="menuitemradio"
                aria-checked={activeId === item.id}
                class={`menu-item ${activeId === item.id ? 'active' : ''}`.trim()}
                disabled={item.disabled}
                onclick={() => selectTab(item.id, item.disabled)}
              >
                <span>{item.label}</span>
                {#if item.count !== undefined && item.count !== null}
                  <strong>{item.count}</strong>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .tabs-shell {
    min-width: 0;
    position: relative;
  }

  .tabs-main {
    display: flex;
    align-items: flex-start;
    gap: 0.55rem;
    min-width: 0;
  }

  .tabs-rail {
    display: flex;
    gap: 0.55rem;
    overflow-x: auto;
    padding-bottom: 0.2rem;
    scrollbar-width: none;
    min-width: 0;
    flex: 1 1 auto;
  }

  .tabs-rail::-webkit-scrollbar {
    display: none;
  }

  .tab-chip,
  .menu-item {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.5rem 0.82rem;
    min-height: 40px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    white-space: nowrap;
    cursor: pointer;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 650;
    flex: 0 0 auto;
  }

  .tab-chip strong,
  .menu-item strong {
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .tab-chip.active,
  .menu-item.active {
    border-color: color-mix(in srgb, var(--color-primary) 52%, var(--border-color));
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .more-wrap {
    position: relative;
    flex: 0 0 auto;
  }

  .more-trigger.is-open {
    border-color: var(--color-primary);
  }

  .more-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 0.45rem);
    z-index: 30;
    min-width: 220px;
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
    justify-content: space-between;
    border-radius: 12px;
    white-space: normal;
  }

  @media (max-width: 900px) {
    .tabs-shell {
      margin-inline: -0.1rem;
    }

    .tabs-main {
      gap: 0.5rem;
    }
  }
</style>
