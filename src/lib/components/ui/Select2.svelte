<script lang="ts">
  import Icon from './Icon.svelte';
  import { slide } from 'svelte/transition';
  import { tick } from 'svelte';

  let {
    value = $bindable(''),
    options = [],
    placeholder = 'Select option',
    disabled = false,
    width = 'auto',
    placement = 'bottom',
    id = '',
    searchPlaceholder = 'Search...',
    noResultsText = 'No results',
    maxItems = 200,
    onchange,
  } = $props<{
    value?: any;
    options?: Array<{ label: string; value: any }> | Array<any>;
    placeholder?: string;
    disabled?: boolean;
    width?: string;
    placement?: 'top' | 'bottom';
    id?: string;
    searchPlaceholder?: string;
    noResultsText?: string;
    maxItems?: number;
    onchange?: (detail: any) => void;
  }>();

  let isOpen = $state(false);
  let selectingViaPointer = $state(false);
  let containerElement: HTMLElement;
  let searchElement: HTMLInputElement | null = $state(null);
  let query = $state('');
  let activeIndex = $state(0);

  let normalizedOptions = $derived(
    (options || []).map((opt: any) => {
      if (typeof opt === 'object' && opt !== null && 'value' in opt) {
        return opt as { label: string; value: any };
      }
      return { label: String(opt), value: opt };
    }),
  );

  let selectedLabel = $derived(
    normalizedOptions.find((opt: { label: string; value: any }) => opt.value === value)?.label ||
      placeholder,
  );

  let filteredOptions = $derived(
    (() => {
      const q = query.trim().toLowerCase();
      if (!q) return normalizedOptions;
      return normalizedOptions.filter((opt: { label: string }) =>
        opt.label.toLowerCase().includes(q),
      );
    })(),
  );

  let visibleOptions = $derived(filteredOptions.slice(0, Math.max(1, maxItems)));

  async function open() {
    if (disabled) return;
    isOpen = true;
    query = '';
    activeIndex = Math.max(
      0,
      visibleOptions.findIndex((o: { label: string; value: any }) => o.value === value),
    );
    await tick();
    searchElement?.focus();
  }

  function close() {
    isOpen = false;
    query = '';
  }

  function toggle() {
    if (isOpen) close();
    else void open();
  }

  function selectOption(optionVal: any) {
    value = optionVal;
    if (onchange) onchange({ detail: value });
    close();
  }

  function isEventInsideContainer(event: Event) {
    if (!containerElement) return false;
    const path = (event as any).composedPath?.() as EventTarget[] | undefined;
    if (Array.isArray(path) && path.length > 0) return path.includes(containerElement);
    const target = event.target as Node | null;
    return !!target && containerElement.contains(target);
  }

  function handleGlobalPointerDown(event: PointerEvent) {
    if (!isOpen || !containerElement) return;
    if (isEventInsideContainer(event)) return;
    close();
  }

  function handleFocusOut() {
    if (!isOpen || !containerElement) return;
    queueMicrotask(() => {
      if (selectingViaPointer) return;
      const active = document.activeElement;
      if (active && containerElement.contains(active)) return;
      close();
    });
  }

  function handleOptionPointerDown(optionVal: any, event: PointerEvent) {
    // Select on pointer-down so blur/focusout from map/canvas won't close before selection applies.
    event.preventDefault();
    event.stopPropagation();
    selectingViaPointer = true;
    selectOption(optionVal);
    queueMicrotask(() => {
      selectingViaPointer = false;
    });
  }

  function stepActive(delta: number) {
    const len = visibleOptions.length;
    if (len <= 0) return;
    activeIndex = (activeIndex + delta + len) % len;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (disabled) return;

    if (!isOpen) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        void open();
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        void open();
      }
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      stepActive(1);
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      stepActive(-1);
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const opt = visibleOptions[activeIndex];
      if (opt) selectOption(opt.value);
      return;
    }
  }
</script>

<svelte:window onpointerdown={handleGlobalPointerDown} />

