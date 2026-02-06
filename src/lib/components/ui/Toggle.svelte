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
    background-color: rgba(255, 255, 255, 0.1);
    transition: 0.3s;
    border-radius: 24px;
    border: 1px solid var(--glass-border);
  }

  :global([data-theme='light']) .slider {
    background-color: rgba(0, 0, 0, 0.06);
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  input:checked + .slider {
    background-color: var(--color-primary);
    border-color: rgba(99, 102, 241, 0.4);
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  .disabled {
    opacity: 0.6;
    pointer-events: none;
  }
</style>

