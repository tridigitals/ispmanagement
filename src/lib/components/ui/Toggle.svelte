<script lang="ts">
  // Minimal toggle switch that matches the styling used in settings pages.
  let {
    checked = $bindable(false),
    disabled = false,
    ariaLabel = 'Toggle',
  }: {
    checked?: boolean;
    disabled?: boolean;
    ariaLabel?: string;
  } = $props();
</script>

<label class="toggle" class:disabled>
  <input type="checkbox" bind:checked {disabled} aria-label={ariaLabel} />
  <span class="slider"></span>
</label>

<style>
  .toggle {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--bg-tertiary);
    transition: 0.3s;
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
  }

  :global([data-theme='light']) .slider {
    background-color: var(--bg-tertiary);
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: var(--text-primary);
    transition: 0.3s;
    border-radius: 50%;
    box-shadow: var(--shadow-sm);
  }

  input:checked + .slider {
    background-color: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 40%, var(--border-color));
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  .disabled {
    opacity: 0.6;
    pointer-events: none;
  }
</style>