<div class="select-container" style="width: {width}" bind:this={containerElement} onfocusout={handleFocusOut}>
  <button
    class="select-trigger {disabled ? 'disabled' : ''} {isOpen ? 'open' : ''}"
    onclick={toggle}
    onkeydown={handleKeydown}
    type="button"
    {disabled}
    {id}
    aria-haspopup="listbox"
    aria-expanded={isOpen}
  >
    <span class="selected-text {value === '' ? 'placeholder' : ''}">
      {selectedLabel}
    </span>
    <div class="icon-wrapper {isOpen ? 'rotate' : ''}">
      <Icon name="chevron-down" size={16} />
    </div>
  </button>

  {#if isOpen}
    <div class="dropdown-menu {placement}" transition:slide|local={{ duration: 200 }}>
      <div class="search-wrap">
        <input
          class="search"
          type="text"
          bind:this={searchElement}
          bind:value={query}
          placeholder={searchPlaceholder}
          onkeydown={handleKeydown}
          spellcheck="false"
          autocomplete="off"
          aria-label={searchPlaceholder}
        />
      </div>

      {#if visibleOptions.length === 0}
        <div class="empty">{noResultsText}</div>
      {:else}
        {#each visibleOptions as option, idx}
          <button
            class="dropdown-item {option.value === value ? 'selected' : ''} {idx === activeIndex
              ? 'active'
              : ''}"
            onpointerdown={(event) => handleOptionPointerDown(option.value, event)}
            onclick={() => selectOption(option.value)}
            onmousemove={() => (activeIndex = idx)}
            type="button"
            role="option"
            aria-selected={option.value === value}
          >
            {option.label}
            {#if option.value === value}
              <Icon name="check" size={14} class="check-icon" />
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .select-container {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    position: relative;
  }

  .select-trigger {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    padding: 0.6rem 2.5rem 0.6rem 1rem;
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
    position: relative;
  }

  .select-trigger:hover:not(.disabled) {
    border-color: var(--text-secondary);
  }

  .select-trigger.open {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
  }

  .selected-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .selected-text.placeholder {
    color: var(--text-secondary);
  }

  .icon-wrapper {
    position: absolute;
    right: 0.8rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    transition: transform 0.2s;
  }

  .icon-wrapper.rotate {
    transform: translateY(-50%) rotate(180deg);
  }

  .dropdown-menu {
    position: absolute;
    left: 0;
    right: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow:
      0 10px 15px -3px rgba(0, 0, 0, 0.1),
      0 4px 6px -2px rgba(0, 0, 0, 0.05);
    z-index: 60;
    max-height: 280px;
    overflow-y: auto;
    padding: 0.25rem;
  }

  .dropdown-menu.bottom {
    top: calc(100% + 0.5rem);
  }

  .dropdown-menu.top {
    bottom: calc(100% + 0.5rem);
  }

  .search-wrap {
    position: sticky;
    top: 0;
    background: var(--bg-surface);
    padding: 0.25rem;
    border-bottom: 1px solid var(--border-color);
    z-index: 1;
  }

  .search {
    width: 100%;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.55rem 0.75rem;
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .search:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .dropdown-item {
    width: 100%;
    text-align: left;
    padding: 0.6rem 1rem;
    background: transparent;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.9rem;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .dropdown-item:hover {
    background: var(--bg-hover);
  }

  .dropdown-item.active {
    background: rgba(99, 102, 241, 0.08);
  }

  .dropdown-item.selected {
    background: rgba(99, 102, 241, 0.12);
    color: var(--color-primary);
    font-weight: 500;
  }

  .empty {
    padding: 0.75rem 1rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: var(--bg-app);
  }

  .dropdown-menu::-webkit-scrollbar {
    width: 6px;
  }

  .dropdown-menu::-webkit-scrollbar-track {
    background: transparent;
  }

  .dropdown-menu::-webkit-scrollbar-thumb {
    background-color: var(--border-color);
    border-radius: 20px;
  }
</style>
