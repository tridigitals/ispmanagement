<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    value = $bindable(''),
    label,
    id,
    placeholder,
    disabled = false,
  }: {
    value?: string;
    label?: string;
    id?: string;
    placeholder?: string;
    disabled?: boolean;
  } = $props();

  let inputEl: HTMLInputElement | null = $state(null);

  function openPicker() {
    if (!inputEl || disabled) return;
    const anyEl = inputEl as any;
    if (typeof anyEl.showPicker === 'function') {
      anyEl.showPicker();
      return;
    }
    inputEl.focus();
  }
</script>

<label class="label">
  {#if label}{label}{/if}
  <div class="wrap">
    <input
      bind:this={inputEl}
      class="input"
      type="datetime-local"
      bind:value
      {id}
      {placeholder}
      {disabled}
    />
    <button
      class="icon-btn"
      type="button"
      onclick={openPicker}
      disabled={disabled}
      aria-label="Open date time picker"
      title="Open date time picker"
    >
      <Icon name="calendar" size={16} />
    </button>
  </div>
</label>

<style>
  .label {
    display: grid;
    gap: 0.35rem;
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.9rem;
  }

  .wrap {
    position: relative;
    display: grid;
    align-items: center;
  }

  .input {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    padding: 0.65rem 2.6rem 0.65rem 0.85rem;
    font-size: 0.95rem;
  }

  .input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  /* Hide native indicator; we provide our own icon so it is always visible in dark/light. */
  .input::-webkit-calendar-picker-indicator {
    opacity: 0;
  }

  .icon-btn {
    position: absolute;
    right: 0.55rem;
    top: 50%;
    transform: translateY(-50%);
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.9);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  :global([data-theme='light']) .icon-btn {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
    color: rgba(0, 0, 0, 0.75);
  }

  .icon-btn:hover:not(:disabled) {
    border-color: rgba(99, 102, 241, 0.45);
  }

  .icon-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>

