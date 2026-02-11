<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';

  export type MiniSelectOption<T extends string | number> = {
    value: T;
    label: string;
    disabled?: boolean;
  };

  let {
    value = $bindable() as string | number,
    options,
    ariaLabel,
    label,
    minWidth = 128,
  }: {
    value?: string | number;
    options: MiniSelectOption<any>[];
    ariaLabel: string;
    label?: string;
    minWidth?: number;
  } = $props();

  let open = $state(false);
  let root: HTMLDivElement | null = null;

  const selectedLabel = $derived(
    options.find((o) => o.value === value)?.label ?? String(value ?? ''),
  );

  function toggle() {
    open = !open;
  }

  function close() {
    open = false;
  }

  function onDocClick(e: MouseEvent) {
    if (!open) return;
    const t = e.target as Node | null;
    if (root && t && root.contains(t)) return;
    close();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  if (typeof document !== 'undefined') {
    document.addEventListener('click', onDocClick);
    document.addEventListener('keydown', onKey);
  }

  onDestroy(() => {
    if (typeof document === 'undefined') return;
    document.removeEventListener('click', onDocClick);
    document.removeEventListener('keydown', onKey);
  });
</script>

<div class="mini" bind:this={root}>
  {#if label}
    <span class="mini-label">{label}</span>
  {/if}

  <button
    class="mini-btn"
    type="button"
    aria-label={ariaLabel}
    title={ariaLabel}
    onclick={toggle}
    style={`min-width:${minWidth}px;`}
  >
    <span class="mini-value">{selectedLabel}</span>
    <Icon name={open ? 'chevron-up' : 'chevron-down'} size={16} />
  </button>

  {#if open}
    <div class="menu" role="listbox" aria-label={ariaLabel}>
      {#each options as opt (opt.value)}
        <button
          type="button"
          class="item"
          class:active={opt.value === value}
          disabled={opt.disabled}
          onclick={() => {
            value = opt.value;
            close();
          }}
        >
          {opt.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .mini {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }

  .mini-label {
    font-size: 11px;
    letter-spacing: 0.12em;
    font-weight: 900;
    color: var(--text-muted);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .mini-btn {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
    color: var(--text-primary);
    outline: none;
    cursor: pointer;
  }

  .mini-btn:focus-visible {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent);
  }

  .mini-value {
    font-weight: 800;
    font-size: 13px;
    letter-spacing: 0.01em;
  }

  .menu {
    position: absolute;
    right: 10px;
    top: calc(100% + 10px);
    min-width: 180px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg);
    padding: 8px;
    z-index: 20;
  }

  .item {
    width: 100%;
    text-align: left;
    padding: 10px 10px;
    border-radius: 12px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 800;
    font-size: 13px;
  }

  .item:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .item.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .item:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>

